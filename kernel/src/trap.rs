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
/// Layout matches the vector stubs: 31 GPRs (x0–x30) + `sp_el0`, then
/// `elr_el1` / `spsr_el1` captured on entry so `eret` can restore the correct
/// return address after another task took an exception (ELR_EL1 is not
/// per-thread). Total 272 bytes; see `sub sp, sp, #272` in assembly.
#[repr(C)]
pub struct TrapFrame {
    x: [u64; 31],
    sp_el0: u64,
    elr_el1: u64,
    spsr_el1: u64,
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
    "sub sp, sp, #272",
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
    "str x3,       [sp, #256]",
    "str x4,       [sp, #264]",
    "mov x2, sp",
    "mov x1, #0",
    "bl handle_exception",
    "ldr x0, [sp, #248]",
    "msr sp_el0, x0",
    "ldr x9,       [sp, #256]",
    "msr elr_el1, x9",
    "ldr x9,       [sp, #264]",
    "msr spsr_el1, x9",
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
    "add sp, sp, #272",
    "eret",
    // ---- EL1 IRQ (SPx) @ 0x280 ----
    ".align 4",
    "el1_irq_spx:",
    "sub sp, sp, #272",
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
    "str x3,       [sp, #256]",
    "str x4,       [sp, #264]",
    "mov x2, sp",
    "mov x1, #0",
    "bl handle_exception",
    "ldr x0, [sp, #248]",
    "msr sp_el0, x0",
    "ldr x9,       [sp, #256]",
    "msr elr_el1, x9",
    "ldr x9,       [sp, #264]",
    "msr spsr_el1, x9",
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
    "add sp, sp, #272",
    "eret",
    // ---- EL1 SError (SPx) @ 0x380 ----
    ".align 4",
    "el1_serror_spx:",
    "sub sp, sp, #272",
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
    "str x3,       [sp, #256]",
    "str x4,       [sp, #264]",
    "mov x2, sp",
    "mov x1, #0",
    "bl handle_exception",
    "ldr x0, [sp, #248]",
    "msr sp_el0, x0",
    "ldr x9,       [sp, #256]",
    "msr elr_el1, x9",
    "ldr x9,       [sp, #264]",
    "msr spsr_el1, x9",
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
    "add sp, sp, #272",
    "eret",
    // ---- EL0 Sync (AArch64) @ 0x400 ----
    ".align 4",
    "el0_sync_a64:",
    "sub sp, sp, #272",
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
    "str x3,       [sp, #256]",
    "str x4,       [sp, #264]",
    "mov x2, sp",
    "mov x1, #0",
    "bl handle_exception",
    "ldr x0, [sp, #248]",
    "msr sp_el0, x0",
    "ldr x9,       [sp, #256]",
    "msr elr_el1, x9",
    "ldr x9,       [sp, #264]",
    "msr spsr_el1, x9",
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
    "add sp, sp, #272",
    "eret",
    // ---- EL0 IRQ (AArch64) @ 0x480 ----
    ".align 4",
    "el0_irq_a64:",
    "sub sp, sp, #272",
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
    "str x3,       [sp, #256]",
    "str x4,       [sp, #264]",
    "mov x2, sp",
    "mov x1, #0",
    "bl handle_exception",
    "ldr x0, [sp, #248]",
    "msr sp_el0, x0",
    "ldr x9,       [sp, #256]",
    "msr elr_el1, x9",
    "ldr x9,       [sp, #264]",
    "msr spsr_el1, x9",
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
    "add sp, sp, #272",
    "eret",
    // ---- EL0 SError (AArch64) @ 0x580 ----
    ".align 4",
    "el0_serror_a64:",
    "sub sp, sp, #272",
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
    "str x3,       [sp, #256]",
    "str x4,       [sp, #264]",
    "mov x2, sp",
    "mov x1, #0",
    "bl handle_exception",
    "ldr x0, [sp, #248]",
    "msr sp_el0, x0",
    "ldr x9,       [sp, #256]",
    "msr elr_el1, x9",
    "ldr x9,       [sp, #264]",
    "msr spsr_el1, x9",
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
    "add sp, sp, #272",
    "eret",
);

