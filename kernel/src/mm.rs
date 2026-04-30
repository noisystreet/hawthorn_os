// SPDX-License-Identifier: MIT OR Apache-2.0

//! AArch64 4-level page table management and MMU enable.
//!
//! Identity-mapped kernel RAM and low MMIO under **TTBR0_EL1**.
//! Page attributes and MMU enable sequence follow the minimal `aarch64_kernel` reference
//! (`kernel/mm/identity.rs`, `bits.rs`): **AP** for EL1-only RW, **UXN** on RAM, **PXN** on
//! device, hierarchical table bits, then **`SCTLR_EL1.M`** via `mrs`/`orr`/`msr` (plus **C**/**I**
//! here because `_start` clears caches). **TCR** matches Linux-style **WBWA**, **IPS** from
//! `ID_AA64MMFR0_EL1`, **EPD1**, **T0SZ=16** (L0 root with 4 KiB granules).
//!
//! Uses 2 MiB block mappings at PMD level (level 2) for simplicity.
//! L3 (4 KiB page) mappings are used only for user task pages.

use core::arch::asm;

use crate::frame_alloc;

const PAGE_SIZE: usize = 4096;
const BLOCK_SIZE: usize = 2 * 1024 * 1024;

const ENTRIES_PER_TABLE: usize = 512;

pub const PTE_VALID: u64 = 1 << 0;
pub const PTE_TABLE: u64 = 1 << 1;
pub const PTE_BLOCK: u64 = 0 << 1;
pub const PTE_PAGE: u64 = 1 << 1;
pub const PTE_AF: u64 = 1 << 10;
#[allow(dead_code)]
pub const PTE_AP_RO: u64 = 0b11 << 6;
/// EL1 read/write, EL0 no access (same as `DESC_AP_RW_EL1` in aarch64_kernel `bits.rs`).
pub const PTE_AP_RW_EL1: u64 = 0b00 << 6;
/// EL1 + EL0 read/write (user pages).
#[allow(dead_code)]
pub const PTE_AP_RW_ALL: u64 = 0b01 << 6;
/// Inner Shareable (descriptor bits [9:8]); complements MAIR for Normal memory.
pub const PTE_SH_IS: u64 = 0b11 << 8;
/// Non-Shareable (descriptor bits [9:8]); use for Device memory.
pub const PTE_SH_NONE: u64 = 0b00 << 8;
/// Unprivileged execute-never (set on kernel RAM / MMIO like aarch64_kernel).
pub const PTE_UXN: u64 = 1 << 54;
pub const PTE_PXN: u64 = 1 << 53;

/// Table descriptor: subtree denies EL0 access; EL0 execute-never default (see aarch64_kernel).
pub const TABLE_APTABLE0: u64 = 1 << 61;
pub const TABLE_UXNTABLE: u64 = 1 << 60;

pub const ATTR_NORMAL: u64 = 0 << 2;
pub const ATTR_DEVICE: u64 = 1 << 2;

const RAM_START: usize = 0x4000_0000;
#[allow(dead_code)]
const RAM_END: usize = 0x4800_0000;
#[allow(dead_code)]
const GIC_START: usize = 0x0800_0000;
#[allow(dead_code)]
const GIC_END: usize = 0x0810_0000;
#[allow(dead_code)]
const UART_START: usize = 0x0900_0000;
#[allow(dead_code)]
const UART_END: usize = 0x0900_1000;

/// Bits of virtual address subsumed by **TTBR0_EL1** (must match our L0 page-table root).
///
/// With **4 KiB granules**, Arm defines the **starting lookup level** from `TCR_EL1.T0SZ`:
/// `T0SZ = 64 − VA_BITS`. For **`T0SZ = 25`** (`VA_BITS = 39`) the walk begins at
/// **level 1** (three levels), but this kernel builds a **four-level L0-rooted** tree.
/// **`VA_BITS_T0 = 48`** gives **`T0SZ = 16`**, so the walk starts at **level 0** and
/// matches [`map_block_2m`] / [`map_page`].
///
/// **TTBR1** is disabled via `TCR_EL1.EPD1` until we add a high-half map.
const VA_BITS_T0: u64 = 48;

/// `MAIR_EL1` compatible with [`ATTR_NORMAL`] (index 0) and [`ATTR_DEVICE`] (index 1).
const MAIR_EL1_BOOT: u64 = 0x0000_0000_00FF_04FF;

