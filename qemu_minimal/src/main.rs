//! Minimal AArch64 image for `qemu-system-aarch64 -machine virt`.
//!
//! Reuses PL011 UART driver and BSS zeroing from `hawthorn_kernel::boot_qemu_virt`.
//! See `scripts/run_qemu_minimal.sh` and `docs/PORTING.md`.

#![no_std]
#![no_main]

use core::arch::global_asm;
use core::panic::PanicInfo;

use hawthorn_kernel::boot_qemu_virt;

global_asm!(
    ".section .text.boot, \"ax\"",
    ".global _start",
    "_start:",
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
    unsafe { boot_qemu_virt::zero_bss() };
    unsafe { boot_qemu_virt::pl011_init() };
    unsafe { boot_qemu_virt::pl011_write_bytes(b"Hawthorn: QEMU virt minimal OK\n") };
    loop {
        core::hint::spin_loop();
    }
}
