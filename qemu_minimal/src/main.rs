//! Minimal AArch64 image for `qemu-system-aarch64 -machine virt`.
//!
//! **UART:** PL011 @ `0x9000_0000` (QEMU `virt` DTB `pl011@9000000`). See `scripts/run_qemu_minimal.sh`
//! and `docs/PORTING.md`.

#![no_std]
#![no_main]

use core::arch::{asm, global_asm};
use core::panic::PanicInfo;
use core::ptr::write_volatile;

/// PL011 base on QEMU `virt` AArch64.
const PL011_BASE: usize = 0x900_0000;
const PL011_DR: usize = PL011_BASE;
const PL011_IBRD: usize = PL011_BASE + 0x24;
const PL011_FBRD: usize = PL011_BASE + 0x28;
const PL011_LCR_H: usize = PL011_BASE + 0x2c;
const PL011_CR: usize = PL011_BASE + 0x30;

extern "C" {
    static __bss_start: u8;
    static __bss_end: u8;
}

global_asm!(
    ".section .text.boot, \"ax\"",
    ".global _start",
    "_start:",
    /* Stack at top of RAM (see `kernel/link-qemu_virt.ld`). */
    "ldr x30, =__stack_top",
    "mov sp, x30",
    "bl rust_entry",
    "0:",
    "wfi",
    "b 0b",
);

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}

#[no_mangle]
pub extern "C" fn rust_entry() -> ! {
    unsafe { zero_bss() };
    unsafe { pl011_init() };
    for b in b"Hawthorn: QEMU virt minimal OK\n" {
        unsafe { pl011_putc(*b) };
    }
    loop {
        core::hint::spin_loop();
    }
}

unsafe fn zero_bss() {
    let start = &__bss_start as *const u8 as usize;
    let end = &__bss_end as *const u8 as usize;
    let mut p = start;
    while p < end {
        (p as *mut u8).write_volatile(0);
        p += 1;
    }
}

unsafe fn pl011_init() {
    write_volatile(PL011_CR as *mut u32, 0);
    write_volatile(PL011_IBRD as *mut u32, 1);
    write_volatile(PL011_FBRD as *mut u32, 0);
    write_volatile(PL011_LCR_H as *mut u32, 0x70);
    write_volatile(PL011_CR as *mut u32, 0x301);
}

unsafe fn pl011_putc(byte: u8) {
    write_volatile(PL011_DR as *mut u32, u32::from(byte));
    /* Ensure store reaches the device before the next MMIO (weak ordering on AArch64). */
    asm!("dsb sy", options(nostack, preserves_flags));
}
