// SPDX-License-Identifier: MIT OR Apache-2.0

//! Hawthorn syscall ABI — constants and types shared by kernel and user space.
//!
//! ## Stability label (human-facing)
//!
//! The book uses **DRAFT-1.0** together with numeric [`ABI_VERSION`]. When the
//! interface is promoted, docs will add a **STABLE-x** line; bump [`ABI_VERSION`]
//! on any **breaking** change to syscall numbers, argument layouts, or error rules.
//!
//! ## Register convention (AArch64)
//!
//! From **EL0** or **EL1**: issue **`SVC #0`**; the syscall number is passed in
//! **`x8`**, arguments in **`x0`–`x5`** (up to six words). Return value in **`x0`**.
//! (Immediate value is unused for dispatch, matching the common Linux-aarch64 pattern.)
//!
//! | Register | Role                          |
//! |----------|-------------------------------|
//! | `x8`     | Syscall number                |
//! | `x0–x5`  | Arguments (up to 6)           |
//! | `x0`     | Return value / error code     |
//!
//! On return, `x0` holds the result. If the value is in the range
//! `-(MAX_ERRNO)..0` (i.e. `-4095..-1`), it is an error code and
//! `x0` should be interpreted as a negative `Errno`.
//!
//! ## Syscall number space
//!
//! | Range | Meaning |
//! |-------|---------|
//! | `0..=SYSCALL_NR_CORE_MAX` (63) | **Kernel core** — fixed table in `hawthorn_kernel::syscall` |
//! | `64..=255` | **Reserved** for future fixed syscalls |
//! | `256..` | **Reserved** for dynamic or out-of-band assignment (policy TBD) |
//!
//! Any unimplemented number in the core table returns [`Errno::ENOSYS`].
//!
//! ## `SYS_ABI_INFO` return value
//!
//! Low **32** bits: [`ABI_VERSION`]. High **32** bits: OR of [`ABI_CAP_*`] flags.
//! This is a **non-negative** word (not an [`Errno`] encoding).

#![cfg_attr(not(test), no_std)]

/// Human-readable ABI draft label (keep in sync with book `docs/系统调用ABI.md`).
pub const ABI_DRAFT_LABEL: &str = "DRAFT-1.0";

/// Bump this when syscall layout or semantics change incompatibly; keep book in sync.
pub const ABI_VERSION: u64 = 1;

/// Last syscall **number** (`inclusive`) reserved for the kernel core fixed table.
pub const SYSCALL_NR_CORE_MAX: u64 = 63;

/// Dispatch table length used by the kernel (`nr` must be `<` this for fast lookup).
pub const SYSCALL_DISPATCH_TABLE_LEN: usize = (SYSCALL_NR_CORE_MAX as usize) + 1;

pub const SYS_WRITE: u64 = 0;
pub const SYS_READ: u64 = 1;
pub const SYS_YIELD: u64 = 2;
pub const SYS_GETPID: u64 = 3;
pub const SYS_EXIT: u64 = 4;
pub const SYS_SLEEP: u64 = 5;
pub const SYS_ENDPOINT_CREATE: u64 = 6;
pub const SYS_ENDPOINT_DESTROY: u64 = 7;
pub const SYS_ENDPOINT_CALL: u64 = 8;
pub const SYS_ENDPOINT_RECV: u64 = 9;
pub const SYS_ENDPOINT_REPLY: u64 = 10;
/// [`SYS_ABI_INFO`] packs [`ABI_VERSION`] (low 32) and capability bits (high 32); see [`abi_info_word`].
pub const SYS_ABI_INFO: u64 = 11;

/// Maximum number of scalar arguments passed in registers for this ABI (`x0`–`x5`).
pub const SYSCALL_MAX_ARGS: usize = 6;

pub const MAX_ERRNO: u64 = 4095;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i64)]
pub enum Errno {
    Ok = 0,
    EPERM = 1,
    ENOENT = 2,
    ESRCH = 3,
    EINTR = 4,
    EIO = 5,
    ENXIO = 6,
    E2BIG = 7,
    ENOEXEC = 8,
    EBADF = 9,
    ECHILD = 10,
    EAGAIN = 11,
    ENOMEM = 12,
    EACCES = 13,
    EFAULT = 14,
    EINVAL = 22,
    ENOSYS = 38,
}

