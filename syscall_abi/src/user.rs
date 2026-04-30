// SPDX-License-Identifier: MIT OR Apache-2.0

//! EL0 / EL1 **thin syscall stubs** (`SVC #0`) for Hawthorn on **AArch64**.
//!
//! Use only on `target_arch = "aarch64"` bare-metal or compatible; this module is
//! **empty** on other hosts (see `hawthorn_syscall_abi::user` in `lib.rs`).
//!
//! All functions follow **DRAFT-1.0** in the crate root: `x8` = syscall number,
//! `x0`–`x5` = arguments, `x0` = return (negative errno on error).

use crate::{
    SYS_ABI_INFO, SYS_ENDPOINT_CALL, SYS_ENDPOINT_CREATE, SYS_ENDPOINT_DESTROY, SYS_ENDPOINT_RECV,
    SYS_ENDPOINT_REPLY, SYS_EXIT, SYS_GETPID, SYS_SLEEP, SYS_WRITE, SYS_YIELD,
};
use core::arch::asm;

/// Raw syscall; **`nr`** must be a valid Hawthorn syscall number.
///
/// # Safety
///
/// Arguments must satisfy the ABI for `nr` (valid pointers where required, etc.).
#[inline]
pub unsafe fn raw_syscall6(
    nr: u64,
    mut a0: u64,
    a1: u64,
    a2: u64,
    a3: u64,
    a4: u64,
    a5: u64,
) -> u64 {
    unsafe {
        asm!(
            "svc #0",
            inout("x0") a0,
            in("x1") a1,
            in("x2") a2,
            in("x3") a3,
            in("x4") a4,
            in("x5") a5,
            in("x8") nr,
            lateout("x16") _,
            lateout("x17") _,
            options(nostack),
        );
    }
    a0
}

#[inline]
pub fn sys_abi_info() -> u64 {
    unsafe { raw_syscall6(SYS_ABI_INFO, 0, 0, 0, 0, 0, 0) }
}

#[inline]
pub fn sys_getpid() -> u64 {
    unsafe { raw_syscall6(SYS_GETPID, 0, 0, 0, 0, 0, 0) }
}

#[inline]
pub fn sys_yield() -> u64 {
    unsafe { raw_syscall6(SYS_YIELD, 0, 0, 0, 0, 0, 0) }
}

#[inline]
pub fn sys_sleep(ms: u64) -> u64 {
    unsafe { raw_syscall6(SYS_SLEEP, ms, 0, 0, 0, 0, 0) }
}

#[inline]
pub fn sys_exit(code: u64) -> ! {
    unsafe {
        raw_syscall6(SYS_EXIT, code, 0, 0, 0, 0, 0);
    }
    loop {
        unsafe {
            asm!("wfe", options(nomem, nostack));
        }
    }
}

#[inline]
pub fn sys_write(fd: u64, buf: *const u8, len: u64) -> u64 {
    unsafe { raw_syscall6(SYS_WRITE, fd, buf as u64, len, 0, 0, 0) }
}

#[inline]
pub fn sys_endpoint_create() -> u64 {
    unsafe { raw_syscall6(SYS_ENDPOINT_CREATE, 0, 0, 0, 0, 0, 0) }
}

#[inline]
pub fn sys_endpoint_destroy(id: u64) -> u64 {
    unsafe { raw_syscall6(SYS_ENDPOINT_DESTROY, id, 0, 0, 0, 0, 0) }
}

/// `msg` is masked with [`crate::ENDPOINT_INLINE_REQ_MASK`] on the server; successful return is the full **`u64`** reply from `reply`.
#[inline]
pub fn sys_endpoint_call(id: u64, msg: u64) -> u64 {
    unsafe { raw_syscall6(SYS_ENDPOINT_CALL, id, msg, 0, 0, 0, 0) }
}

#[inline]
pub fn sys_endpoint_recv(id: u64) -> u64 {
    unsafe { raw_syscall6(SYS_ENDPOINT_RECV, id, 0, 0, 0, 0, 0) }
}

/// Full **`u64`** reply word delivered to the caller’s `call` return (request path remains 32-bit in Phase 1).
#[inline]
pub fn sys_endpoint_reply(id: u64, client_id: u64, msg: u64) -> u64 {
    unsafe { raw_syscall6(SYS_ENDPOINT_REPLY, id, client_id, msg, 0, 0, 0) }
}
