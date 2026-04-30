// SPDX-License-Identifier: MIT OR Apache-2.0

//! Endpoint object table for IPC with blocking rendezvous.
//!
//! This module provides endpoint lifecycle and a blocking rendezvous path:
//! **call → recv → reply**.
//!
//! When the peer is not ready, `call` / `recv` **block** the calling task
//! instead of returning `EAGAIN`. The blocked task is woken when the peer
//! performs the matching operation:
//!
//! - `call(ep, msg)` when no server is `recv`-ing → caller blocks until
//!   a server does `recv` (which consumes the message and returns) **and**
//!   then `reply` (which delivers the reply and unblocks the caller).
//! - `recv(ep)` when no client has `call`-ed → server blocks until a
//!   client calls; the `call` message is consumed and the server unblocks.
//! - `reply(ep, client, msg)` → stores the reply value and unblocks the
//!   waiting client task.

use hawthorn_syscall_abi::{endpoint_recv_pack, Errno, ENDPOINT_INLINE_REQ_MASK};

#[cfg(all(target_arch = "aarch64", target_os = "none"))]
use crate::task::TaskId;

#[cfg(not(all(target_arch = "aarch64", target_os = "none")))]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct TaskId(pub u16);

const MAX_ENDPOINTS: usize = 16;
const MAX_TASK_SLOTS: usize = 16;
const INVALID_TASK_ID: u16 = u16::MAX;

/// UART trace output only exists in the bare-metal AArch64 kernel image (`console`);
/// host `cargo test` builds omit `console`/`task`, so this expands to nothing there.
macro_rules! ep_trace {
    ($($arg:tt)*) => {{
        #[cfg(all(target_arch = "aarch64", target_os = "none"))]
        $crate::println!($($arg)*);
    }};
}

#[cfg(all(target_arch = "aarch64", target_os = "none"))]
#[inline]
fn endpoint_task_unblock(id: TaskId) {
    crate::task::unblock(id);
}

#[cfg(not(all(target_arch = "aarch64", target_os = "none")))]
#[inline]
fn endpoint_task_unblock(_id: TaskId) {}

#[derive(Clone, Copy)]
struct Endpoint {
    in_use: bool,
    owner: TaskId,
    has_pending_call: bool,
    pending_client: TaskId,
    pending_msg: u64,
    blocked_receiver: TaskId,
}

impl Endpoint {
    const EMPTY: Self = Self {
        in_use: false,
        owner: TaskId(0),
        has_pending_call: false,
        pending_client: TaskId(INVALID_TASK_ID),
        pending_msg: 0,
        blocked_receiver: TaskId(INVALID_TASK_ID),
    };
}

#[allow(static_mut_refs)]
static mut ENDPOINT_TABLE: [Endpoint; MAX_ENDPOINTS] = [Endpoint::EMPTY; MAX_ENDPOINTS];

#[allow(static_mut_refs)]
static mut CALLER_BLOCKED: [bool; MAX_TASK_SLOTS] = [false; MAX_TASK_SLOTS];
#[allow(static_mut_refs)]
static mut REPLY_VALUE: [u64; MAX_TASK_SLOTS] = [0; MAX_TASK_SLOTS];

pub fn init() {
    unsafe {
        let mut idx = 0usize;
        while idx < MAX_ENDPOINTS {
            ENDPOINT_TABLE[idx] = Endpoint::EMPTY;
            idx += 1;
        }
        for i in 0..MAX_TASK_SLOTS {
            CALLER_BLOCKED[i] = false;
            REPLY_VALUE[i] = 0;
        }
    }
}

pub fn create() -> Option<u16> {
    let owner = current_task_id();
    create_with_owner(owner)
}

pub fn destroy(id: u64) -> Result<(), Errno> {
    let caller = current_task_id();
    destroy_with_caller(id, caller)
}

/// Result of a `call` operation that may block.
///
/// `Blocked` means the caller has been registered as waiting and the
/// scheduler must block the task (call `task::block()` + `schedule()`).
/// The caller will be unblocked by `reply`.
pub enum CallResult {
    Reply(u64),
    Blocked,
}