const SCTLR_EL1_WXN: u64 = 1 << 19;
const SCTLR_EL1_M: u64 = 1 << 0;
const SCTLR_EL1_CI: u64 = (1 << 2) | (1 << 12); // C | I

/// `TCR_EL1` in the spirit of Linux `__cpu_setup` for a TTBR0-only early map.
fn tcr_el1_boot() -> u64 {
    let t0sz = 64 - VA_BITS_T0;
    let epd1 = 1u64 << 23;
    // Linux `TCR_CACHE_FLAGS` for TTBR0: IRGN_WBWA | ORGN_WBWA.
    let cache = (1u64 << 8) | (1u64 << 10);
    let sh = 3u64 << 12;
    // TG0=4K (0), TG1=4K (2) — TG1 fields are ignored while EPD1=1.
    let tg = 2u64 << 30;
    let ips = unsafe { tcr_ips_from_parange() };
    t0sz | epd1 | cache | sh | tg | ips
}

/// Map `ID_AA64MMFR0_EL1.PARange` into `TCR_EL1.IPS` (Linux `tcr_compute_pa_size`, 3-bit field).
unsafe fn tcr_ips_from_parange() -> u64 {
    let mmfr0: u64;
    asm!("mrs {0}, id_aa64mmfr0_el1", out(reg) mmfr0);
    let parange = (mmfr0 & 0xf) as u8;
    // Cap at 48-bit PA (encoding 5); avoids needing FEAT_LPA / 52-bit paths here.
    let capped = parange.min(5);
    (u64::from(capped & 0x7)) << 32
}

static mut KERNEL_PAGE_TABLE: usize = 0;

/// Clean + invalidate D-cache for a physical range (MMU off: VA equals PA).
unsafe fn dcache_civac_range(start: usize, end: usize) {
    const LINE: usize = 64;
    let mut a = start & !(LINE - 1);
    while a < end {
        asm!("dc civac, {}", in(reg) a, options(nostack));
        a += LINE;
    }
    asm!("dsb ish", options(nostack, preserves_flags));
    asm!("isb", options(nostack, preserves_flags));
}

fn index_at(vaddr: usize, level: usize) -> usize {
    let shift = 12 + (3 - level) * 9;
    (vaddr >> shift) & 0x1FF
}

fn entry_to_paddr(entry: u64) -> usize {
    ((entry >> 12) & 0xFFFF_FFFF_FFFF) as usize * PAGE_SIZE
}

fn make_entry(paddr: usize, flags: u64) -> u64 {
    ((paddr as u64 >> 12) << 12) | PTE_AF | flags
}

fn make_table_desc(paddr: usize) -> u64 {
    // Table descriptors: bits [11:2] ignored by hardware for the next-level address;
    // omit AF so the low bits are only the required [1:0]=0b11.
    ((paddr as u64 >> 12) << 12) | PTE_VALID | PTE_TABLE | TABLE_UXNTABLE | TABLE_APTABLE0
}

fn make_user_table_desc(paddr: usize) -> u64 {
    // User table descriptors must not force APTable[0]=1, otherwise EL0 access to
    // the entire subtree is denied regardless of leaf PTE permissions.
    ((paddr as u64 >> 12) << 12) | PTE_VALID | PTE_TABLE
}

fn get_or_alloc_table(current_table: usize, idx: usize) -> Option<usize> {
    get_or_alloc_table_with(current_table, idx, make_table_desc)
}

fn get_or_alloc_table_with(
    current_table: usize,
    idx: usize,
    make_desc: fn(usize) -> u64,
) -> Option<usize> {
    let entry_ptr = (current_table + idx * 8) as *mut u64;
    let entry = unsafe { *entry_ptr };

    if entry & PTE_VALID != 0 {
        if entry & PTE_TABLE != 0 {
            Some(entry_to_paddr(entry))
        } else {
            None
        }
    } else {
        let new_frame = frame_alloc::alloc_zeroed_frame()?;
        let new_entry = make_desc(new_frame);
        unsafe {
            *entry_ptr = new_entry;
        }
        Some(new_frame)
    }
}

