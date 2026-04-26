// SPDX-License-Identifier: MIT OR Apache-2.0

//! AArch64 exception vector table and trap handling.
//!
//! Sets up `VBAR_EL1` and provides assembly entry points for all 16 vector
//! slots. On exception entry the general-purpose registers and SP_EL0 are
//! saved into a [`TrapFrame`] on the kernel stack, then Rust
//! [`handle_exception`] is called for dispatch.
//!
//! See `docs/TRAP.md` for the full design.

use core::arch::asm;
use core::arch::global_asm;

use crate::boot_qemu_virt::{pl011_init, pl011_write_bytes};

/// Saved general-purpose register state on exception entry.
///
/// 31 registers (x0–x30) + SP_EL0 = 32 × 8 = 256 bytes, matching the
/// `sub sp, sp, #256` in the vector table assembly.
#[repr(C)]
pub struct TrapFrame {
    x: [u64; 31],
    sp_el0: u64,
}

/// Classification of the active vector slot passed to [`handle_exception`].
#[repr(u64)]
pub enum ExceptionKind {
    El1SyncSpx = 0,
    El1IrqSpx = 1,
    El1FiqSpx = 2,
    El1SErrorSpx = 3,
    El0SyncA64 = 4,
    El0IrqA64 = 5,
    El0FiqA64 = 6,
    El0SErrorA64 = 7,
}

impl TryFrom<u8> for ExceptionKind {
    type Error = ();

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::El1SyncSpx),
            1 => Ok(Self::El1IrqSpx),
            2 => Ok(Self::El1FiqSpx),
            3 => Ok(Self::El1SErrorSpx),
            4 => Ok(Self::El0SyncA64),
            5 => Ok(Self::El0IrqA64),
            6 => Ok(Self::El0FiqA64),
            7 => Ok(Self::El0SErrorA64),
            _ => Err(()),
        }
    }
}

global_asm!(
    ".section .text.vector, \"ax\"",
    ".align 12",
    ".global __exception_vector_table",
    "__exception_vector_table:",
    // ---- Current EL, SP_EL0 ----
    ".align 7",
    "b generic_stub", // 0x000 Sync
    ".align 7",
    "b generic_stub", // 0x080 IRQ
    ".align 7",
    "b generic_stub", // 0x100 FIQ
    ".align 7",
    "b generic_stub", // 0x180 SError
    // ---- Current EL, SP_ELx ----
    ".align 7",
    "b el1_sync_spx", // 0x200 Sync
    ".align 7",
    "b el1_irq_spx", // 0x280 IRQ
    ".align 7",
    "b generic_stub", // 0x300 FIQ
    ".align 7",
    "b el1_serror_spx", // 0x380 SError
    // ---- Lower EL, AArch64 ----
    ".align 7",
    "b el0_sync_a64", // 0x400 Sync
    ".align 7",
    "b el0_irq_a64", // 0x480 IRQ
    ".align 7",
    "b generic_stub", // 0x500 FIQ
    ".align 7",
    "b el0_serror_a64", // 0x580 SError
    // ---- Lower EL, AArch32 ----
    ".align 7",
    "b generic_stub", // 0x600 Sync
    ".align 7",
    "b generic_stub", // 0x680 IRQ
    ".align 7",
    "b generic_stub", // 0x700 FIQ
    ".align 7",
    "b generic_stub", // 0x780 SError
);