pub fn call(id: u64, msg: u64) -> CallResult {
    let caller = current_task_id();
    call_with_caller(id, msg, caller)
}

/// Result of a `recv` operation that may block.
///
/// `Blocked` means no pending call was available; the receiver has been
/// registered on the endpoint and the scheduler must block the task.
/// The receiver will be unblocked when a `call` arrives.
pub enum RecvResult {
    Message(u64),
    Blocked,
}

pub fn recv(id: u64) -> RecvResult {
    let caller = current_task_id();
    recv_with_caller(id, caller)
}

pub fn reply(id: u64, client_id: u64, msg: u64) -> Result<(), Errno> {
    let caller = current_task_id();
    reply_with_caller(id, client_id, msg, caller)
}

fn create_with_owner(owner: TaskId) -> Option<u16> {
    unsafe {
        let mut idx = 0usize;
        while idx < MAX_ENDPOINTS {
            if !ENDPOINT_TABLE[idx].in_use {
                ENDPOINT_TABLE[idx].in_use = true;
                ENDPOINT_TABLE[idx].owner = owner;
                return Some(idx as u16);
            }
            idx += 1;
        }
    }
    None
}

fn destroy_with_caller(id: u64, caller: TaskId) -> Result<(), Errno> {
    let idx = id as usize;
    if idx >= MAX_ENDPOINTS {
        return Err(Errno::EINVAL);
    }

    unsafe {
        let ep = &mut ENDPOINT_TABLE[idx];
        if !ep.in_use {
            return Err(Errno::ENOENT);
        }
        if ep.owner != caller {
            return Err(Errno::EPERM);
        }

        if ep.blocked_receiver.0 != INVALID_TASK_ID {
            endpoint_task_unblock(ep.blocked_receiver);
            ep.blocked_receiver = TaskId(INVALID_TASK_ID);
        }

        *ep = Endpoint::EMPTY;
    }
    Ok(())
}

fn call_with_caller(id: u64, msg: u64, caller: TaskId) -> CallResult {
    let idx = id as usize;
    if idx >= MAX_ENDPOINTS {
        return CallResult::Reply(Errno::EINVAL.as_u64());
    }
    let caller_idx = caller.0 as usize;
    if caller_idx >= MAX_TASK_SLOTS {
        return CallResult::Reply(Errno::EINVAL.as_u64());
    }

    unsafe {
        let ep = &mut ENDPOINT_TABLE[idx];
        ep_trace!(
            "[endpoint] call ep={} caller={} owner={} has_pending={} blocked_rcv={}",
            idx,
            caller.0,
            ep.owner.0,
            ep.has_pending_call,
            ep.blocked_receiver.0
        );
        if !ep.in_use {
            return CallResult::Reply(Errno::ENOENT.as_u64());
        }
        if ep.owner == caller {
            return CallResult::Reply(Errno::EPERM.as_u64());
        }
        if ep.has_pending_call {
            return CallResult::Reply(Errno::EAGAIN.as_u64());
        }

        ep.has_pending_call = true;
        ep.pending_client = caller;
        ep.pending_msg = msg & ENDPOINT_INLINE_REQ_MASK;
        CALLER_BLOCKED[caller_idx] = true;

        if ep.blocked_receiver.0 != INVALID_TASK_ID {
            ep_trace!(
                "[endpoint] call unblocking receiver {}",
                ep.blocked_receiver.0
            );
            endpoint_task_unblock(ep.blocked_receiver);
            ep.blocked_receiver = TaskId(INVALID_TASK_ID);
        }

        ep_trace!(
            "[endpoint] call => Blocked (caller {} waiting for recv+reply)",
            caller.0
        );
        CallResult::Blocked
    }
}