impl Errno {
    /// POSIX-style errno number (positive), matching the enum discriminant.
    pub fn as_i64(self) -> i64 {
        self as i64
    }

    /// Encodes `x0` return value for syscall handlers: `Ok` → `0`, errors → **`-errno`** (two's complement).
    pub fn as_u64(self) -> u64 {
        let code = self as i64;
        if code == 0 {
            0
        } else {
            (-code) as u64
        }
    }

    pub fn is_ok(self) -> bool {
        self == Errno::Ok
    }
}

pub fn is_error(ret: u64) -> bool {
    let signed = ret as i64;
    signed < 0 && -signed <= MAX_ERRNO as i64
}

pub fn errno_from_ret(ret: u64) -> Option<Errno> {
    if !is_error(ret) {
        return None;
    }
    let code = -(ret as i64);
    match code {
        0 => Some(Errno::Ok),
        1 => Some(Errno::EPERM),
        2 => Some(Errno::ENOENT),
        3 => Some(Errno::ESRCH),
        4 => Some(Errno::EINTR),
        5 => Some(Errno::EIO),
        6 => Some(Errno::ENXIO),
        7 => Some(Errno::E2BIG),
        8 => Some(Errno::ENOEXEC),
        9 => Some(Errno::EBADF),
        10 => Some(Errno::ECHILD),
        11 => Some(Errno::EAGAIN),
        12 => Some(Errno::ENOMEM),
        13 => Some(Errno::EACCES),
        14 => Some(Errno::EFAULT),
        22 => Some(Errno::EINVAL),
        38 => Some(Errno::ENOSYS),
        _ => None,
    }
}

/// Kernel exposes per-task EL0 page tables + user-pointer validation for the low fixed window.
pub const ABI_CAP_EL0_USER_AS: u64 = 1 << 0;

/// Return value for [`SYS_ABI_INFO`]: version in low half, [`ABI_CAP_*`] mask in high half.
#[inline]
pub fn abi_info_word() -> u64 {
    ABI_VERSION | (ABI_CAP_EL0_USER_AS << 32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abi_info_syscall_number_reserved() {
        assert_eq!(SYS_ABI_INFO, 11);
    }

    #[test]
    fn dispatch_table_len_matches_core_range() {
        assert_eq!(SYSCALL_DISPATCH_TABLE_LEN, SYSCALL_NR_CORE_MAX as usize + 1);
    }

    #[test]
    fn abi_draft_label_non_empty() {
        assert!(!ABI_DRAFT_LABEL.is_empty());
    }

    #[test]
    fn abi_info_word_packing() {
        let w = abi_info_word();
        assert_eq!(w & 0xFFFF_FFFF, ABI_VERSION);
        assert_eq!(w >> 32, ABI_CAP_EL0_USER_AS);
    }

    #[test]
    fn errno_as_u64_encodes_negative_for_errors() {
        assert_eq!(Errno::Ok.as_u64(), 0);
        assert_eq!(Errno::EPERM.as_u64(), (-1i64) as u64);
        assert_eq!(Errno::EINVAL.as_u64(), (-22i64) as u64);
        assert_eq!(Errno::ENOSYS.as_u64(), (-38i64) as u64);
    }

    #[test]
    fn is_error_checks_boundaries() {
        assert!(is_error((-1i64) as u64));
        assert!(is_error((-4095i64) as u64));
        assert!(!is_error((-4096i64) as u64));
        assert!(!is_error(0));
        assert!(!is_error(1));
    }

    #[test]
    fn errno_from_ret_roundtrip_for_known_codes() {
        let errs = [
            Errno::EPERM,
            Errno::ENOENT,
            Errno::ENOMEM,
            Errno::EINVAL,
            Errno::ENOSYS,
        ];
        for err in errs {
            let ret = err.as_u64();
            assert_eq!(errno_from_ret(ret), Some(err));
        }
        assert_eq!(errno_from_ret((-999i64) as u64), None);
        assert_eq!(errno_from_ret(0), None);
    }
}
