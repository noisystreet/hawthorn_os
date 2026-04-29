// SPDX-License-Identifier: MIT OR Apache-2.0

//! QEMU `virt` AArch64 bring-up: BSS, PL011 @ `0x9000_0000`, and `kernel_main`.

use core::arch::asm;
use core::fmt;

/// PL011 base on QEMU `virt` AArch64 (DT `pl011@9000000`).
pub const PL011_BASE: usize = 0x900_0000;
const PL011_DR: usize = PL011_BASE;
const PL011_IBRD: usize = PL011_BASE + 0x24;
const PL011_FBRD: usize = PL011_BASE + 0x28;
const PL011_LCR_H: usize = PL011_BASE + 0x2c;
const PL011_CR: usize = PL011_BASE + 0x30;

extern "C" {
    static __bss_start: u8;
    static __bss_end: u8;
    static __user_program_start: u8;
    static __user_program_end: u8;
}

/// Simple user program that prints "hello from EL0!" via syscall and exits.
/// This is embedded in the .user_program section and mapped into user address space.
#[link_section = ".user_program"]
#[no_mangle]
#[used]
static USER_PROGRAM: [u8; 52] = [
    // mov x8, #0          // SYS_write syscall number
    0x80, 0x00, 0x80, 0xd2, // mov x0, #1          // fd = stdout
    0x20, 0x00, 0x80, 0xd2,
    // adr x1, msg         // pointer to embedded message (PC-relative)
    0xe1, 0x00, 0x00, 0x10, // mov x2, #16         // length
    0x02, 0x02, 0x80, 0xd2, // svc #0              // syscall
    0x01, 0x00, 0x00, 0xd4, // mov x8, #1          // SYS_exit
    0x88, 0x00, 0x80, 0xd2, // mov x0, #0          // exit code 0
    0x00, 0x00, 0x80, 0xd2, // svc #0              // syscall
    0x01, 0x00, 0x00, 0xd4,
    // b .                 // loop forever (should not reach here)
    0x00, 0x00, 0x00, 0x14, // msg: .ascii "hello from EL0!\n"
    b'h', b'e', b'l', b'l', b'o', b' ', b'f', b'r', b'o', b'm', b' ', b'E', b'L', b'0', b'!',
    b'\n',
];

/// Write a 32-bit value to a MMIO address.
///
/// Uses inline assembly to avoid `core::ptr::write_volatile` debug assertions
/// that may panic in bare-metal environments without a working panic handler.
///
/// # Safety
///
/// `addr` must be a valid, aligned MMIO address.
#[inline(always)]
unsafe fn mmio_write32(addr: usize, val: u32) {
    asm!("str {val:w}, [{addr}]",
         addr = in(reg) addr,
         val = in(reg) val,
         options(nostack, preserves_flags));
}

/// Write a byte to a MMIO address.
///
/// # Safety
///
/// `addr` must be a valid, aligned MMIO address.
#[inline(always)]
unsafe fn mmio_write8(addr: usize, val: u8) {
    asm!("strb {val:w}, [{addr}]",
         addr = in(reg) addr,
         val = in(reg) val,
         options(nostack, preserves_flags));
}

/// Zero `.bss` before using any static `mut` or `static` that lives in BSS.
///
/// # Safety
///
/// Linker symbols `__bss_start` / `__bss_end` must bound the BSS range in RAM.
pub unsafe fn zero_bss() {
    let start = core::ptr::addr_of!(__bss_start) as usize;
    let end = core::ptr::addr_of!(__bss_end) as usize;
    let mut p = start;
    while p < end {
        mmio_write8(p, 0);
        p += 1;
    }
}

/// Zero-sized token representing the PL011 UART on QEMU `virt`.
///
/// Obtain via [`pl011()`] after [`pl011_init()`]. Implements [`fmt::Write`]
/// so you can use `write!` / `writeln!` for formatted output.
pub struct Pl011 {
    _private: (),
}

/// Returns a [`Pl011`] writer handle.
///
/// # Safety
///
/// Caller must ensure [`pl011_init()`] has been called and the MMIO region is valid.
pub unsafe fn pl011() -> Pl011 {
    Pl011 { _private: () }
}

/// Minimal PL011 init (8n1, UART enabled). Safe after MMIO region is reachable.
///
/// # Safety
///
/// Caller must be on QEMU `virt` AArch64 (or compatible PL011 at [`PL011_BASE`]).
pub unsafe fn pl011_init() {
    mmio_write32(PL011_CR, 0);
    mmio_write32(PL011_IBRD, 1);
    mmio_write32(PL011_FBRD, 0);
    mmio_write32(PL011_LCR_H, 0x70);
    mmio_write32(PL011_CR, 0x301);
}