global_asm!(
    ".text",
    ".align 4",
    "generic_stub:",
    "b .",
    // ---- EL1 Sync (SPx) @ 0x200 ----
    ".align 4",
    "el1_sync_spx:",
    "sub sp, sp, #256",
    "stp x0, x1,   [sp, #0]",
    "stp x2, x3,   [sp, #16]",
    "stp x4, x5,   [sp, #32]",
    "stp x6, x7,   [sp, #48]",
    "stp x8, x9,   [sp, #64]",
    "stp x10, x11, [sp, #80]",
    "stp x12, x13, [sp, #96]",
    "stp x14, x15, [sp, #112]",
    "stp x16, x17, [sp, #128]",
    "stp x18, x19, [sp, #144]",
    "stp x20, x21, [sp, #160]",
    "stp x22, x23, [sp, #176]",
    "stp x24, x25, [sp, #192]",
    "stp x26, x27, [sp, #208]",
    "stp x28, x29, [sp, #224]",
    "str x30,      [sp, #240]",
    "mrs x0, sp_el0",
    "str x0,       [sp, #248]",
    "mov x0, #0",
    "mrs x3, elr_el1",
    "mrs x4, spsr_el1",
    "mov x2, sp",
    "bl handle_exception",
    "ldr x0, [sp, #248]",
    "msr sp_el0, x0",
    "ldp x0, x1,   [sp, #0]",
    "ldp x2, x3,   [sp, #16]",
    "ldp x4, x5,   [sp, #32]",
    "ldp x6, x7,   [sp, #48]",
    "ldp x8, x9,   [sp, #64]",
    "ldp x10, x11, [sp, #80]",
    "ldp x12, x13, [sp, #96]",
    "ldp x14, x15, [sp, #112]",
    "ldp x16, x17, [sp, #128]",
    "ldp x18, x19, [sp, #144]",
    "ldp x20, x21, [sp, #160]",
    "ldp x22, x23, [sp, #176]",
    "ldp x24, x25, [sp, #192]",
    "ldp x26, x27, [sp, #208]",
    "ldp x28, x29, [sp, #224]",
    "ldr x30,      [sp, #240]",
    "add sp, sp, #256",
    "eret",
    // ---- EL1 IRQ (SPx) @ 0x280 ----
    ".align 4",
    "el1_irq_spx:",
    "sub sp, sp, #256",
    "stp x0, x1,   [sp, #0]",
    "stp x2, x3,   [sp, #16]",
    "stp x4, x5,   [sp, #32]",
    "stp x6, x7,   [sp, #48]",
    "stp x8, x9,   [sp, #64]",
    "stp x10, x11, [sp, #80]",
    "stp x12, x13, [sp, #96]",
    "stp x14, x15, [sp, #112]",
    "stp x16, x17, [sp, #128]",
    "stp x18, x19, [sp, #144]",
    "stp x20, x21, [sp, #160]",
    "stp x22, x23, [sp, #176]",
    "stp x24, x25, [sp, #192]",
    "stp x26, x27, [sp, #208]",
    "stp x28, x29, [sp, #224]",
    "str x30,      [sp, #240]",
    "mrs x0, sp_el0",
    "str x0,       [sp, #248]",
    "mov x0, #1",
    "mrs x3, elr_el1",
    "mrs x4, spsr_el1",
    "mov x2, sp",
    "bl handle_exception",
    "ldr x0, [sp, #248]",
    "msr sp_el0, x0",
    "ldp x0, x1,   [sp, #0]",
    "ldp x2, x3,   [sp, #16]",
    "ldp x4, x5,   [sp, #32]",
    "ldp x6, x7,   [sp, #48]",
    "ldp x8, x9,   [sp, #64]",
    "ldp x10, x11, [sp, #80]",
    "ldp x12, x13, [sp, #96]",
    "ldp x14, x15, [sp, #112]",
    "ldp x16, x17, [sp, #128]",
    "ldp x18, x19, [sp, #144]",
    "ldp x20, x21, [sp, #160]",
    "ldp x22, x23, [sp, #176]",
    "ldp x24, x25, [sp, #192]",
    "ldp x26, x27, [sp, #208]",
    "ldp x28, x29, [sp, #224]",
    "ldr x30,      [sp, #240]",
    "add sp, sp, #256",
    "eret",
    // ---- EL1 SError (SPx) @ 0x380 ----
    ".align 4",
    "el1_serror_spx:",
    "sub sp, sp, #256",
    "stp x0, x1,   [sp, #0]",
    "stp x2, x3,   [sp, #16]",
    "stp x4, x5,   [sp, #32]",
    "stp x6, x7,   [sp, #48]",
    "stp x8, x9,   [sp, #64]",
    "stp x10, x11, [sp, #80]",
    "stp x12, x13, [sp, #96]",
    "stp x14, x15, [sp, #112]",
    "stp x16, x17, [sp, #128]",
    "stp x18, x19, [sp, #144]",
    "stp x20, x21, [sp, #160]",
    "stp x22, x23, [sp, #176]",
    "stp x24, x25, [sp, #192]",
    "stp x26, x27, [sp, #208]",
    "stp x28, x29, [sp, #224]",
    "str x30,      [sp, #240]",
    "mrs x0, sp_el0",
    "str x0,       [sp, #248]",
    "mov x0, #3",
    "mrs x3, elr_el1",
    "mrs x4, spsr_el1",
    "mov x2, sp",
    "bl handle_exception",
    "ldr x0, [sp, #248]",
    "msr sp_el0, x0",
    "ldp x0, x1,   [sp, #0]",
    "ldp x2, x3,   [sp, #16]",
    "ldp x4, x5,   [sp, #32]",
    "ldp x6, x7,   [sp, #48]",
    "ldp x8, x9,   [sp, #64]",
    "ldp x10, x11, [sp, #80]",
    "ldp x12, x13, [sp, #96]",
    "ldp x14, x15, [sp, #112]",
    "ldp x16, x17, [sp, #128]",
    "ldp x18, x19, [sp, #144]",
    "ldp x20, x21, [sp, #160]",
    "ldp x22, x23, [sp, #176]",
    "ldp x24, x25, [sp, #192]",
    "ldp x26, x27, [sp, #208]",
    "ldp x28, x29, [sp, #224]",
    "ldr x30,      [sp, #240]",
    "add sp, sp, #256",
    "eret",
    // ---- EL0 Sync (AArch64) @ 0x400 ----
    ".align 4",
    "el0_sync_a64:",
    "sub sp, sp, #256",
    "stp x0, x1,   [sp, #0]",
    "stp x2, x3,   [sp, #16]",
    "stp x4, x5,   [sp, #32]",
    "stp x6, x7,   [sp, #48]",
    "stp x8, x9,   [sp, #64]",
    "stp x10, x11, [sp, #80]",
    "stp x12, x13, [sp, #96]",
    "stp x14, x15, [sp, #112]",
    "stp x16, x17, [sp, #128]",
    "stp x18, x19, [sp, #144]",
    "stp x20, x21, [sp, #160]",
    "stp x22, x23, [sp, #176]",
    "stp x24, x25, [sp, #192]",
    "stp x26, x27, [sp, #208]",
    "stp x28, x29, [sp, #224]",
    "str x30,      [sp, #240]",
    "mrs x0, sp_el0",
    "str x0,       [sp, #248]",
    "mov x0, #4",
    "mrs x3, elr_el1",
    "mrs x4, spsr_el1",
    "mov x2, sp",
    "bl handle_exception",
    "ldr x0, [sp, #248]",
    "msr sp_el0, x0",
    "ldp x0, x1,   [sp, #0]",
    "ldp x2, x3,   [sp, #16]",
    "ldp x4, x5,   [sp, #32]",
    "ldp x6, x7,   [sp, #48]",
    "ldp x8, x9,   [sp, #64]",
    "ldp x10, x11, [sp, #80]",
    "ldp x12, x13, [sp, #96]",
    "ldp x14, x15, [sp, #112]",
    "ldp x16, x17, [sp, #128]",
    "ldp x18, x19, [sp, #144]",
    "ldp x20, x21, [sp, #160]",
    "ldp x22, x23, [sp, #176]",
    "ldp x24, x25, [sp, #192]",
    "ldp x26, x27, [sp, #208]",
    "ldp x28, x29, [sp, #224]",
    "ldr x30,      [sp, #240]",
    "add sp, sp, #256",
    "eret",
    // ---- EL0 IRQ (AArch64) @ 0x480 ----
    ".align 4",
    "el0_irq_a64:",
    "sub sp, sp, #256",
    "stp x0, x1,   [sp, #0]",
    "stp x2, x3,   [sp, #16]",
    "stp x4, x5,   [sp, #32]",
    "stp x6, x7,   [sp, #48]",
    "stp x8, x9,   [sp, #64]",
    "stp x10, x11, [sp, #80]",
    "stp x12, x13, [sp, #96]",
    "stp x14, x15, [sp, #112]",
    "stp x16, x17, [sp, #128]",
    "stp x18, x19, [sp, #144]",
    "stp x20, x21, [sp, #160]",
    "stp x22, x23, [sp, #176]",
    "stp x24, x25, [sp, #192]",
    "stp x26, x27, [sp, #208]",
    "stp x28, x29, [sp, #224]",
    "str x30,      [sp, #240]",
    "mrs x0, sp_el0",
    "str x0,       [sp, #248]",
    "mov x0, #5",
    "mrs x3, elr_el1",
    "mrs x4, spsr_el1",
    "mov x2, sp",
    "bl handle_exception",
    "ldr x0, [sp, #248]",
    "msr sp_el0, x0",
    "ldp x0, x1,   [sp, #0]",
    "ldp x2, x3,   [sp, #16]",
    "ldp x4, x5,   [sp, #32]",
    "ldp x6, x7,   [sp, #48]",
    "ldp x8, x9,   [sp, #64]",
    "ldp x10, x11, [sp, #80]",
    "ldp x12, x13, [sp, #96]",
    "ldp x14, x15, [sp, #112]",
    "ldp x16, x17, [sp, #128]",
    "ldp x18, x19, [sp, #144]",
    "ldp x20, x21, [sp, #160]",
    "ldp x22, x23, [sp, #176]",
    "ldp x24, x25, [sp, #192]",
    "ldp x26, x27, [sp, #208]",
    "ldp x28, x29, [sp, #224]",
    "ldr x30,      [sp, #240]",
    "add sp, sp, #256",
    "eret",
    // ---- EL0 SError (AArch64) @ 0x580 ----
    ".align 4",
    "el0_serror_a64:",
    "sub sp, sp, #256",
    "stp x0, x1,   [sp, #0]",
    "stp x2, x3,   [sp, #16]",
    "stp x4, x5,   [sp, #32]",
    "stp x6, x7,   [sp, #48]",
    "stp x8, x9,   [sp, #64]",
    "stp x10, x11, [sp, #80]",
    "stp x12, x13, [sp, #96]",
    "stp x14, x15, [sp, #112]",
    "stp x16, x17, [sp, #128]",
    "stp x18, x19, [sp, #144]",
    "stp x20, x21, [sp, #160]",
    "stp x22, x23, [sp, #176]",
    "stp x24, x25, [sp, #192]",
    "stp x26, x27, [sp, #208]",
    "stp x28, x29, [sp, #224]",
    "str x30,      [sp, #240]",
    "mrs x0, sp_el0",
    "str x0,       [sp, #248]",
    "mov x0, #7",
    "mrs x3, elr_el1",
    "mrs x4, spsr_el1",
    "mov x2, sp",
    "bl handle_exception",
    "ldr x0, [sp, #248]",
    "msr sp_el0, x0",
    "ldp x0, x1,   [sp, #0]",
    "ldp x2, x3,   [sp, #16]",
    "ldp x4, x5,   [sp, #32]",
    "ldp x6, x7,   [sp, #48]",
    "ldp x8, x9,   [sp, #64]",
    "ldp x10, x11, [sp, #80]",
    "ldp x12, x13, [sp, #96]",
    "ldp x14, x15, [sp, #112]",
    "ldp x16, x17, [sp, #128]",
    "ldp x18, x19, [sp, #144]",
    "ldp x20, x21, [sp, #160]",
    "ldp x22, x23, [sp, #176]",
    "ldp x24, x25, [sp, #192]",
    "ldp x26, x27, [sp, #208]",
    "ldp x28, x29, [sp, #224]",
    "ldr x30,      [sp, #240]",
    "add sp, sp, #256",
    "eret",
);