fn map_block_2m(table: usize, vaddr: usize, paddr: usize, attr: u64) -> bool {
    let idx0 = index_at(vaddr, 0);
    let l1 = match get_or_alloc_table(table, idx0) {
        Some(t) => t,
        None => return false,
    };

    let idx1 = index_at(vaddr, 1);
    let l2 = match get_or_alloc_table(l1, idx1) {
        Some(t) => t,
        None => return false,
    };

    let idx2 = index_at(vaddr, 2);
    let entry_ptr = (l2 + idx2 * 8) as *mut u64;
    let entry = unsafe { *entry_ptr };

    if entry & PTE_VALID != 0 {
        return true;
    }

    // For 2 MiB block: output address bits [47:21], bits [20:12] must be 0
    // Block entries: omit NS unless platform is verified non-secure; QEMU virt EL1
    // has seen fewer faults with NS=0 in some firmware combinations.
    let block_entry = ((paddr as u64 >> 21) << 21) | PTE_VALID | PTE_BLOCK | PTE_AF | attr;
    unsafe {
        *entry_ptr = block_entry;
    }

    true
}

fn map_range_2m(table: usize, vaddr_start: usize, paddr_start: usize, size: usize, attr: u64) {
    let mut vaddr = vaddr_start & !(BLOCK_SIZE - 1);
    let paddr_aligned = paddr_start & !(BLOCK_SIZE - 1);
    let mut paddr = paddr_aligned;
    let end = vaddr_start + size;

    while vaddr < end {
        if !map_block_2m(table, vaddr, paddr, attr) {
            crate::println!("[mm] FAILED to map block {:#x} -> {:#x}", vaddr, paddr);
            loop {
                core::hint::spin_loop();
            }
        }
        vaddr += BLOCK_SIZE;
        paddr += BLOCK_SIZE;
    }
}

pub fn map_page(table: usize, vaddr: usize, paddr: usize, attr: u64) -> bool {
    let idx0 = index_at(vaddr, 0);
    let l1 = match get_or_alloc_table(table, idx0) {
        Some(t) => t,
        None => return false,
    };

    let idx1 = index_at(vaddr, 1);
    let l2 = match get_or_alloc_table(l1, idx1) {
        Some(t) => t,
        None => return false,
    };

    let idx2 = index_at(vaddr, 2);
    let l3 = match get_or_alloc_table(l2, idx2) {
        Some(t) => t,
        None => return false,
    };

    let idx3 = index_at(vaddr, 3);
    let entry_ptr = (l3 + idx3 * 8) as *mut u64;
    let pte = make_entry(paddr, PTE_VALID | PTE_PAGE | attr);
    unsafe {
        *entry_ptr = pte;
    }

    true
}

pub fn create_empty_page_table() -> Option<usize> {
    frame_alloc::alloc_zeroed_frame()
}

pub fn clone_kernel_mappings(dst_table: usize) -> bool {
    let src = unsafe { KERNEL_PAGE_TABLE };
    if src == 0 {
        return false;
    }

    let src_ptr = src as *const u64;
    let dst_ptr = dst_table as *mut u64;

    for i in 0..ENTRIES_PER_TABLE {
        let entry = unsafe { *src_ptr.add(i) };
        if entry & PTE_VALID != 0 && entry & PTE_TABLE != 0 {
            // User page-table roots must not inherit APTable[0]=1 / UXNTable=1 from
            // kernel tables, otherwise EL0 data access or instruction fetch is denied
            // for the entire subtree regardless of leaf PTE permissions.
            let user_entry = entry & !(TABLE_APTABLE0 | TABLE_UXNTABLE);
            unsafe {
                *dst_ptr.add(i) = user_entry;
            }
        }
    }

    true
}

/// Create a new user page table with kernel identity mappings cloned.
///
/// The user page table shares kernel L1 tables (via PGD[0] pointer) but can have
/// independent L2/L3 mappings for user code/stack.
pub fn create_user_page_table() -> Option<usize> {
    let table = frame_alloc::alloc_zeroed_frame()?;

    // Clone kernel mappings (PGD[0] → L1 tables)
    // This gives user tasks access to kernel RAM and device mappings
    // In a fully isolated system, we would use TTBR0/TTBR1 split instead
    if !clone_kernel_mappings(table) {
        return None;
    }

    Some(table)
}

