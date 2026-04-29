// SPDX-License-Identifier: MIT OR Apache-2.0

//! Syscall dispatch and handler implementations.
//!
//! Entry point: [`dispatch`] takes the syscall number (x8) and up to 6
//! arguments (x0–x5), dispatches to the corresponding handler, and
//! returns the result in x0.
//!
//! Register convention (matching `hawthorn_syscall_abi`):
//! - x8  = syscall number
//! - x0–x5 = arguments
//! - x0  = return value (negative = Errno)

use hawthorn_syscall_abi::{Errno, SYS_EXIT, SYS_GETPID, SYS_SLEEP, SYS_WRITE, SYS_YIELD};

const MAX_SYSCALL: u64 = 64;

type SyscallHandler = fn(u64, u64, u64, u64, u64, u64) -> u64;

#[allow(static_mut_refs)]
static mut SYSCALL_TABLE: [Option<SyscallHandler>; MAX_SYSCALL as usize] =
    [None; MAX_SYSCALL as usize];

pub fn init() {
    unsafe {
        SYSCALL_TABLE[SYS_WRITE as usize] = Some(sys_write);
        SYSCALL_TABLE[SYS_YIELD as usize] = Some(sys_yield);
        SYSCALL_TABLE[SYS_GETPID as usize] = Some(sys_getpid);
        SYSCALL_TABLE[SYS_EXIT as usize] = Some(sys_exit);
        SYSCALL_TABLE[SYS_SLEEP as usize] = Some(sys_sleep);
    }
}

pub fn dispatch(nr: u64, a0: u64, a1: u64, a2: u64, a3: u64, a4: u64, a5: u64) -> u64 {
    if nr >= MAX_SYSCALL {
        return Errno::ENOSYS.as_u64();
    }

    let handler = unsafe { SYSCALL_TABLE[nr as usize] };

    match handler {
        Some(h) => h(a0, a1, a2, a3, a4, a5),
        None => Errno::ENOSYS.as_u64(),
    }
}

fn sys_write(fd: u64, buf: u64, len: u64, _a3: u64, _a4: u64, _a5: u64) -> u64 {
    if fd != 1 {
        return Errno::EBADF.as_u64();
    }

    let ptr = buf as *const u8;
    let count = len as usize;

    if ptr.is_null() || count == 0 {
        return Errno::EINVAL.as_u64();
    }

    unsafe {
        let slice = core::slice::from_raw_parts(ptr, count);
        crate::boot_qemu_virt::pl011_write_bytes(slice);
    }

    len
}

fn sys_yield(_a0: u64, _a1: u64, _a2: u64, _a3: u64, _a4: u64, _a5: u64) -> u64 {
    crate::task::yield_now();
    0
}

fn sys_getpid(_a0: u64, _a1: u64, _a2: u64, _a3: u64, _a4: u64, _a5: u64) -> u64 {
    crate::task::current_id().0 as u64
}

fn sys_exit(code: u64, _a1: u64, _a2: u64, _a3: u64, _a4: u64, _a5: u64) -> u64 {
    crate::println!(
        "[syscall] task {} exit({})",
        crate::task::current_id().0,
        code
    );
    crate::task::exit_current();
}

fn sys_sleep(ms: u64, _a1: u64, _a2: u64, _a3: u64, _a4: u64, _a5: u64) -> u64 {
    crate::task::sleep(ms);
    0
}
