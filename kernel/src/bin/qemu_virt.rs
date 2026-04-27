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
    // Like aarch64_kernel `head.S`: mask IRQ/FIQ/SError before touching device registers.
    "msr daifset, #0xf",
    // QEMU -kernel may enter at EL2 or EL1.
    // We need to ensure MMU is off and we're at EL1 for bare-metal MMIO.
    "mrs x0, CurrentEL",
    "lsr x0, x0, #2",
    "cmp x0, #2",
    "b.lt 2f",
    // ---- EL2: drop to EL1 with MMU disabled ----
    // Do **not** `msr sctlr_el1, xzr`: RES1 fields must stay set; zeroing breaks
    // MMU bring-up. Label `2` clears M|C|I via read/modify/write after `eret`.
    // Set HCR_EL2: RW=1 (EL1 is AArch64), no stage-2, no traps
    "msr hcr_el2, xzr",
    "mov x0, #(1 << 31)",
    "msr hcr_el2, x0",
    // Flush TLBs while still at EL2
    "tlbi vmalle1is",
    "tlbi alle2is",
    "isb",
    // SPSR_EL2: EL1h, all IRQ/FIQ/SError masked
    "mov x0, #0x3c5",
    "msr spsr_el2, x0",
    // Return to label 2f below
    "adr x0, 2f",
    "msr elr_el2, x0",
    "eret",
    "2:",
    // ---- EL1: MMU off; data & instruction caches off ----
    // QEMU `-kernel` may enter EL1 directly. If SCTLR.C stays 1, page-table writes
    // may not be visible to the MMU walk once M is enabled.
    // AArch64 `bic #imm` cannot encode every bit; build a mask in x1.
    "mrs x0, sctlr_el1",
    "mov x1, #5", // M | C
    "mov x2, #1",
    "lsl x2, x2, #12", // I
    "orr x1, x1, x2",
    "bic x0, x0, x1",
    "msr sctlr_el1, x0",
    "isb",
    // Flush EL1 TLB (safe at EL1)
    "tlbi vmalle1is",
    "isb",
    // QEMU may leave SPSel=0 so `mov sp` targets SP_EL0; EL1 then uses VBAR slots
    // 0x0–0x180 (generic_stub → `b .`) instead of el1_* handlers. Select SP_EL1.
    "msr spsel, #1",
    // Stack at top of RAM (see `link-qemu_virt.ld`).
    "ldr x30, =__stack_top",
    "mov sp, x30",
    "bl kernel_main",
    "0:",
    "wfi",
    "b 0b",
);

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // SAFETY: panic path on QEMU `virt`; re-init UART to guarantee output.
    unsafe { pl011_init() };
    // SAFETY: UART initialized above. Use raw write, not println!, to avoid
    // re-panicking inside core::fmt if debug assertions are active.
    unsafe { pl011_write_bytes(b"hawthorn_kernel: panic\n") };
    let _ = info;
    loop {
        core::hint::spin_loop();
    }
}