/// Write raw bytes to PL011 TX (blocking-ish: FIFO on QEMU is generous).
///
/// # Safety
///
/// UART must be initialized ([`pl011_init`]) and MMIO must be valid.
pub unsafe fn pl011_write_bytes(bytes: &[u8]) {
    for &b in bytes {
        pl011_putc(b);
    }
}

unsafe fn pl011_putc(byte: u8) {
    mmio_write32(PL011_DR, u32::from(byte));
    asm!("dsb sy", options(nostack, preserves_flags));
}

impl fmt::Write for Pl011 {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        // SAFETY: caller of `pl011()` guaranteed UART is initialized.
        unsafe { pl011_write_bytes(s.as_bytes()) };
        Ok(())
    }
}

/// Rust entry from `_start` (see `src/bin/qemu_virt.rs`): BSS → UART → trap → GIC → IRQ → banner → idle loop.
#[no_mangle]
pub extern "C" fn kernel_main() -> ! {
    // SAFETY: early boot on `virt`; linker defines BSS bounds.
    unsafe { zero_bss() };
    // SAFETY: fixed PL011 mapping for this platform.
    unsafe { pl011_init() };
    // Step 2: Initialize frame allocator and verify
    crate::frame_alloc::init();
    // Test allocation
    if let Some(frame) = crate::frame_alloc::alloc_frame() {
        crate::println!("[step2] frame_alloc OK, allocated frame at {:#x}", frame);
        crate::frame_alloc::free_frame(frame);
    } else {
        crate::println!("[step2] ERROR: frame_alloc failed!");
    }
    // Step 3: Create page tables and dump for verification
    crate::mm::init();
    crate::println!("[step3] page tables created, dumping...");
    crate::mm::dump_tables();
    // Install VBAR before enabling the MMU so prefetch/data aborts are diagnosable.
    crate::trap::init();
    // Step 4: Enable MMU (MAIR/TCR/TTBR0 only, verify by readback)
    crate::mm::enable_mmu_step4();
    // Step 5: Enable SCTLR.M and verify
    crate::mm::enable_mmu_step5();
    // Test MMIO access with MMU enabled - PL011 UART
    crate::println!("[test] Testing PL011 UART access with MMU enabled...");
    // SAFETY: PL011 base is identity mapped.
    unsafe {
        let pl011_base = 0x0900_0000 as *mut u32;
        let dr = pl011_base.read_volatile(); // Read data register
        crate::println!("[test] PL011 DR read OK: {:#x}", dr);
    }
    crate::println!("[test] PL011 UART access OK, now testing GIC...");
    // SAFETY: GICv3 MMIO base addresses are fixed on QEMU `virt`.
    unsafe { crate::gic::init() };
    // Initialize IRQ dispatch table (must follow GIC init).
    crate::irq::init();
    // Initialize ARM Generic Timer (must follow IRQ init; registers PPI 30 handler).
    crate::timer::init();
    // Initialize preemptive FP scheduler (time-slice + same-priority RR).
    crate::task::init();
    // Initialize endpoint table (IPC MVP step 1 lifecycle only).
    crate::endpoint::init();
    // Initialize syscall dispatch table.
    crate::syscall::init();
    // Create demo tasks: task_d tests SVC syscall path from EL1.
    extern "C" fn task_a() {
        for i in 0..3 {
            crate::println!("[task A] round {}", i);
            crate::task::sleep(500);
        }
        crate::println!("[task A] done");
    }
    extern "C" fn task_b() {
        for i in 0..3 {
            crate::println!("[task B] round {}", i);
            crate::task::sleep(300);
        }
        crate::println!("[task B] done");
    }
    extern "C" fn task_d() {
        crate::println!("[task D] testing syscall via SVC...");

        let pid: u64;
        unsafe {
            asm!(
                "mov x8, #3",
                "svc #0",
                "mov {}, x0",
                out(reg) pid,
            );
        }
        crate::println!("[task D] SYS_getpid returned {}", pid);

        let msg = b"hello from SVC write!\n";
        let len = msg.len() as u64;
        let ptr = msg.as_ptr() as u64;
        let ret: u64;
        unsafe {
            asm!(
                "mov x8, #0",
                "mov x0, #1",
                "mov x1, {ptr}",
                "mov x2, {len}",
                "svc #0",
                "mov {ret}, x0",
                ptr = in(reg) ptr,
                len = in(reg) len,
                ret = out(reg) ret,
            );
        }
        crate::println!("[task D] SYS_write returned {}", ret);

        let bad_ret: u64;
        unsafe {
            asm!(
                "mov x8, #0",
                "mov x0, #1",
                "mov x1, {ptr}",
                "mov x2, #8",
                "svc #0",
                "mov {ret}, x0",
                ptr = in(reg) 0xdead_beef_u64,
                ret = out(reg) bad_ret,
            );
        }
        crate::println!(
            "[task D] SYS_write(bad ptr) returned {} (expect -14 EFAULT)",
            bad_ret as i64
        );

        let endpoint_id = 0u64;
        crate::println!("[task D] using endpoint {}", endpoint_id);

        let call_ret = loop {
            let ret: u64;
            unsafe {
                asm!(
                    "mov x8, #8",
                    "mov x0, {id}",
                    "mov x1, #42",
                    "svc #0",
                    "mov {ret}, x0",
                    id = in(reg) endpoint_id,
                    ret = out(reg) ret,
                );
            }
            if (ret as i64) == -11 {
                crate::task::yield_now();
                continue;
            }
            break ret;
        };
        crate::println!(
            "[task D] endpoint_call returned {} (expect 43)",
            call_ret as i64
        );

        crate::println!("[task D] done");
    }
    extern "C" fn task_e() {
        crate::println!("[task E] endpoint server start");

        let endpoint_id: u64;
        unsafe {
            asm!(
                "mov x8, #6",
                "svc #0",
                "mov {id}, x0",
                id = out(reg) endpoint_id,
            );
        }
        crate::println!("[task E] endpoint_create returned {}", endpoint_id);

        let packed = loop {
            let recv_ret: u64;
            unsafe {
                asm!(
                    "mov x8, #9",
                    "mov x0, {id}",
                    "svc #0",
                    "mov {out}, x0",
                    id = in(reg) endpoint_id,
                    out = out(reg) recv_ret,
                );
            }
            if (recv_ret as i64) == -11 {
                crate::task::sleep(1);
                continue;
            }
            break recv_ret;
        };
        let client_id = (packed >> 32) & 0xFFFF_FFFF;
        let request = packed & 0xFFFF_FFFF;
        crate::println!(
            "[task E] endpoint_recv got client={} msg={}",
            client_id,
            request
        );

        let reply_ret: u64;
        unsafe {
            asm!(
                "mov x8, #10",
                "mov x0, {id}",
                "mov x1, {client}",
                "mov x2, {reply}",
                "svc #0",
                "mov {ret}, x0",
                id = in(reg) endpoint_id,
                client = in(reg) client_id,
                reply = in(reg) (request + 1),
                ret = out(reg) reply_ret,
            );
        }
        crate::println!("[task E] endpoint_reply returned {}", reply_ret as i64);

        let destroy_ret: u64;
        unsafe {
            asm!(
                "mov x8, #7",
                "mov x0, {id}",
                "svc #0",
                "mov {ret}, x0",
                id = in(reg) endpoint_id,
                ret = out(reg) destroy_ret,
            );
        }
        crate::println!("[task E] endpoint_destroy returned {}", destroy_ret as i64);

        crate::println!("[task E] done");
    }
    crate::task::create(task_a, 1);
    crate::task::create(task_b, 1);
    crate::task::create(task_e, 1);
    crate::task::create(task_d, 1);
    crate::println!("[task] created tasks A, B, D, E (syscall + endpoint test)");

    // Create EL0 user task: code at 0x1000, stack top at 0x8000.
    match crate::task::create_user(0x1000, 0x8000) {
        Some(id) => {
            let user_prog_start = unsafe { &__user_program_start as *const _ as usize };
            let user_prog_end = unsafe { &__user_program_end as *const _ as usize };
            crate::println!(
                "[user] created user task {:?}, image {:#x}-{:#x}",
                id,
                user_prog_start,
                user_prog_end
            );
        }
        None => {
            crate::println!("[user] failed to create user task");
        }
    }

    // Enable IRQ exceptions at EL1 (clear DAIF.I bit).
    unsafe { asm!("msr daifclr, #2") };
    // SAFETY: UART initialized above.
    crate::println!("Hawthorn: hawthorn_kernel on QEMU virt OK");
    loop {
        crate::task::yield_now();
    }
}
