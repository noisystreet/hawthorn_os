// SPDX-License-Identifier: MIT OR Apache-2.0

//! QEMU `virt` AArch64 bring-up: BSS, PL011 @ `0x9000_0000`, and `kernel_main`.

use core::arch::asm;
use core::ptr::write_volatile;

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
        (p as *mut u8).write_volatile(0);
        p += 1;
    }
}

/// Minimal PL011 init (8n1, UART enabled). Safe after MMIO region is reachable.
///
/// # Safety
///
/// Caller must be on QEMU `virt` AArch64 (or compatible PL011 at [`PL011_BASE`]).
pub unsafe fn pl011_init() {
    write_volatile(PL011_CR as *mut u32, 0);
    write_volatile(PL011_IBRD as *mut u32, 1);
    write_volatile(PL011_FBRD as *mut u32, 0);
    write_volatile(PL011_LCR_H as *mut u32, 0x70);
    write_volatile(PL011_CR as *mut u32, 0x301);
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
    write_volatile(PL011_DR as *mut u32, u32::from(byte));
    // Ensure store reaches the device before the next MMIO (weak ordering on AArch64).
    asm!("dsb sy", options(nostack, preserves_flags));
}

/// Rust entry from `_start` (see `src/bin/qemu_virt.rs`): BSS → UART → banner → idle loop.
#[no_mangle]
pub extern "C" fn kernel_main() -> ! {
    // SAFETY: early boot on `virt`; linker defines BSS bounds.
    unsafe { zero_bss() };
    // SAFETY: fixed PL011 mapping for this platform.
    unsafe { pl011_init() };
    // SAFETY: UART initialized.
    unsafe { pl011_write_bytes(b"Hawthorn: hawthorn_kernel on QEMU virt OK\n") };
    loop {
        core::hint::spin_loop();
    }
}