/// Map a page in the specified page table (can be user or kernel).
///
/// # Safety
/// Caller must ensure:
/// - `table` is a valid page table root
/// - `vaddr` and `paddr` are page-aligned (4K)
/// - The mapping does not conflict with existing entries
pub unsafe fn map_user_page(table: usize, vaddr: usize, paddr: usize, attr: u64) -> bool {
    let idx0 = index_at(vaddr, 0);
    let l0_entry_ptr = (table + idx0 * 8) as *mut u64;
    let mut l0_entry = *l0_entry_ptr;
    let l1 = if l0_entry & PTE_VALID != 0 && (l0_entry & PTE_TABLE) == PTE_TABLE {
        l0_entry &= !(TABLE_APTABLE0 | TABLE_UXNTABLE);
        *l0_entry_ptr = l0_entry;
        entry_to_paddr(l0_entry)
    } else if l0_entry == 0 {
        let new_l1 = match frame_alloc::alloc_zeroed_frame() {
            Some(f) => f,
            None => return false,
        };
        *l0_entry_ptr = make_user_table_desc(new_l1);
        new_l1
    } else {
        return false;
    };

    let idx1 = index_at(vaddr, 1);
    let l1_entry_ptr = (l1 + idx1 * 8) as *mut u64;
    let mut l1_entry = *l1_entry_ptr;
    let l2 = if l1_entry & PTE_VALID != 0 && (l1_entry & PTE_TABLE) == PTE_TABLE {
        l1_entry &= !(TABLE_APTABLE0 | TABLE_UXNTABLE);
        *l1_entry_ptr = l1_entry;
        entry_to_paddr(l1_entry)
    } else if l1_entry == 0 {
        let new_l2 = match frame_alloc::alloc_zeroed_frame() {
            Some(f) => f,
            None => return false,
        };
        *l1_entry_ptr = make_user_table_desc(new_l2);
        new_l2
    } else {
        return false;
    };

    let idx2 = index_at(vaddr, 2);
    let l2_entry_ptr = (l2 + idx2 * 8) as *mut u64;
    let l2_entry = *l2_entry_ptr;
    let l3 = if l2_entry & PTE_VALID != 0 && (l2_entry & PTE_TABLE) == PTE_TABLE {
        let l2_table_entry = l2_entry & !(TABLE_APTABLE0 | TABLE_UXNTABLE);
        *l2_entry_ptr = l2_table_entry;
        entry_to_paddr(l2_table_entry)
    } else if l2_entry & PTE_VALID != 0 {
        // Split an existing 2 MiB block entry into a user-table L3 page table so a
        // user mapping can override a specific 4 KiB page.
        let new_l3 = match frame_alloc::alloc_zeroed_frame() {
            Some(f) => f,
            None => return false,
        };
        let block_base = ((l2_entry >> 21) << 21) as usize;
        // Preserve relevant block attributes for the 511 untouched pages.
        let common_attr = l2_entry
            & (0xFFFu64 // AttrIdx/AP/SH/AF/nG and low control bits
                | (1 << 51) // DBM
                | (1 << 52) // Contiguous
                | (1 << 53) // PXN
                | (1 << 54)); // UXN
        for i in 0..ENTRIES_PER_TABLE {
            let pa = block_base + i * PAGE_SIZE;
            let page_desc = ((pa as u64 >> 12) << 12) | common_attr | PTE_VALID | PTE_PAGE;
            *((new_l3 + i * 8) as *mut u64) = page_desc;
        }
        *l2_entry_ptr = make_user_table_desc(new_l3);
        new_l3
    } else {
        match get_or_alloc_table_with(l2, idx2, make_user_table_desc) {
            Some(t) => t,
            None => return false,
        }
    };

    let idx3 = index_at(vaddr, 3);
    let entry_ptr = (l3 + idx3 * 8) as *mut u64;
    let pte = make_entry(paddr, PTE_VALID | PTE_PAGE | attr);
    *entry_ptr = pte;
    true
}

pub fn init_from_boot() {
    unsafe {
        let mut ttbr0: u64;
        asm!("mrs {}, ttbr0_el1", out(reg) ttbr0);
        KERNEL_PAGE_TABLE = ttbr0 as usize;
    }
    crate::println!("[mm] using boot page table at {:#x}", unsafe {
        KERNEL_PAGE_TABLE
    });
}