fn recv_with_caller(id: u64, caller: TaskId) -> RecvResult {
    let idx = id as usize;
    if idx >= MAX_ENDPOINTS {
        return RecvResult::Message(Errno::EINVAL.as_u64());
    }

    unsafe {
        let ep = &mut ENDPOINT_TABLE[idx];
        ep_trace!(
            "[endpoint] recv ep={} caller={} owner={} has_pending={}",
            idx,
            caller.0,
            ep.owner.0,
            ep.has_pending_call
        );
        if !ep.in_use {
            return RecvResult::Message(Errno::ENOENT.as_u64());
        }
        if ep.owner != caller {
            return RecvResult::Message(Errno::EPERM.as_u64());
        }

        if ep.has_pending_call {
            let client = ep.pending_client.0 as u64;
            let msg = ep.pending_msg & ENDPOINT_INLINE_REQ_MASK;
            ep_trace!(
                "[endpoint] recv got pending call: client={} msg={}",
                client,
                msg
            );
            ep.has_pending_call = false;
            ep.pending_client = TaskId(INVALID_TASK_ID);
            ep.pending_msg = 0;
            return RecvResult::Message(endpoint_recv_pack(client, msg));
        }

        ep_trace!(
            "[endpoint] recv => Blocked (receiver {} waiting for call)",
            caller.0
        );
        ep.blocked_receiver = caller;
        RecvResult::Blocked
    }
}

fn reply_with_caller(id: u64, client_id: u64, msg: u64, caller: TaskId) -> Result<(), Errno> {
    let idx = id as usize;
    if idx >= MAX_ENDPOINTS {
        return Err(Errno::EINVAL);
    }
    let client_idx = client_id as usize;
    if client_idx >= MAX_TASK_SLOTS {
        return Err(Errno::EINVAL);
    }

    ep_trace!(
        "[endpoint] reply ep={} client={} msg={} caller={}",
        idx,
        client_id,
        msg,
        caller.0
    );

    unsafe {
        let ep = &mut ENDPOINT_TABLE[idx];
        if !ep.in_use {
            return Err(Errno::ENOENT);
        }
        if ep.owner != caller {
            return Err(Errno::EPERM);
        }

        REPLY_VALUE[client_idx] = msg;
        CALLER_BLOCKED[client_idx] = false;
        endpoint_task_unblock(TaskId(client_id as u16));
        ep_trace!("[endpoint] reply unblocked caller {}", client_id);
    }
    Ok(())
}

/// Check if a task is blocked inside `call` (waiting for `reply`).
///
/// Called from the syscall dispatcher after `call` returns `CallResult::Blocked`:
/// if true, the dispatcher must `task::block()` + `schedule()`.
pub fn is_caller_blocked(task_idx: usize) -> bool {
    if task_idx >= MAX_TASK_SLOTS {
        return false;
    }
    unsafe { CALLER_BLOCKED[task_idx] }
}

/// Retrieve the reply value for a caller that has been unblocked.
///
/// Called from the syscall dispatcher when a blocked `call` returns:
/// the `x0` return value should be the reply from the server.
pub fn take_reply_value(task_idx: usize) -> u64 {
    if task_idx >= MAX_TASK_SLOTS {
        return Errno::EINVAL.as_u64();
    }
    unsafe {
        let v = REPLY_VALUE[task_idx];
        REPLY_VALUE[task_idx] = 0;
        v
    }
}

#[cfg(all(target_arch = "aarch64", target_os = "none"))]
fn current_task_id() -> TaskId {
    crate::task::current_id()
}

#[cfg(not(all(target_arch = "aarch64", target_os = "none")))]
fn current_task_id() -> TaskId {
    TaskId(0)
}

