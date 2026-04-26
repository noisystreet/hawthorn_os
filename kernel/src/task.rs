// SPDX-License-Identifier: MIT OR Apache-2.0

//! Cooperative scheduler MVP: TCB, ready queue, context switch.
//!
//! Supports up to `MAX_TASKS` concurrent tasks with round-robin scheduling.
//! Task 0 is the idle task (the `kernel_main` context). Tasks yield
//! explicitly via [`yield_now`].
//!
//! Context switch saves/restores callee-saved registers (x19–x28, x29/FP,
//! x30/LR) and the stack pointer. New tasks start via `task_trampoline`
//! which calls the entry function stored in x19 and falls through to
//! `task_exit` if the entry ever returns.

use core::arch::global_asm;
use core::mem::size_of;

/// Maximum number of concurrent tasks (including idle).
const MAX_TASKS: usize = 8;

/// Per-task kernel stack size in bytes.
const STACK_SIZE: usize = 4096;

/// Task lifecycle states.
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TaskState {
    Unused = 0,
    Ready = 1,
    Running = 2,
    Blocked = 3,
    Exited = 4,
}

/// Opaque task identifier (index into the task table).
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct TaskId(pub u16);

/// Callee-saved register frame for context switch.
///
/// Layout matches the push/pop order in `context_switch`:
/// x19–x28, x29 (FP), x30 (LR) → 12 registers × 8 B = 96 B.
#[repr(C)]
struct TaskContext {
    x19: u64,
    x20: u64,
    x21: u64,
    x22: u64,
    x23: u64,
    x24: u64,
    x25: u64,
    x26: u64,
    x27: u64,
    x28: u64,
    fp: u64,
    lr: u64,
}

/// Task control block.
struct Task {
    sp: u64,
    state: TaskState,
    #[allow(dead_code)]
    priority: u8,
    id: TaskId,
}

impl Task {
    const EMPTY: Self = Self {
        sp: 0,
        state: TaskState::Unused,
        priority: 0,
        id: TaskId(0),
    };
}

#[allow(static_mut_refs)]
static mut TASK_TABLE: [Task; MAX_TASKS] = [Task::EMPTY; MAX_TASKS];

static mut CURRENT_TASK: usize = 0;

static mut TASK_STACKS: [u8; MAX_TASKS * STACK_SIZE] = [0; MAX_TASKS * STACK_SIZE];

// ---------------------------------------------------------------------------
// Context switch (assembly)
// ---------------------------------------------------------------------------
// x0 = &old_task.sp   (where to save current SP)
// x1 = &new_task.sp   (where to load new SP from)
//
// Push order: x29/x30, x27/x28, x25/x26, x23/x24, x21/x22, x19/x20
// → matches TaskContext layout from low to high address.
global_asm!(
    ".global context_switch",
    ".type context_switch, @function",
    ".align 4",
    "context_switch:",
    "stp x29, x30, [sp, #-16]!",
    "stp x27, x28, [sp, #-16]!",
    "stp x25, x26, [sp, #-16]!",
    "stp x23, x24, [sp, #-16]!",
    "stp x21, x22, [sp, #-16]!",
    "stp x19, x20, [sp, #-16]!",
    "mov x2, sp",
    "str x2, [x0]",
    "ldr x2, [x1]",
    "mov sp, x2",
    "ldp x19, x20, [sp], #16",
    "ldp x21, x22, [sp], #16",
    "ldp x23, x24, [sp], #16",
    "ldp x25, x26, [sp], #16",
    "ldp x27, x28, [sp], #16",
    "ldp x29, x30, [sp], #16",
    "ret",
);

// ---------------------------------------------------------------------------
// Task entry trampoline (assembly)
// ---------------------------------------------------------------------------
// x19 = entry point (set during task_create)
// x30 = this trampoline (set during task_create as lr)
//
// Calls entry via blr x19; if it returns, falls through to task_exit.
global_asm!(
    ".global task_trampoline",
    ".type task_trampoline, @function",
    ".align 4",
    "task_trampoline:",
    "blr x19",
    "b task_exit",
);

extern "C" {
    fn context_switch(old_sp: *mut u64, new_sp: *const u64);
    fn task_trampoline();
}

/// Safety net: called if a task entry function returns.
#[no_mangle]
extern "C" fn task_exit() -> ! {
    unsafe {
        let id = CURRENT_TASK as u16;
        TASK_TABLE[CURRENT_TASK].state = TaskState::Exited;
        crate::println!("[task] task {} exited", id);
    }
    loop {
        yield_now();
    }
}

/// Initialize the task subsystem. Must be called once before `create` / `yield_now`.
///
/// Sets up task 0 as the current (idle) running context.
pub fn init() {
    unsafe {
        TASK_TABLE[0] = Task {
            sp: 0,
            state: TaskState::Running,
            priority: 255,
            id: TaskId(0),
        };
        CURRENT_TASK = 0;
    }
}

/// Create a new task.
///
/// `entry` is the task function (should never return; if it does,
/// `task_exit` is invoked as a safety net).
/// `priority` is stored for future use (round-robin ignores it for now).
///
/// Returns `Some(TaskId)` on success, `None` if the task table is full.
pub fn create(entry: extern "C" fn(), priority: u8) -> Option<TaskId> {
    unsafe {
        let mut idx = None;
        for i in 1..MAX_TASKS {
            if TASK_TABLE[i].state == TaskState::Unused {
                idx = Some(i);
                break;
            }
        }
        let idx = idx?;

        let stack_base = core::ptr::addr_of!(TASK_STACKS) as u64;
        let stack_top = stack_base + ((idx + 1) * STACK_SIZE) as u64;
        let stack_top = stack_top & !0xFu64;

        let ctx_ptr = (stack_top - size_of::<TaskContext>() as u64) as *mut TaskContext;
        core::ptr::write_bytes(ctx_ptr, 0, 1);

        (*ctx_ptr).lr = task_trampoline as *const () as usize as u64;
        (*ctx_ptr).x19 = entry as *const () as usize as u64;

        TASK_TABLE[idx] = Task {
            sp: ctx_ptr as u64,
            state: TaskState::Ready,
            priority,
            id: TaskId(idx as u16),
        };

        Some(TaskId(idx as u16))
    }
}

/// Get the current task's ID.
pub fn current_id() -> TaskId {
    unsafe { TASK_TABLE[CURRENT_TASK].id }
}

/// Cooperatively yield to the next ready task.
///
/// If no other task is ready, returns immediately.
pub fn yield_now() {
    unsafe {
        let current = CURRENT_TASK;

        let mut next = None;
        for i in 1..=MAX_TASKS {
            let candidate = (current + i) % MAX_TASKS;
            if TASK_TABLE[candidate].state == TaskState::Ready {
                next = Some(candidate);
                break;
            }
        }

        let next = match next {
            Some(n) => n,
            None => return,
        };

        TASK_TABLE[current].state = TaskState::Ready;
        TASK_TABLE[next].state = TaskState::Running;
        CURRENT_TASK = next;

        context_switch(&mut TASK_TABLE[current].sp, &TASK_TABLE[next].sp);
    }
}
