// SPDX-License-Identifier: MIT OR Apache-2.0

//! Minimal endpoint object table for IPC MVP.
//!
//! This module provides endpoint lifecycle and a small rendezvous path:
//! **call → recv → reply**.
//!
//! When the peer is not ready, [`call`] / [`recv`] return [`Errno::EAGAIN`]
//! so callers can poll (e.g. `yield` / `sleep`) without blocking inside the kernel.

use hawthorn_syscall_abi::Errno;

#[cfg(all(target_arch = "aarch64", target_os = "none"))]
use crate::task::TaskId;

#[cfg(not(all(target_arch = "aarch64", target_os = "none")))]
#[derive(Clone, Copy, PartialEq, Eq)]
struct TaskId(pub u16);

const MAX_ENDPOINTS: usize = 16;
const MAX_TASK_SLOTS: usize = 16;
const INVALID_TASK_ID: u16 = u16::MAX;

#[derive(Clone, Copy)]
struct Endpoint {
    in_use: bool,
    owner: TaskId,
    has_pending_call: bool,
    pending_client: TaskId,
    pending_msg: u64,
}

impl Endpoint {
    const EMPTY: Self = Self {
        in_use: false,
        owner: TaskId(0),
        has_pending_call: false,
        pending_client: TaskId(INVALID_TASK_ID),
        pending_msg: 0,
    };
}

#[allow(static_mut_refs)]
static mut ENDPOINT_TABLE: [Endpoint; MAX_ENDPOINTS] = [Endpoint::EMPTY; MAX_ENDPOINTS];
#[allow(static_mut_refs)]
static mut REPLY_READY: [bool; MAX_TASK_SLOTS] = [false; MAX_TASK_SLOTS];
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
            REPLY_READY[i] = false;
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

pub fn call(id: u64, msg: u64) -> Result<u64, Errno> {
    let caller = current_task_id();
    call_with_caller(id, msg, caller)
}

pub fn recv(id: u64) -> Result<u64, Errno> {
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
        *ep = Endpoint::EMPTY;
    }
    Ok(())
}

fn call_with_caller(id: u64, msg: u64, caller: TaskId) -> Result<u64, Errno> {
    let idx = id as usize;
    if idx >= MAX_ENDPOINTS {
        return Err(Errno::EINVAL);
    }
    let caller_idx = caller.0 as usize;
    if caller_idx >= MAX_TASK_SLOTS {
        return Err(Errno::EINVAL);
    }

    unsafe {
        if REPLY_READY[caller_idx] {
            REPLY_READY[caller_idx] = false;
            return Ok(REPLY_VALUE[caller_idx]);
        }

        let ep = &mut ENDPOINT_TABLE[idx];
        if !ep.in_use {
            return Err(Errno::ENOENT);
        }
        if ep.owner == caller {
            return Err(Errno::EPERM);
        }
        if ep.has_pending_call {
            return Err(Errno::EAGAIN);
        }

        ep.has_pending_call = true;
        ep.pending_client = caller;
        ep.pending_msg = msg;
        REPLY_READY[caller_idx] = false;
    }
    Err(Errno::EAGAIN)
}

fn recv_with_caller(id: u64, caller: TaskId) -> Result<u64, Errno> {
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

        if ep.has_pending_call {
            let client = ep.pending_client.0 as u64;
            let msg = ep.pending_msg & 0xFFFF_FFFF;
            ep.has_pending_call = false;
            ep.pending_client = TaskId(INVALID_TASK_ID);
            ep.pending_msg = 0;
            return Ok((client << 32) | msg);
        }
    }
    Err(Errno::EAGAIN)
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

    unsafe {
        let ep = &mut ENDPOINT_TABLE[idx];
        if !ep.in_use {
            return Err(Errno::ENOENT);
        }
        if ep.owner != caller {
            return Err(Errno::EPERM);
        }

        REPLY_VALUE[client_idx] = msg;
        REPLY_READY[client_idx] = true;
    }
    Ok(())
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
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Global endpoint table is `static mut`; default parallel unit tests would race.
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
    fn call_recv_reply_roundtrip() {
        with_table(|| {
            let endpoint = create_with_owner(TaskId(1)).unwrap();

            // Prepare a pending call as if client 2 already issued call().
            unsafe {
                ENDPOINT_TABLE[endpoint as usize].has_pending_call = true;
                ENDPOINT_TABLE[endpoint as usize].pending_client = TaskId(2);
                ENDPOINT_TABLE[endpoint as usize].pending_msg = 0x1234;
            }

            let packed = recv_with_caller(endpoint as u64, TaskId(1)).unwrap();
            assert_eq!(packed >> 32, 2);
            assert_eq!(packed & 0xFFFF_FFFF, 0x1234);

            assert_eq!(
                reply_with_caller(endpoint as u64, 2, 0x5678, TaskId(1)),
                Ok(())
            );
            unsafe {
                assert!(REPLY_READY[2]);
                assert_eq!(REPLY_VALUE[2], 0x5678);
            }
        });
    }
}