pub fn init() {
    let table = frame_alloc::alloc_zeroed_frame().expect("[mm] failed to alloc root page table");
    unsafe {
        KERNEL_PAGE_TABLE = table;
    }

    let ram_size = 128 * 1024 * 1024; // 128 MiB
    map_range_2m(
        table,
        RAM_START,
        RAM_START,
        ram_size,
        ATTR_NORMAL | PTE_AP_RW_EL1 | PTE_SH_IS | PTE_UXN,
    );

    // Device region at 0x00000000-0x08000000 (2 MiB blocks)
    // Exclude 0x08000000-0x08200000 (GIC region, needs 4K pages)
    let dev_size = 0x0800_0000; // 128 MiB
    map_range_2m(
        table,
        0x0000_0000,
        0x0000_0000,
        dev_size,
        ATTR_DEVICE | PTE_AP_RW_EL1 | PTE_SH_NONE | PTE_UXN | PTE_PXN,
    );

    // GIC region: 0x08000000-0x08200000 (2 MiB, use 4K pages for precise mapping)
    // GICD: 0x08000000-0x08010000 (64 KiB)
    // GICR RD base: 0x080A0000-0x080AFFFF (64 KiB)
    // GICR SGI base: 0x080B0000-0x080BFFFF (64 KiB)
    // Map the entire 2 MiB region with 4K pages to ensure proper alignment
    crate::println!("[mm] mapping GIC region 0x08000000-0x08200000 with 4K pages...");
    for i in 0..512 {
        let vaddr = 0x0800_0000 + i * 4096;
        let paddr = 0x0800_0000 + i * 4096;
        if !map_page(
            table,
            vaddr,
            paddr,
            ATTR_DEVICE | PTE_AP_RW_EL1 | PTE_SH_NONE | PTE_UXN | PTE_PXN,
        ) {
            crate::println!("[mm] ERROR: failed to map page at {:#x}", vaddr);
            break;
        }
    }
    crate::println!("[mm] GIC region mapped");

    // Device region at 0x08200000-0x40000000 (2 MiB blocks)
    // PL011 is at 0x09000000
    map_range_2m(
        table,
        0x0820_0000,
        0x0820_0000,
        0x4000_0000 - 0x0820_0000,
        ATTR_DEVICE | PTE_AP_RW_EL1 | PTE_SH_NONE | PTE_UXN | PTE_PXN,
    );

    // Ordering + PoC: translation tables must be visible to the walker (DSB) and
    // coherent if any cache was enabled during construction (dc civac).
    unsafe {
        dcache_civac_range(table, table + 0x80_0000);
        asm!("dsb ishst", options(nostack, preserves_flags));
        asm!("isb", options(nostack, preserves_flags));
    }

    crate::println!("[mm] TTBR0 root {:#x}, 2MiB block mappings", table);
}