fn read_esr() -> u64 {
    let esr: u64;
    // SAFETY: reading a system register has no side effects.
    unsafe { asm!("mrs {}, esr_el1", out(reg) esr) };
    esr
}

fn read_far() -> u64 {
    let far: u64;
    // SAFETY: reading a system register has no side effects.
    unsafe { asm!("mrs {}, far_el1", out(reg) far) };
    far
}

fn dump_exception(kind: ExceptionKind, elr: u64, spsr: u64) {
    let esr = read_esr();
    let far = read_far();
    let ec = (esr >> 26) & 0x3F;
    let iss = esr & 0x1FFFFFF;

    unsafe { pl011_init() };

    match kind {
        ExceptionKind::El1SyncSpx => {
            unsafe { pl011_write_bytes(b"[EXCEPTION] EL1 Sync (SPx)\n") };
        }
        ExceptionKind::El1SErrorSpx => {
            unsafe { pl011_write_bytes(b"[EXCEPTION] EL1 SError (SPx)\n") };
        }
        ExceptionKind::El0SyncA64 => {
            unsafe { pl011_write_bytes(b"[EXCEPTION] EL0 Sync (AArch64)\n") };
        }
        ExceptionKind::El0SErrorA64 => {
            unsafe { pl011_write_bytes(b"[EXCEPTION] EL0 SError (AArch64)\n") };
        }
        ExceptionKind::El1IrqSpx => {
            unsafe { pl011_write_bytes(b"[EXCEPTION] EL1 IRQ (SPx)\n") };
        }
        ExceptionKind::El0IrqA64 => {
            unsafe { pl011_write_bytes(b"[EXCEPTION] EL0 IRQ (AArch64)\n") };
        }
        _ => {
            unsafe { pl011_write_bytes(b"[EXCEPTION] Unknown\n") };
        }
    }

    crate::println!("  ESR={:#010x} EC={:#04x} ISS={:#07x}", esr, ec, iss);
    crate::println!("  FAR={:#018x}", far);
    crate::println!("  ELR={:#018x} SPSR={:#010x}", elr, spsr);
}

