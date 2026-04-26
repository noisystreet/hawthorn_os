// SPDX-License-Identifier: MIT OR Apache-2.0

//! GICv3 (Generic Interrupt Controller v3) driver for QEMU `virt` AArch64.
//!
//! Supports single-core initialization of:
//! - Distributor (GICD): global interrupt configuration
//! - Redistributor (GICR): per-core configuration
//! - CPU Interface (ICC): priority masking and enable
//!
//! MMIO layout for QEMU virt:
//! - GICD: 0x0800_0000, size 64 KiB
//! - GICR RD base: 0x080A_0000, size 64 KiB (control registers)
//! - GICR SGI base: 0x080B_0000, size 64 KiB (SGI/PPI configuration)
//!
//! Reference: ARM IHI 0069F (GICv3/v4 Architecture Specification)

use core::arch::asm;

/// GIC Distributor base address (QEMU virt).
const GICD_BASE: usize = 0x0800_0000;
/// GIC Redistributor RD base address (QEMU virt, single core).
const GICR_BASE: usize = 0x080A_0000;
/// GIC Redistributor SGI base address (QEMU virt, single core).
/// Each redistributor has two 64 KiB pages: RD base + SGI base.
const GICR_SGI_BASE: usize = GICR_BASE + 0x1_0000;

// Distributor register offsets
const GICD_CTLR: usize = GICD_BASE;
const GICD_TYPER: usize = GICD_BASE + 0x0004;
const GICD_IGROUPR: usize = GICD_BASE + 0x0080;
const GICD_ISENABLER: usize = GICD_BASE + 0x0100;
const GICD_ICENABLER: usize = GICD_BASE + 0x0180;
const GICD_ICPENDR: usize = GICD_BASE + 0x0280;
const GICD_ICACTIVER: usize = GICD_BASE + 0x0380;
const GICD_IPRIORITYR: usize = GICD_BASE + 0x0400;
#[allow(dead_code)]
const GICD_ITARGETSR: usize = GICD_BASE + 0x0800;
#[allow(dead_code)]
const GICD_ICFGR: usize = GICD_BASE + 0x0C00;

// Redistributor RD base page registers
#[allow(dead_code)]
const GICR_CTLR: usize = GICR_BASE;
#[allow(dead_code)]
const GICR_TYPER: usize = GICR_BASE + 0x0008;
const GICR_WAKER: usize = GICR_BASE + 0x0014;

// Redistributor SGI base page registers (GICR_SGI_BASE + offset)
const GICR_IGROUPR0: usize = GICR_SGI_BASE + 0x0080;
const GICR_ISENABLER0: usize = GICR_SGI_BASE + 0x0100;
const GICR_ICENABLER0: usize = GICR_SGI_BASE + 0x0180;
const GICR_ICPENDR0: usize = GICR_SGI_BASE + 0x0280;
const GICR_ICACTIVER0: usize = GICR_SGI_BASE + 0x0380;
const GICR_IPRIORITYR0: usize = GICR_SGI_BASE + 0x0400;
#[allow(dead_code)]
const GICR_ICFGR0: usize = GICR_SGI_BASE + 0x0C00;

/// Maximum SPI number supported (from GICD_TYPER).
static mut GIC_MAX_SPI: u32 = 0;

/// Write 32-bit value to MMIO address.
///
/// # Safety
/// `addr` must be a valid, aligned MMIO address.
#[inline(always)]
unsafe fn mmio_write32(addr: usize, val: u32) {
    asm!("str {val:w}, [{addr}]",
         addr = in(reg) addr,
         val = in(reg) val,
         options(nostack, preserves_flags));
}

/// Read 32-bit value from MMIO address.
///
/// # Safety
/// `addr` must be a valid, aligned MMIO address.
#[inline(always)]
unsafe fn mmio_read32(addr: usize) -> u32 {
    let val: u32;
    asm!("ldr {val:w}, [{addr}]",
         addr = in(reg) addr,
         val = out(reg) val,
         options(nostack, preserves_flags));
    val
}

