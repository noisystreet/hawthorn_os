// SPDX-License-Identifier: MIT OR Apache-2.0

//! Fixed low virtual-memory layout for the QEMU `virt` EL0 MVP.
//!
//! See [KERNEL.md](../../docs/内核.md) §3.8. Syscall copy-in validation
//! (`syscall` module) only allows user buffers that lie **entirely** in the
//! code window or the stack window — not the unmapped hole between them.

/// Userspace mapping granularity (matches AArch64 4KiB user pages in this kernel).
pub const PAGE_SIZE: usize = 4096;

/// User text starts here (`create_user` maps the embedded `.user_program` copy).
pub const USER_CODE_BASE: usize = 0x1000;

/// Bytes of user code region accepted by syscall **pointer** validation (MVP: one page).
/// If the embedded program grows beyond this, raise the constant and map more pages in
/// [`crate::task::create_user`].
pub const USER_CODE_BYTES_MAX: usize = PAGE_SIZE;

/// User stack grows down toward [`USER_STACK_BOTTOM`]; `SP_EL0` starts at `USER_STACK_TOP`.
pub const USER_STACK_TOP: usize = 0x8000;

/// User stack size in the MVP (one page; separate from the kernel thread [`crate::task`] stack).
pub const USER_STACK_BYTES: usize = PAGE_SIZE;

/// Lowest **mapped** user stack address (inclusive).
pub const USER_STACK_BOTTOM: usize = USER_STACK_TOP - USER_STACK_BYTES;

/// Upper bound of the unmapped guard region directly below the stack page (`..USER_STACK_BOTTOM`).
pub const USER_GUARD_HIGH: usize = USER_STACK_BOTTOM;

/// `true` if `[start, start + len)` lies wholly inside the code window or wholly inside the stack window.
pub fn user_range_valid(start: usize, len: usize) -> bool {
    if len == 0 {
        return true;
    }
    let Some(end) = start.checked_add(len) else {
        return false;
    };
    if start >= end {
        return false;
    }
    let code_end = USER_CODE_BASE.saturating_add(USER_CODE_BYTES_MAX);
    let wholly_in_code = start >= USER_CODE_BASE && end <= code_end;
    let wholly_in_stack = start >= USER_STACK_BOTTOM && end <= USER_STACK_TOP;
    wholly_in_code || wholly_in_stack
}