// User task return path: restore EL0 state and eret to user mode
// This is called from context_switch when switching to a user task
global_asm!(
    ".global user_return",
    ".type user_return, @function",
    ".align 4",
    "user_return:",
    // x0 = task struct pointer (from task table)
    // Restore user state and eret to EL0

    // Load saved_elr, saved_spsr, saved_sp_el0 from task struct
    // Task struct layout (after sp at offset 0):
    //   0: sp (u64)
    //   8: state (u8) + padding
    //   16: priority (u8) + id (u16) + padding
    //   24: time_slice (u64)
    //   32: daif (u64)
    //   40: wake_tick (u64)
    //   40: is_user (bool) + 7-byte padding
    //   48: user_page_table (usize)
    //   56: saved_elr (u64)
    //   64: saved_spsr (u64)
    //   72: saved_sp_el0 (u64)

    // Load saved_elr
    "ldr x1, [x0, #56]",
    "msr elr_el1, x1",
    // Load saved_spsr
    "ldr x1, [x0, #64]",
    "msr spsr_el1, x1",
    // Load saved_sp_el0
    "ldr x1, [x0, #72]",
    "msr sp_el0, x1",
    // Load user_page_table and switch TTBR0
    "ldr x1, [x0, #48]",
    "msr ttbr0_el1, x1",
    "isb",
    "tlbi vmalle1is",
    "isb",
    // Now we need to restore x0-x30 from the task's kernel stack
    // The context was saved by context_switch
    "ldr x1, [x0]", // Load sp (kernel stack pointer)
    // Restore callee-saved registers from kernel stack
    "ldp x19, x20, [x1], #16",
    "ldp x21, x22, [x1], #16",
    "ldp x23, x24, [x1], #16",
    "ldp x25, x26, [x1], #16",
    "ldp x27, x28, [x1], #16",
    "ldp x29, x30, [x1], #16",
    // Restore caller-saved registers (x0-x18) from trap frame
    // For initial entry, we set x0-x18 to 0
    // For syscall return, they were saved in trap frame

    // Clear x0-x18 for initial entry
    "mov x0, #0",
    "mov x1, #0",
    "mov x2, #0",
    "mov x3, #0",
    "mov x4, #0",
    "mov x5, #0",
    "mov x6, #0",
    "mov x7, #0",
    "mov x8, #0",
    "mov x9, #0",
    "mov x10, #0",
    "mov x11, #0",
    "mov x12, #0",
    "mov x13, #0",
    "mov x14, #0",
    "mov x15, #0",
    "mov x16, #0",
    "mov x17, #0",
    "mov x18, #0",
    // Eret to EL0
    "eret",
);

/// EL1 syscall path in its own stack frame so `trap_frame` survives `dispatch()`
/// (which may block/reschedule) without relying on a spill slot in the large
/// `handle_exception` frame — that slot could be clobbered across deep calls.
#[inline(never)]
unsafe fn handle_el1_syscall(_trap_frame: *mut TrapFrame) {
    unsafe {
        asm!("msr daifset, #0xf; isb", options(nomem, nostack));
    }
    let nr = unsafe { (*_trap_frame).x[8] };
    let a0 = unsafe { (*_trap_frame).x[0] };
    let a1 = unsafe { (*_trap_frame).x[1] };
    let a2 = unsafe { (*_trap_frame).x[2] };
    let a3 = unsafe { (*_trap_frame).x[3] };
    let a4 = unsafe { (*_trap_frame).x[4] };
    let a5 = unsafe { (*_trap_frame).x[5] };

    let ret = crate::syscall::dispatch(nr, a0, a1, a2, a3, a4, a5);

    unsafe {
        (*_trap_frame).x[0] = ret;
    }
}