/// Enable system register access to ICC_* registers (EL1).
///
/// Sets ICC_SRE_EL1.SRE = 1 to allow MRS/MSR access instead of MMIO.
fn gicv3_cpuif_enable_sre() {
    let sre: u64;
    // SAFETY: reading/writing ICC_SRE_EL1 is safe at EL1.
    unsafe {
        asm!("mrs {}, s3_0_c12_c12_5", out(reg) sre);
        asm!("msr s3_0_c12_c12_5, {}", in(reg) sre | 0x1);
        asm!("isb");
    }
}

/// Initialize GICv3 Distributor.
///
/// # Safety
/// Must be called once during early boot, before enabling interrupts.
unsafe fn gicv3_dist_init() {
    // Disable distributor
    mmio_write32(GICD_CTLR, 0);

    // Wait for RWP (Register Write Pending) to clear
    while mmio_read32(GICD_CTLR) & 0x80000000 != 0 {}

    // Read GICD_TYPER to determine max SPI
    let typer = mmio_read32(GICD_TYPER);
    let itlines = (typer & 0x1F) as usize;
    GIC_MAX_SPI = (32 * (itlines + 1) - 1) as u32;

    // Configure all SPIs as non-secure Group 1
    let num_regs = (GIC_MAX_SPI as usize + 1) / 32;
    for i in 0..num_regs {
        mmio_write32(GICD_IGROUPR + i * 4, 0xFFFF_FFFF);
    }

    // Set default priority for all SPIs (lowest priority)
    let num_priority_regs = (GIC_MAX_SPI as usize + 1) * 8 / 32;
    for i in 0..num_priority_regs {
        mmio_write32(GICD_IPRIORITYR + i * 4, 0x8080_8080);
    }

    // Deactivate and clear pending for all SPIs
    for i in 0..num_regs {
        mmio_write32(GICD_ICACTIVER + i * 4, 0xFFFF_FFFF);
        mmio_write32(GICD_ICPENDR + i * 4, 0xFFFF_FFFF);
    }

    // Enable distributor (EnableGrp1NS = bit 0, EnableGrp1A = bit 1)
    mmio_write32(GICD_CTLR, 0x3);

    // Wait for RWP to clear after enabling
    while mmio_read32(GICD_CTLR) & 0x80000000 != 0 {}
}

/// Initialize GICv3 Redistributor (for current core).
///
/// # Safety
/// Must be called once per core during early boot.
unsafe fn gicv3_redist_init() {
    // Wake up the redistributor: clear ProcessorSleep bit
    let waker = mmio_read32(GICR_WAKER);
    mmio_write32(GICR_WAKER, waker & !0x0000_0002);
    // Wait for ChildrenAsleep to clear (indicates redistributor is ready)
    while mmio_read32(GICR_WAKER) & 0x0000_0004 != 0 {}

    // Configure SGIs and PPIs (ID 0-31) as non-secure Group 1
    mmio_write32(GICR_IGROUPR0, 0xFFFF_FFFF);

    // Set default priority for SGIs and PPIs
    for i in 0..8 {
        mmio_write32(GICR_IPRIORITYR0 + i * 4, 0x8080_8080);
    }

    // Deactivate and clear pending for all SGIs and PPIs
    mmio_write32(GICR_ICACTIVER0, 0xFFFF_FFFF);
    mmio_write32(GICR_ICPENDR0, 0xFFFF_FFFF);
}

/// Initialize GICv3 CPU Interface.
///
/// # Safety
/// Must be called once per core after distributor and redistributor init.
unsafe fn gicv3_cpuif_init() {
    // Enable system register access
    gicv3_cpuif_enable_sre();

    // Set priority mask (allow all priorities)
    asm!("msr s3_0_c4_c6_0, {}", in(reg) 0xFFu64);

    // Set binary point registers
    asm!("msr s3_0_c12_c8_3, {}", in(reg) 0u64); // ICC_BPR0_EL1
    asm!("msr s3_0_c12_c12_3, {}", in(reg) 0u64); // ICC_BPR1_EL1

    // Enable Group 1 interrupts
    asm!("msr s3_0_c12_c12_7, {}", in(reg) 1u64); // ICC_IGRPEN1_EL1

    asm!("isb");
}

