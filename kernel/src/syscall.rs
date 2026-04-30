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

use hawthorn_syscall_abi::{
    Errno, SYSCALL_DISPATCH_TABLE_LEN, SYS_ABI_INFO, SYS_ENDPOINT_CALL, SYS_ENDPOINT_CREATE,
    SYS_ENDPOINT_DESTROY, SYS_ENDPOINT_RECV, SYS_ENDPOINT_REPLY, SYS_EXIT, SYS_GETPID, SYS_READ,
    SYS_SLEEP, SYS_WRITE, SYS_YIELD,
};
const WRITE_CHUNK_SIZE: usize = 256;
/// Identity RAM window (see `kernel/src/mm.rs` / `frame_alloc`).
const KERNEL_RAM_START: usize = 0x4000_0000;
const KERNEL_RAM_END_EXCL: usize = 0x4800_0000;

type SyscallHandler = fn(u64, u64, u64, u64, u64, u64) -> u64;

#[allow(static_mut_refs)]
static mut SYSCALL_TABLE: [Option<SyscallHandler>; SYSCALL_DISPATCH_TABLE_LEN] =
    [None; SYSCALL_DISPATCH_TABLE_LEN];

pub fn init() {
    unsafe {
        SYSCALL_TABLE[SYS_WRITE as usize] = Some(sys_write);
        SYSCALL_TABLE[SYS_READ as usize] = Some(sys_read);
        SYSCALL_TABLE[SYS_YIELD as usize] = Some(sys_yield);
        SYSCALL_TABLE[SYS_GETPID as usize] = Some(sys_getpid);
        SYSCALL_TABLE[SYS_EXIT as usize] = Some(sys_exit);
        SYSCALL_TABLE[SYS_SLEEP as usize] = Some(sys_sleep);
        SYSCALL_TABLE[SYS_ENDPOINT_CREATE as usize] = Some(sys_endpoint_create);
        SYSCALL_TABLE[SYS_ENDPOINT_DESTROY as usize] = Some(sys_endpoint_destroy);
        SYSCALL_TABLE[SYS_ENDPOINT_CALL as usize] = Some(sys_endpoint_call);
        SYSCALL_TABLE[SYS_ENDPOINT_RECV as usize] = Some(sys_endpoint_recv);
        SYSCALL_TABLE[SYS_ENDPOINT_REPLY as usize] = Some(sys_endpoint_reply);
        SYSCALL_TABLE[SYS_ABI_INFO as usize] = Some(sys_abi_info);
    }
}

pub fn dispatch(nr: u64, a0: u64, a1: u64, a2: u64, a3: u64, a4: u64, a5: u64) -> u64 {
    if nr >= SYSCALL_DISPATCH_TABLE_LEN as u64 {
        return Errno::ENOSYS.as_u64();
    }

    let handler = unsafe { SYSCALL_TABLE[nr as usize] };

    match handler {
        Some(h) => h(a0, a1, a2, a3, a4, a5),
        None => Errno::ENOSYS.as_u64(),
    }
}

fn sys_read(_fd: u64, _buf: u64, _len: u64, _a3: u64, _a4: u64, _a5: u64) -> u64 {
    Errno::ENOSYS.as_u64()
}

fn sys_write(fd: u64, buf: u64, len: u64, _a3: u64, _a4: u64, _a5: u64) -> u64 {
    if fd != 1 {
        return Errno::EBADF.as_u64();
    }

    let count = len as usize;

    if buf == 0 || count == 0 {
        return Errno::EINVAL.as_u64();
    }

    let mut copied = 0usize;
    let mut chunk = [0u8; WRITE_CHUNK_SIZE];

    while copied < count {
        let n = core::cmp::min(WRITE_CHUNK_SIZE, count - copied);
        let user_src = (buf as usize).saturating_add(copied);

        if copy_write_payload(user_src, &mut chunk[..n]).is_err() {
            return Errno::EFAULT.as_u64();
        }

        // SAFETY: PL011 UART has been initialized during early boot.
        unsafe {
            crate::boot_qemu_virt::pl011_write_bytes(&chunk[..n]);
        }
        copied += n;
    }

    len
}