#[cfg(test)]
impl RecvResult {
    fn unwrap_message(self) -> u64 {
        match self {
            RecvResult::Message(v) => v,
            RecvResult::Blocked => panic!("called unwrap_message on Blocked"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static TABLE_LOCK: Mutex<()> = Mutex::new(());

    fn with_table(f: impl FnOnce()) {
        let _guard = TABLE_LOCK.lock().unwrap();
        init();
        f();
    }

    #[test]
    fn create_allocates_until_full() {
        with_table(|| {
            for i in 0..MAX_ENDPOINTS {
                assert_eq!(create_with_owner(TaskId(1)), Some(i as u16));
            }
            assert_eq!(create_with_owner(TaskId(1)), None);
        });
    }

    #[test]
    fn destroy_rejects_invalid_and_missing_endpoint() {
        with_table(|| {
            assert_eq!(
                destroy_with_caller(MAX_ENDPOINTS as u64, TaskId(1)),
                Err(Errno::EINVAL)
            );
            assert_eq!(destroy_with_caller(0, TaskId(1)), Err(Errno::ENOENT));
        });
    }

    #[test]
    fn destroy_checks_owner_permission() {
        with_table(|| {
            let id = create_with_owner(TaskId(7)).unwrap();
            assert_eq!(destroy_with_caller(id as u64, TaskId(8)), Err(Errno::EPERM));
            assert_eq!(destroy_with_caller(id as u64, TaskId(7)), Ok(()));
            assert_eq!(
                destroy_with_caller(id as u64, TaskId(7)),
                Err(Errno::ENOENT)
            );
        });
    }

    #[test]
    fn call_blocks_and_recv_unblocks() {
        with_table(call_blocks_and_recv_unblocks_body);
    }

    fn call_blocks_and_recv_unblocks_body() {
        let endpoint = create_with_owner(TaskId(1)).unwrap();
        assert_call_from_task2_blocks(endpoint);
        assert_recv_from_owner_returns_packed_client_and_msg(endpoint);
        assert_reply_from_owner_unblocks_task2(endpoint);
    }

    fn assert_call_from_task2_blocks(endpoint: u16) {
        assert!(matches!(
            call_with_caller(endpoint as u64, 0x1234, TaskId(2)),
            CallResult::Blocked
        ));
        unsafe {
            assert!(CALLER_BLOCKED[2]);
            assert!(ENDPOINT_TABLE[endpoint as usize].has_pending_call);
        }
    }

    fn assert_recv_from_owner_returns_packed_client_and_msg(endpoint: u16) {
        let packed = recv_with_caller(endpoint as u64, TaskId(1)).unwrap_message();
        assert_eq!(packed >> 32, 2);
        assert_eq!(packed & 0xFFFF_FFFF, 0x1234);
        unsafe {
            assert!(!ENDPOINT_TABLE[endpoint as usize].has_pending_call);
        }
    }

    fn assert_reply_from_owner_unblocks_task2(endpoint: u16) {
        assert_eq!(
            reply_with_caller(endpoint as u64, 2, 0x5678, TaskId(1)),
            Ok(())
        );
        unsafe {
            assert!(!CALLER_BLOCKED[2]);
            assert_eq!(REPLY_VALUE[2], 0x5678);
        }
    }

    #[test]
    fn recv_blocks_when_no_pending_call() {
        with_table(|| {
            let endpoint = create_with_owner(TaskId(1)).unwrap();

            let result = recv_with_caller(endpoint as u64, TaskId(1));
            match result {
                RecvResult::Blocked => {}
                RecvResult::Message(v) => panic!("expected Blocked, got Message({:#x})", v),
            }

            unsafe {
                assert_eq!(
                    ENDPOINT_TABLE[endpoint as usize].blocked_receiver,
                    TaskId(1)
                );
            }
        });
    }

    #[test]
    fn recv_gets_message_when_call_pending() {
        with_table(|| {
            let endpoint = create_with_owner(TaskId(1)).unwrap();

            unsafe {
                ENDPOINT_TABLE[endpoint as usize].has_pending_call = true;
                ENDPOINT_TABLE[endpoint as usize].pending_client = TaskId(2);
                ENDPOINT_TABLE[endpoint as usize].pending_msg = 0xABCD;
            }

            let packed = recv_with_caller(endpoint as u64, TaskId(1)).unwrap_message();
            assert_eq!(packed >> 32, 2);
            assert_eq!(packed & 0xFFFF_FFFF, 0xABCD);
        });
    }

    #[test]
    fn call_masks_request_to_low_32_bits() {
        with_table(|| {
            let endpoint = create_with_owner(TaskId(1)).unwrap();
            assert!(matches!(
                call_with_caller(endpoint as u64, 0x1_0000_0042, TaskId(2)),
                CallResult::Blocked
            ));
            let packed = recv_with_caller(endpoint as u64, TaskId(1)).unwrap_message();
            assert_eq!(packed & 0xFFFF_FFFF, 0x42);
        });
    }
}