/// EL0 syscall path — same isolation as [`handle_el1_syscall`].
#[inline(never)]
unsafe fn handle_el0_syscall(_trap_frame: *mut TrapFrame, is_user: bool) {
    unsafe {
        asm!("msr daifset, #0xf; isb", options(nomem, nostack));
    }
    let nr = unsafe { (*_trap_frame).x[8] };
    let a0 = unsafe { (*_trap_frame).x[0] };
    let a1 = unsafe { (*_trap_frame).x[1] };
    let a2 = unsafe { (*_trap_frame).x[2] };
    let a3 = unsafe { (*_trap_frame).x[3] };
    let a4 = unsafe { (*_trap_frame).x[4] };
    let a5 = unsafe { (*_trap_frame).x[5] };

    let ret = crate::syscall::dispatch(nr, a0, a1, a2, a3, a4, a5);

    unsafe {
        (*_trap_frame).x[0] = ret;
    }
    if is_user {
        restore_el0_return_context_from_current_task();
    }
}

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

    // Check if current task is a user task (for EL0 exceptions)
    let is_user = crate::task::current_is_user();

    match kind {
        Ok(ExceptionKind::El1IrqSpx) => {
            crate::irq::dispatch();

            if crate::task::need_reschedule() {
                crate::task::clear_need_reschedule();
                crate::task::schedule();
            }
        }
        Ok(ExceptionKind::El0IrqA64) => {
            // Save user state before handling IRQ
            if is_user {
                crate::task::set_current_saved_context(elr, spsr, read_sp_el0());
            }

            crate::irq::dispatch();

            if crate::task::need_reschedule() {
                crate::task::clear_need_reschedule();
                crate::task::schedule();
            }
            if crate::task::current_is_user() {
                restore_el0_return_context_from_current_task();
            }
        }
        Ok(ExceptionKind::El0SyncA64) => {
            // Save user state before handling syscall or dumping an EL0 fault.
            if is_user {
                crate::task::set_current_saved_context(elr, spsr, read_sp_el0());
            }

            let esr = read_esr();
            let ec = (esr >> 26) & 0x3F;
            if ec == 0x15 {
                // Syscall handling is isolated in `handle_el0_syscall` so the trap-frame pointer
                // is not kept only in a spill slot of this large function (see module comment on
                // `handle_el1_syscall`).
                unsafe {
                    handle_el0_syscall(_trap_frame, is_user);
                }
            } else {
                dump_exception(kind.unwrap(), elr, spsr);
                loop {
                    core::hint::spin_loop();
                }
            }
        }
        Ok(ExceptionKind::El1SyncSpx) => {
            let esr = read_esr();
            let ec = (esr >> 26) & 0x3F;
            if ec == 0x15 {
                unsafe {
                    handle_el1_syscall(_trap_frame);
                }
            } else {
                dump_exception(kind.unwrap(), elr, spsr);
                loop {
                    core::hint::spin_loop();
                }
            }
        }
        Ok(ExceptionKind::El1SErrorSpx) | Ok(ExceptionKind::El0SErrorA64) => {
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

fn read_sp_el0() -> u64 {
    let sp: u64;
    unsafe { asm!("mrs {}, sp_el0", out(reg) sp) };
    sp
}

fn restore_el0_return_context_from_current_task() {
    let elr = crate::task::current_saved_elr();
    let spsr = crate::task::current_saved_spsr();
    let sp_el0 = crate::task::current_saved_sp_el0();
    unsafe {
        asm!("msr elr_el1, {}", in(reg) elr, options(nostack, preserves_flags));
        asm!("msr spsr_el1, {}", in(reg) spsr, options(nostack, preserves_flags));
        asm!("msr sp_el0, {}", in(reg) sp_el0, options(nostack, preserves_flags));
        asm!("isb", options(nostack, preserves_flags));
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