/// Dump page table structure for debugging (MMU must be OFF)
pub fn dump_tables() {
    let table = unsafe { KERNEL_PAGE_TABLE };
    if table == 0 {
        crate::println!("[mm/dump] ERROR: no page table");
        return;
    }

    crate::println!("[mm/dump] Page table base: {:#x}", table);

    // Dump PGD entries 0-3
    for i in 0..4 {
        let entry = unsafe { *((table + i * 8) as *const u64) };
        if entry != 0 {
            let typ = if entry & PTE_TABLE == PTE_TABLE {
                "Table"
            } else if entry & PTE_VALID != 0 && (entry & PTE_TABLE) == PTE_BLOCK {
                "Block"
            } else {
                "Unknown"
            };
            crate::println!(
                "[mm/dump] PGD[{}] = {:#x} ({}, paddr={:#x})",
                i,
                entry,
                typ,
                entry_to_paddr(entry)
            );
        }
    }

    // If PGD[0] is a table, dump its L1 entries 0-3
    let pgd0 = unsafe { *((table) as *const u64) };
    if pgd0 & PTE_TABLE != 0 {
        let l1 = entry_to_paddr(pgd0);
        crate::println!("[mm/dump] L1 table at {:#x}:", l1);
        for i in 0..4 {
            let entry = unsafe { *((l1 + i * 8) as *const u64) };
            if entry != 0 {
                let typ = if entry & PTE_TABLE == PTE_TABLE {
                    "Table"
                } else if entry & PTE_VALID != 0 && (entry & PTE_TABLE) == PTE_BLOCK {
                    "Block"
                } else {
                    "Unknown"
                };
                let paddr = entry_to_paddr(entry);
                crate::println!(
                    "[mm/dump]   L1[{}] = {:#x} ({}, paddr={:#x})",
                    i,
                    entry,
                    typ,
                    paddr
                );
            }
        }

        // Dump L2[0] for devices (via L1[0]) - includes GIC
        let l1_0 = unsafe { *(l1 as *const u64) };
        if l1_0 & PTE_TABLE != 0 {
            let l2 = entry_to_paddr(l1_0);
            crate::println!("[mm/dump] L2 table (devices) at {:#x}:", l2);
            // Show L2[64] which covers GIC region (0x0800_0000)
            for i in [0, 64, 65, 72, 73] {
                let entry = unsafe { *((l2 + i * 8) as *const u64) };
                if entry != 0 {
                    let typ = if entry & PTE_TABLE == PTE_TABLE {
                        "Table"
                    } else if entry & PTE_VALID != 0 && (entry & PTE_TABLE) == PTE_BLOCK {
                        "Block"
                    } else {
                        "Unknown"
                    };
                    let paddr: usize = if entry & PTE_TABLE != 0 {
                        entry_to_paddr(entry)
                    } else {
                        ((entry >> 21) << 21) as usize // For 2MiB block
                    };
                    crate::println!(
                        "[mm/dump]   L2[{}] = {:#x} ({}, paddr={:#x})",
                        i,
                        entry,
                        typ,
                        paddr
                    );
                }
            }
        }

        // Dump L2[0] for RAM (via L1[1])
        let l1_1 = unsafe { *((l1 + 8) as *const u64) };
        if l1_1 & PTE_TABLE != 0 {
            let l2 = entry_to_paddr(l1_1);
            crate::println!("[mm/dump] L2 table (RAM) at {:#x}:", l2);
            for i in 0..4 {
                let entry = unsafe { *((l2 + i * 8) as *const u64) };
                if entry != 0 {
                    let typ = if entry & PTE_TABLE == PTE_TABLE {
                        "Table"
                    } else if entry & PTE_VALID != 0 && (entry & PTE_TABLE) == PTE_BLOCK {
                        "Block"
                    } else {
                        "Unknown"
                    };
                    let paddr = (entry >> 21) << 21; // For 2MiB block
                    crate::println!(
                        "[mm/dump]   L2[{}] = {:#x} ({}, paddr={:#x})",
                        i,
                        entry,
                        typ,
                        paddr
                    );
                }
            }
        }
    }

    // Verify specific addresses
    let test_addrs = [0x4000_0000usize, 0x4000_1000, 0x0900_0000, 0x080a_0000];
    for addr in test_addrs {
        let idx0 = index_at(addr, 0);
        let idx1 = index_at(addr, 1);
        let idx2 = index_at(addr, 2);
        let idx3 = index_at(addr, 3);
        crate::println!(
            "[mm/dump] Address {:#x}: PGD[{}] L1[{}] L2[{}] L3[{}]",
            addr,
            idx0,
            idx1,
            idx2,
            idx3
        );

        // Actually walk the page table
        let pgd_entry = unsafe { *((table + idx0 * 8) as *const u64) };
        if pgd_entry & PTE_VALID != 0 && pgd_entry & PTE_TABLE != 0 {
            let l1 = entry_to_paddr(pgd_entry);
            let l1_entry = unsafe { *((l1 + idx1 * 8) as *const u64) };
            if l1_entry & PTE_VALID != 0 && l1_entry & PTE_TABLE != 0 {
                let l2 = entry_to_paddr(l1_entry);
                let l2_entry = unsafe { *((l2 + idx2 * 8) as *const u64) };
                if l2_entry & PTE_VALID != 0 && l2_entry & PTE_TABLE != 0 {
                    let l3 = entry_to_paddr(l2_entry);
                    let l3_entry = unsafe { *((l3 + idx3 * 8) as *const u64) };
                    crate::println!(
                        "[mm/dump]   L3[{}] = {:#x} (valid={}, page={})",
                        idx3,
                        l3_entry,
                        l3_entry & PTE_VALID != 0,
                        l3_entry & PTE_PAGE != 0
                    );
                } else {
                    crate::println!("[mm/dump]   L2[{}] not a table: {:#x}", idx2, l2_entry);
                }
            } else {
                crate::println!("[mm/dump]   L1[{}] not a table: {:#x}", idx1, l1_entry);
            }
        } else {
            crate::println!("[mm/dump]   PGD[{}] not a table: {:#x}", idx0, pgd_entry);
        }
    }
}

