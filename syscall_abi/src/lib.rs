// SPDX-License-Identifier: MIT OR Apache-2.0

//! Hawthorn syscall ABI — constants and types shared by kernel and user space.
//!
//! ## Register convention (AArch64)
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
//! ## Syscall numbers
//!
//! Numbers 0–63 are reserved for the kernel core. 64–255 are reserved
//! for future expansion. 256+ are dynamically assigned.

#![cfg_attr(not(test), no_std)]

pub const SYS_WRITE: u64 = 0;
pub const SYS_READ: u64 = 1;
pub const SYS_YIELD: u64 = 2;
pub const SYS_GETPID: u64 = 3;
pub const SYS_EXIT: u64 = 4;
pub const SYS_SLEEP: u64 = 5;

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

pub const ABI_VERSION: u64 = 1;