fn copy_write_payload(src: usize, dst: &mut [u8]) -> Result<(), Errno> {
    if dst.is_empty() {
        return Ok(());
    }

    if crate::task::current_is_user() {
        if !user_range_valid(src, dst.len()) {
            return Err(Errno::EFAULT);
        }
        // SAFETY: `user_range_valid` bounds the EL0 buffer; `dst` is a kernel buffer.
        unsafe {
            for (i, out) in dst.iter_mut().enumerate() {
                *out = core::ptr::read_volatile((src + i) as *const u8);
            }
        }
    } else {
        if !kernel_buffer_range_ok(src, dst.len()) {
            return Err(Errno::EFAULT);
        }
        // SAFETY: `kernel_buffer_range_ok` ensures the source lies in mapped RAM.
        unsafe {
            core::ptr::copy_nonoverlapping(src as *const u8, dst.as_mut_ptr(), dst.len());
        }
    }
    Ok(())
}

fn kernel_buffer_range_ok(start: usize, len: usize) -> bool {
    if len == 0 {
        return true;
    }
    let Some(end) = start.checked_add(len) else {
        return false;
    };
    start >= KERNEL_RAM_START && end <= KERNEL_RAM_END_EXCL && start < end
}

fn user_range_valid(start: usize, len: usize) -> bool {
    crate::user_layout::user_range_valid(start, len)
}

fn sys_abi_info(_a0: u64, _a1: u64, _a2: u64, _a3: u64, _a4: u64, _a5: u64) -> u64 {
    hawthorn_syscall_abi::abi_info_word()
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

fn sys_endpoint_create(_a0: u64, _a1: u64, _a2: u64, _a3: u64, _a4: u64, _a5: u64) -> u64 {
    match crate::endpoint::create() {
        Some(id) => u64::from(id),
        None => Errno::ENOMEM.as_u64(),
    }
}

fn sys_endpoint_destroy(id: u64, _a1: u64, _a2: u64, _a3: u64, _a4: u64, _a5: u64) -> u64 {
    match crate::endpoint::destroy(id) {
        Ok(()) => 0,
        Err(e) => e.as_u64(),
    }
}

fn sys_endpoint_call(id: u64, msg: u64, _a2: u64, _a3: u64, _a4: u64, _a5: u64) -> u64 {
    use crate::endpoint::CallResult;
    let caller = crate::task::current_id().0;
    crate::println!(
        "[syscall] endpoint_call(ep={}, msg={}) caller={}",
        id,
        msg,
        caller
    );
    match crate::endpoint::call(id, msg) {
        CallResult::Reply(v) => {
            crate::println!("[syscall] endpoint_call => Reply({})", v as i64);
            v
        }
        CallResult::Blocked => {
            crate::println!(
                "[syscall] endpoint_call => Blocked, blocking task {}",
                caller
            );
            let task_idx = crate::task::current_id().0 as usize;
            crate::task::block();
            crate::println!(
                "[syscall] endpoint_call task {} resumed after block",
                caller
            );
            // Do not `schedule()` here: `block()` already switched away. A second
            // schedule before `take_reply_value` can reorder peers and break the
            // recv → reply → wake(caller) handshake (call may observe garbage).
            let reply = crate::endpoint::take_reply_value(task_idx);
            crate::println!(
                "[syscall] endpoint_call task {} take_reply_value={}",
                caller,
                reply as i64
            );
            reply
        }
    }
}

fn sys_endpoint_recv(id: u64, _a1: u64, _a2: u64, _a3: u64, _a4: u64, _a5: u64) -> u64 {
    use crate::endpoint::RecvResult;
    let caller = crate::task::current_id().0;
    crate::println!("[syscall] endpoint_recv(ep={}) caller={}", id, caller);
    match crate::endpoint::recv(id) {
        RecvResult::Message(v) => {
            crate::println!("[syscall] endpoint_recv => Message({:#x})", v);
            v
        }
        RecvResult::Blocked => {
            crate::println!(
                "[syscall] endpoint_recv => Blocked, blocking task {}",
                caller
            );
            crate::task::block();
            crate::println!(
                "[syscall] endpoint_recv task {} resumed after block",
                caller
            );
            // Do not `schedule()` here: `block()` already switched away. A second
            // schedule before the recv retry can reorder tasks and corrupt the
            // syscall return handshake (another task's trap frame / return path).
            match crate::endpoint::recv(id) {
                RecvResult::Message(v) => {
                    crate::println!("[syscall] endpoint_recv retry => Message({:#x})", v);
                    v
                }
                RecvResult::Blocked => {
                    crate::println!("[syscall] endpoint_recv retry => Blocked again, EAGAIN");
                    hawthorn_syscall_abi::Errno::EAGAIN.as_u64()
                }
            }
        }
    }
}

fn sys_endpoint_reply(id: u64, client_id: u64, msg: u64, _a3: u64, _a4: u64, _a5: u64) -> u64 {
    match crate::endpoint::reply(id, client_id, msg) {
        Ok(()) => 0,
        Err(e) => e.as_u64(),
    }
}