/// Step 4: Set MAIR/TCR/TTBR0 (and clear TTBR1), read back to verify, do NOT enable MMU
pub fn enable_mmu_step4() {
    let table = unsafe { KERNEL_PAGE_TABLE };
    if table == 0 {
        crate::println!("[mm/step4] ERROR: no page table");
        return;
    }

    let mair_val: u64 = MAIR_EL1_BOOT;
    let tcr_val: u64 = tcr_el1_boot();

    unsafe {
        // Linux `__cpu_setup`: invalidate TLB before programming translation registers.
        asm!("tlbi vmalle1is", options(nostack, preserves_flags));
        asm!("dsb ish", options(nostack, preserves_flags));
        asm!("isb", options(nostack, preserves_flags));

        // Write MAIR
        asm!("msr mair_el1, {}", in(reg) mair_val);
        asm!("isb");
        // Read back and verify
        let mut mair_read: u64;
        asm!("mrs {}, mair_el1", out(reg) mair_read);
        crate::println!(
            "[mm/step4] MAIR write={:#x} read={:#x} {}",
            mair_val,
            mair_read,
            if mair_read == mair_val {
                "OK"
            } else {
                "MISMATCH!"
            }
        );

        // Write TCR
        asm!("msr tcr_el1, {}", in(reg) tcr_val);
        asm!("isb");
        // Read back and verify
        let mut tcr_read: u64;
        asm!("mrs {}, tcr_el1", out(reg) tcr_read);
        crate::println!(
            "[mm/step4] TCR write={:#x} read={:#x} {}",
            tcr_val,
            tcr_read,
            if tcr_read == tcr_val {
                "OK"
            } else {
                "MISMATCH!"
            }
        );

        asm!("msr ttbr0_el1, {}", in(reg) table as u64);
        asm!("isb", options(nostack, preserves_flags));
        asm!("msr ttbr1_el1, xzr", options(nostack, preserves_flags));
        asm!("isb", options(nostack, preserves_flags));

        let mut ttbr0_read: u64;
        asm!("mrs {}, ttbr0_el1", out(reg) ttbr0_read);
        let mut ttbr1_read: u64;
        asm!("mrs {}, ttbr1_el1", out(reg) ttbr1_read);
        crate::println!(
            "[mm/step4] TTBR0 write={:#x} read={:#x} {}",
            table,
            ttbr0_read,
            if ttbr0_read == table as u64 {
                "OK"
            } else {
                "MISMATCH!"
            }
        );
        crate::println!(
            "[mm/step4] TTBR1 cleared read={:#x} {}",
            ttbr1_read,
            if ttbr1_read == 0 { "OK" } else { "MISMATCH!" }
        );

        // Ensure PTE stores are visible before invalidating TLB entries.
        asm!("dsb ishst", options(nostack, preserves_flags));
        asm!("isb", options(nostack, preserves_flags));
        asm!("tlbi vmalle1is", options(nostack, preserves_flags));
        asm!("isb", options(nostack, preserves_flags));
        crate::println!("[mm/step4] TLB invalidated, MMU NOT enabled yet");

        // Verify L2 table entry for GICR_WAKER (0x080A_0014) - now using 2MiB block
        let gicr_waker_vaddr = 0x080A_0014usize;
        let idx0 = index_at(gicr_waker_vaddr, 0);
        let idx1 = index_at(gicr_waker_vaddr, 1);
        let idx2 = index_at(gicr_waker_vaddr, 2);

        let pgd_entry = *((table + idx0 * 8) as *const u64);
        let l1 = entry_to_paddr(pgd_entry);
        let l1_entry = *((l1 + idx1 * 8) as *const u64);
        let l2 = entry_to_paddr(l1_entry);
        let l2_entry = *((l2 + idx2 * 8) as *const u64);

        crate::println!("[mm/step4] GICR_WAKER (0x080A_0014) page table walk:");
        crate::println!(
            "[mm/step4]   PGD[{}] = {:#x} (L1={:#x})",
            idx0,
            pgd_entry,
            l1
        );
        crate::println!("[mm/step4]   L1[{}] = {:#x} (L2={:#x})", idx1, l1_entry, l2);
        crate::println!("[mm/step4]   L2[{}] = {:#x} (block entry)", idx2, l2_entry);
    }
}

