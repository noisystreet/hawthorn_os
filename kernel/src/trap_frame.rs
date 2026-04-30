// SPDX-License-Identifier: MIT OR Apache-2.0

//! AArch64 [`TrapFrame`] layout used by the vector stubs in [`crate::trap`].
//!
//! This module is built on the host (`cargo test`) so layout and offset
//! invariants are checked without pulling in `global_asm!`.
//!
//! **Contract:** `TRAP_FRAME_*` constants must stay in sync with
//! `kernel/src/trap.rs` (`sub sp, sp, #272`, stores/loads at `[sp, #256]`
//! and `[sp, #264]`). See `docs/TRAP.md` §3.

use core::mem::{align_of, offset_of, size_of};

/// Saved general-purpose register state on exception entry.
///
/// Layout matches the vector stubs: 31 GPRs (x0–x30) + `sp_el0`, then
/// `elr_el1` / `spsr_el1` captured on entry so `eret` can restore the correct
/// return address after another task took an exception (`ELR_EL1` is not
/// per-thread).
#[repr(C)]
pub struct TrapFrame {
    pub x: [u64; 31],
    pub sp_el0: u64,
    pub elr_el1: u64,
    pub spsr_el1: u64,
}

/// Bytes reserved on the stack per exception entry (`sub sp, sp, #N` in vector stubs).
pub const TRAP_FRAME_SIZE: usize = size_of::<TrapFrame>();

/// Byte offset of `elr_el1` from the trap frame base (`str x3, [sp, #256]`).
pub const TRAP_FRAME_OFFSET_ELR_EL1: usize = offset_of!(TrapFrame, elr_el1);

/// Byte offset of `spsr_el1` from the trap frame base (`str x4, [sp, #264]`).
pub const TRAP_FRAME_OFFSET_SPSR_EL1: usize = offset_of!(TrapFrame, spsr_el1);

const _: () = assert!(TRAP_FRAME_SIZE == 272);
const _: () = assert!(TRAP_FRAME_OFFSET_ELR_EL1 == 256);
const _: () = assert!(TRAP_FRAME_OFFSET_SPSR_EL1 == 264);
const _: () = assert!(offset_of!(TrapFrame, x) == 0);
const _: () = assert!(offset_of!(TrapFrame, sp_el0) == 31 * 8);
const _: () = assert!(align_of::<TrapFrame>() == 8);

#[cfg(test)]
mod tests {
    use super::{
        TrapFrame, TRAP_FRAME_OFFSET_ELR_EL1, TRAP_FRAME_OFFSET_SPSR_EL1, TRAP_FRAME_SIZE,
    };
    use core::mem::{align_of, offset_of, size_of};

    /// Regression: `ELR_EL1` / `SPSR_EL1` must live in the frame so a task that
    /// blocked inside a syscall does not `eret` with another task's ELR after
    /// a context switch. See `docs/TRAP.md` §3.4.
    #[test]
    fn trap_frame_includes_elr_spsr_for_blocking_syscall_eret() {
        assert_trap_frame_size_and_align_matches_asm();
        assert_trap_frame_field_offsets_match_vector_stub();
    }

    fn assert_trap_frame_size_and_align_matches_asm() {
        assert_eq!(size_of::<TrapFrame>(), 272);
        assert_eq!(TRAP_FRAME_SIZE, 272);
        assert_eq!(align_of::<TrapFrame>(), 8);
    }

    fn assert_trap_frame_field_offsets_match_vector_stub() {
        assert_eq!(offset_of!(TrapFrame, x), 0);
        assert_eq!(offset_of!(TrapFrame, sp_el0), 31 * 8);
        assert_eq!(offset_of!(TrapFrame, elr_el1), TRAP_FRAME_OFFSET_ELR_EL1);
        assert_eq!(offset_of!(TrapFrame, spsr_el1), TRAP_FRAME_OFFSET_SPSR_EL1);
        assert_eq!(TRAP_FRAME_OFFSET_ELR_EL1, 256);
        assert_eq!(TRAP_FRAME_OFFSET_SPSR_EL1, 264);
        assert_eq!(
            offset_of!(TrapFrame, sp_el0) + size_of::<u64>(),
            TRAP_FRAME_OFFSET_ELR_EL1,
            "sp_el0 must be immediately before elr_el1 (asm relies on +248, +256)"
        );
    }

    #[test]
    fn trap_frame_asm_slots_alias_struct_fields() {
        let mut tf = TrapFrame {
            x: [0u64; 31],
            sp_el0: 0,
            elr_el1: 0,
            spsr_el1: 0,
        };
        let base = core::ptr::addr_of_mut!(tf).cast::<u8>();

        unsafe {
            base.add(256)
                .cast::<u64>()
                .write_unaligned(0x11_22_33_44_55_66_77_88);
            base.add(264)
                .cast::<u64>()
                .write_unaligned(0xaa_bb_cc_dd_ee_ff_00_11);
        }

        assert_eq!(tf.elr_el1, 0x11_22_33_44_55_66_77_88);
        assert_eq!(tf.spsr_el1, 0xaa_bb_cc_dd_ee_ff_00_11);
    }
}