#[no_mangle]
unsafe extern "C" fn handle_exception(
    kind: u64,
    _reserved: u64,
    _trap_frame: *mut TrapFrame,
    elr: u64,
    spsr: u64,
) {
    let Ok(kind) = u8::try_from(kind).map(|k| k.try_into()) else {
        loop {
            core::hint::spin_loop();
        }
    };

    match kind {
        Ok(ExceptionKind::El1IrqSpx) | Ok(ExceptionKind::El0IrqA64) => {
            // M2 stub: no GIC yet, just return.
        }
        Ok(ExceptionKind::El1SyncSpx)
        | Ok(ExceptionKind::El1SErrorSpx)
        | Ok(ExceptionKind::El0SyncA64)
        | Ok(ExceptionKind::El0SErrorA64) => {
            dump_exception(kind.unwrap(), elr, spsr);
            loop {
                core::hint::spin_loop();
            }
        }
        _ => loop {
            core::hint::spin_loop();
        },
    }
}

/// Install the exception vector table by writing its address to `VBAR_EL1`.
///
/// Must be called once during early boot, after BSS and UART are initialized.
pub fn init() {
    extern "C" {
        static __exception_vector_table: u8;
    }
    // SAFETY: the vector table symbol is defined by `global_asm!` above and
    // placed in `.text.vector`; its address is a valid, aligned VBAR value.
    unsafe {
        let vbar = core::ptr::addr_of!(__exception_vector_table) as u64;
        asm!("msr vbar_el1, {}", in(reg) vbar);
        asm!("isb");
    }
}