/// Step 5: Enable MMU (**M** then **C**/**I**), matching `aarch64_kernel` `identity::enable_mmu`
/// (single `orr` for **M**) plus cache bring-up after `_start` cleared **C**/**I**.
pub fn enable_mmu_step5() {
    unsafe {
        // `aarch64_kernel` `head.S`: mask all exceptions before MMU; avoids IRQ to
        // `irq::dispatch` before `gic::init`.
        asm!("msr daifset, #0xf", options(nostack, preserves_flags));
        let mut sctlr_before: u64;
        asm!("mrs {}, sctlr_el1", out(reg) sctlr_before);
        let with_m = (sctlr_before | SCTLR_EL1_M) & !SCTLR_EL1_WXN;
        asm!(
            "msr sctlr_el1, {s}",
            "isb",
            "ic iallu",
            "dsb nsh",
            "isb",
            s = in(reg) with_m,
            options(nostack),
        );
        let with_c_i = with_m | SCTLR_EL1_CI;
        asm!(
            "msr sctlr_el1, {s}",
            "isb",
            "ic iallu",
            "dsb nsh",
            "isb",
            s = in(reg) with_c_i,
            options(nostack),
        );

        let mut sctlr_after: u64;
        asm!("mrs {}, sctlr_el1", out(reg) sctlr_after);
        crate::println!(
            "[mm/step5] SCTLR before={:#x} after={:#x} (M={} C={} I={})",
            sctlr_before,
            sctlr_after,
            sctlr_after & 1,
            (sctlr_after >> 2) & 1,
            (sctlr_after >> 12) & 1,
        );

        if sctlr_after & 1 == 1 {
            crate::println!("[mm/step5] MMU ENABLED!");
        } else {
            crate::println!("[mm/step5] ERROR: MMU not enabled!");
        }
    }
}

pub fn enable_mmu() {
    let table = unsafe { KERNEL_PAGE_TABLE };
    if table == 0 {
        crate::println!("[mm] ERROR: no page table, cannot enable MMU");
        loop {
            core::hint::spin_loop();
        }
    }

    unsafe {
        let mair: u64 = MAIR_EL1_BOOT;
        let tcr: u64 = tcr_el1_boot();
        let mut sctlr: u64;
        asm!("mrs {}, sctlr_el1", out(reg) sctlr);
        let with_m = (sctlr | SCTLR_EL1_M) & !SCTLR_EL1_WXN;
        let with_c_i = with_m | SCTLR_EL1_CI;

        asm!(
            "tlbi vmalle1is",
            "dsb ish",
            "isb",
            "msr mair_el1, {mair}",
            "isb",
            "msr tcr_el1, {tcr}",
            "isb",
            "msr ttbr0_el1, {tbl0}",
            "isb",
            "msr ttbr1_el1, xzr",
            "isb",
            "dsb ishst",
            "isb",
            "tlbi vmalle1is",
            "isb",
            "msr sctlr_el1, {s1}",
            "isb",
            "ic iallu",
            "dsb nsh",
            "isb",
            "msr sctlr_el1, {s2}",
            "isb",
            "ic iallu",
            "dsb nsh",
            "isb",
            mair = in(reg) mair,
            tcr = in(reg) tcr,
            tbl0 = in(reg) table as u64,
            s1 = in(reg) with_m,
            s2 = in(reg) with_c_i,
            options(nostack),
        );
    }

    crate::println!("[mm] MMU enabled (identity map TTBR0={:#x})", table);
}

pub fn kernel_page_table() -> usize {
    unsafe { KERNEL_PAGE_TABLE }
}

pub fn set_ttbr0(table: usize) {
    unsafe {
        asm!("msr ttbr0_el1, {}", in(reg) table as u64);
        asm!("isb");
        asm!("tlbi vmalle1is");
        asm!("isb");
    }
}