/// Initialize GICv3 (distributor + redistributor + CPU interface).
///
/// # Safety
/// Must be called once during early boot on single core, before enabling interrupts.
pub unsafe fn init() {
    gicv3_dist_init();
    gicv3_redist_init();
    gicv3_cpuif_init();
}

/// Enable a specific SPI (Shared Peripheral Interrupt).
///
/// # Safety
/// `irq` must be a valid SPI number (32 <= irq <= GIC_MAX_SPI).
pub unsafe fn enable_spi(irq: u32) {
    if irq < 32 || irq > GIC_MAX_SPI {
        return;
    }
    let reg = (irq / 32) as usize * 4;
    let bit = 1u32 << (irq % 32);
    mmio_write32(GICD_ISENABLER + reg, bit);
}

/// Disable a specific SPI.
///
/// # Safety
/// `irq` must be a valid SPI number.
pub unsafe fn disable_spi(irq: u32) {
    if irq < 32 || irq > GIC_MAX_SPI {
        return;
    }
    let reg = (irq / 32) as usize * 4;
    let bit = 1u32 << (irq % 32);
    mmio_write32(GICD_ICENABLER + reg, bit);
}

/// Enable a specific PPI (Private Peripheral Interrupt, ID 16-31).
///
/// # Safety
/// `irq` must be 16 <= irq < 32.
pub unsafe fn enable_ppi(irq: u32) {
    if !(16..32).contains(&irq) {
        return;
    }
    let bit = 1u32 << irq;
    mmio_write32(GICR_ISENABLER0, bit);
}

/// Disable a specific PPI.
///
/// # Safety
/// `irq` must be 16 <= irq < 32.
pub unsafe fn disable_ppi(irq: u32) {
    if !(16..32).contains(&irq) {
        return;
    }
    let bit = 1u32 << irq;
    mmio_write32(GICR_ICENABLER0, bit);
}

/// Set priority for an interrupt.
///
/// # Safety
/// `irq` must be valid; `priority` is 0-255 (lower = higher priority).
pub unsafe fn set_priority(irq: u32, priority: u8) {
    if irq > GIC_MAX_SPI {
        return;
    }

    let reg = (irq as usize * 8) / 32 * 4;
    let shift = (irq % 4) * 8;

    if irq < 32 {
        // SGI/PPI via redistributor SGI base page
        let val = mmio_read32(GICR_IPRIORITYR0 + reg);
        let new_val = (val & !(0xFF << shift)) | ((priority as u32) << shift);
        mmio_write32(GICR_IPRIORITYR0 + reg, new_val);
    } else {
        // SPI via distributor
        let val = mmio_read32(GICD_IPRIORITYR + reg);
        let new_val = (val & !(0xFF << shift)) | ((priority as u32) << shift);
        mmio_write32(GICD_IPRIORITYR + reg, new_val);
    }
}

/// Acknowledge interrupt and return IRQ number.
///
/// Returns the interrupt ID (0-1019) or 1023 (spurious).
pub fn ack() -> u32 {
    let iar: u64;
    // SAFETY: reading ICC_IAR1_EL1 is safe.
    unsafe {
        asm!("mrs {}, s3_0_c12_c12_0", out(reg) iar);
    }
    (iar & 0x3FF) as u32
}

/// Signal End of Interrupt.
///
/// # Safety
/// `irq` must be the same value returned by `ack()`.
pub unsafe fn eoi(irq: u32) {
    asm!("msr s3_0_c12_c12_1, {}", in(reg) irq as u64);
}
