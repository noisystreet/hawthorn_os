// SPDX-License-Identifier: MIT OR Apache-2.0

//! Bare-metal QEMU `virt` image: `_start` calls `kernel_main` from `hawthorn_kernel::boot_qemu_virt`.

#![no_std]
#![no_main]

use core::arch::global_asm;
use core::panic::PanicInfo;

use hawthorn_kernel::boot_qemu_virt::{pl011_init, pl011_write_bytes};

global_asm!(
    ".section .text.boot, \"ax\"",
    ".global _start",
    "_start:",
    /* Stack at top of RAM (see `link-qemu_virt.ld`). */
    "ldr x30, =__stack_top",
    "mov sp, x30",
    "bl kernel_main",
    "0:",
    "wfi",
    "b 0b",
);

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    // SAFETY: panic path on QEMU `virt`; UART is self-contained.
    unsafe { pl011_init() };
    // SAFETY: UART initialized.
    unsafe { pl011_write_bytes(b"hawthorn_kernel: panic\n") };
    loop {
        core::hint::spin_loop();
    }
}
