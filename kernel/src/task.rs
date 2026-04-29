// SPDX-License-Identifier: MIT OR Apache-2.0

//! Preemptive scheduler with blocking primitives.
//!
//! Fixed-priority preemptive scheduling (FP) with round-robin within the
//! same priority. Task 0 is the idle task (lowest priority). Supports:
//! - **Time-slice preemption**: timer tick decrements `time_slice`; on
//!   expiry the flag `NEED_RESCHEDULE` is set and `schedule()` is called
//!   before returning from the IRQ handler.
//! - **Voluntary yield**: [`yield_now`] sets current to Ready and calls
//!   `schedule()`.
//! - **Sleep**: [`sleep`] blocks the current task for N milliseconds; the
//!   timer handler scans sleeping tasks and unblocks those whose
//!   `wake_tick` has been reached.
//! - **Manual block/unblock**: [`block`] / [`unblock`] for IPC and other
//!   primitives.
//!
//! Context switch saves/restores callee-saved registers (x19–x30) **and**
//! the DAIF register so that IRQ masking state is correctly preserved when
//! switching between IRQ-preempted and voluntarily-yielded tasks.

use core::arch::asm;
use core::arch::global_asm;
use core::mem::size_of;

use crate::task_policy::{
    pick_next_index, sleep_wait_ticks, timer_tick_step, SchedState, TaskView, TimerSlot,
};

extern "C" {
    static __user_program_start: u8;
    static __user_program_end: u8;
}

const MAX_TASKS: usize = 8;

const STACK_SIZE: usize = 4096;
const MAX_USER_OWNED_FRAMES: usize = 16;

const DEFAULT_TIME_SLICE: u64 = 10;

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TaskState {
    Unused = 0,
    Ready = 1,
    Running = 2,
    Blocked = 3,
    Exited = 4,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(transparent)]
pub struct TaskId(pub u16);

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

/// Task Control Block (TCB)
///
/// This structure is accessed from assembly code in trap.rs.
/// Do not change field order without updating the assembly offsets.
#[repr(C)]
pub struct Task {
    pub sp: u64,
    pub state: TaskState,
    pub priority: u8,
    pub id: TaskId,
    pub time_slice: u64,
    pub daif: u64,
    pub wake_tick: u64,
    // User task fields (only valid when is_user is true)
    pub is_user: bool,
    pub user_page_table: usize,
    pub saved_elr: u64,
    pub saved_spsr: u64,
    pub saved_sp_el0: u64,
}

impl Task {
    const EMPTY: Self = Self {
        sp: 0,
        state: TaskState::Unused,
        priority: 0,
        id: TaskId(0),
        time_slice: 0,
        daif: 0,
        wake_tick: 0,
        is_user: false,
        user_page_table: 0,
        saved_elr: 0,
        saved_spsr: 0,
        saved_sp_el0: 0,
    };
}

#[allow(static_mut_refs)]
static mut TASK_TABLE: [Task; MAX_TASKS] = [Task::EMPTY; MAX_TASKS];
static mut USER_OWNED_FRAMES: [[usize; MAX_USER_OWNED_FRAMES]; MAX_TASKS] =
    [[0; MAX_USER_OWNED_FRAMES]; MAX_TASKS];
static mut USER_OWNED_FRAME_COUNTS: [usize; MAX_TASKS] = [0; MAX_TASKS];

static mut CURRENT_TASK: usize = 0;

static mut TASK_STACKS: [u8; MAX_TASKS * STACK_SIZE] = [0; MAX_TASKS * STACK_SIZE];

static mut NEED_RESCHEDULE: bool = false;

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
    #[allow(dead_code)]
    fn user_return(task_ptr: *mut Task);
}

#[no_mangle]
extern "C" fn task_exit() -> ! {
    unsafe {
        release_task_resources(CURRENT_TASK);
        TASK_TABLE[CURRENT_TASK].state = TaskState::Exited;
        NEED_RESCHEDULE = true;
        schedule();
    }
    loop {
        core::hint::spin_loop();
    }
}

pub fn init() {
    unsafe {
        TASK_TABLE[0] = Task {
            sp: 0,
            state: TaskState::Running,
            priority: 255,
            id: TaskId(0),
            time_slice: DEFAULT_TIME_SLICE,
            daif: 0,
            wake_tick: 0,
            is_user: false,
            user_page_table: 0,
            saved_elr: 0,
            saved_spsr: 0,
            saved_sp_el0: 0,
        };
        CURRENT_TASK = 0;
        NEED_RESCHEDULE = false;
    }
}

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
            time_slice: DEFAULT_TIME_SLICE,
            daif: 0,
            wake_tick: 0,
            is_user: false,
            user_page_table: 0,
            saved_elr: 0,
            saved_spsr: 0,
            saved_sp_el0: 0,
        };
        for slot in 0..MAX_USER_OWNED_FRAMES {
            USER_OWNED_FRAMES[idx][slot] = 0;
        }
        USER_OWNED_FRAME_COUNTS[idx] = 0;

        Some(TaskId(idx as u16))
    }
}

/// Create a new EL0 user task.
///
/// # Arguments
/// - `entry`: User program entry virtual address
/// - `stack_top`: User stack top virtual address
///
/// # Returns
/// - `Some(TaskId)`: Success
/// - `None`: Task table full or out of memory
pub fn create_user(entry: usize, stack_top: usize) -> Option<TaskId> {
    unsafe {
        // Find a free task slot
        let mut idx = None;
        for i in 1..MAX_TASKS {
            if TASK_TABLE[i].state == TaskState::Unused {
                idx = Some(i);
                break;
            }
        }
        let idx = idx?;

        // Allocate kernel stack for the task (for syscall/trap handling)
        let stack_base = core::ptr::addr_of!(TASK_STACKS) as u64;
        let kernel_stack_top = stack_base + ((idx + 1) * STACK_SIZE) as u64;
        let kernel_stack_top = kernel_stack_top & !0xFu64;

        // Create initial context (will be used on first schedule)
        // The context will be restored by context_switch, which then returns
        // to task_trampoline -> entry function
        let ctx_ptr = (kernel_stack_top - size_of::<TaskContext>() as u64) as *mut TaskContext;
        core::ptr::write_bytes(ctx_ptr, 0, 1);

        // For user tasks, we use a special trampoline that sets up EL0 state
        (*ctx_ptr).lr = user_task_trampoline as *const () as usize as u64;
        (*ctx_ptr).x19 = entry as u64;

        // Create user page table with kernel mappings cloned
        let user_pt = crate::mm::create_user_page_table()?;

        // Map user code pages (copy embedded .user_program into user frames).
        let user_prog_start = core::ptr::addr_of!(__user_program_start) as usize;
        let user_prog_end = core::ptr::addr_of!(__user_program_end) as usize;
        let user_prog_size = user_prog_end.saturating_sub(user_prog_start);
        let pages_needed = (user_prog_size + 4095) / 4096;
        for slot in 0..MAX_USER_OWNED_FRAMES {
            USER_OWNED_FRAMES[idx][slot] = 0;
        }
        USER_OWNED_FRAME_COUNTS[idx] = 0;
        if pages_needed + 1 > MAX_USER_OWNED_FRAMES {
            crate::println!(
                "[task] create_user needs {} frames, max tracked {}",
                pages_needed + 1,
                MAX_USER_OWNED_FRAMES
            );
            crate::frame_alloc::free_frame(user_pt);
            return None;
        }

        for i in 0..pages_needed {
            let src_paddr = user_prog_start + i * 4096;
            let dst_vaddr = entry + i * 4096;
            let Some(frame) = crate::frame_alloc::alloc_frame() else {
                cleanup_user_allocation(idx, user_pt);
                return None;
            };
            core::ptr::copy_nonoverlapping(src_paddr as *const u8, frame as *mut u8, 4096);
            if !crate::mm::map_user_page(
                user_pt,
                dst_vaddr,
                frame,
                crate::mm::PTE_AP_RW_ALL | crate::mm::ATTR_NORMAL,
            ) {
                crate::frame_alloc::free_frame(frame);
                cleanup_user_allocation(idx, user_pt);
                return None;
            }
            let owned = USER_OWNED_FRAME_COUNTS[idx];
            USER_OWNED_FRAMES[idx][owned] = frame;
            USER_OWNED_FRAME_COUNTS[idx] = owned + 1;
        }

        // Map user stack (4K stack page)
        let user_stack_bottom = stack_top - STACK_SIZE;
        let Some(stack_frame) = crate::frame_alloc::alloc_frame() else {
            cleanup_user_allocation(idx, user_pt);
            return None;
        };
        if !crate::mm::map_user_page(
            user_pt,
            user_stack_bottom,
            stack_frame,
            crate::mm::PTE_AP_RW_ALL | crate::mm::ATTR_NORMAL | crate::mm::PTE_UXN,
        ) {
            crate::frame_alloc::free_frame(stack_frame);
            cleanup_user_allocation(idx, user_pt);
            return None;
        }
        let owned = USER_OWNED_FRAME_COUNTS[idx];
        USER_OWNED_FRAMES[idx][owned] = stack_frame;
        USER_OWNED_FRAME_COUNTS[idx] = owned + 1;

        TASK_TABLE[idx] = Task {
            sp: ctx_ptr as u64,
            state: TaskState::Ready,
            priority: 128, // User tasks default to middle priority
            id: TaskId(idx as u16),
            time_slice: DEFAULT_TIME_SLICE,
            daif: 0,
            wake_tick: 0,
            is_user: true,
            user_page_table: user_pt,
            saved_elr: entry as u64,
            // Initial EL0 entry state:
            // - M[3:0]=0b0000 (EL0t)
            // - NZCV/DAIF cleared as baseline for a fresh user task
            // Subsequent returns from exceptions must restore the saved SPSR value.
            saved_spsr: 0x0000_0000,
            saved_sp_el0: stack_top as u64,
        };

        Some(TaskId(idx as u16))
    }
}

pub fn exit_current() -> ! {
    unsafe {
        release_task_resources(CURRENT_TASK);
        TASK_TABLE[CURRENT_TASK].state = TaskState::Exited;
        NEED_RESCHEDULE = true;
        schedule();
    }
    loop {
        core::hint::spin_loop();
    }
}

fn cleanup_user_allocation(task_idx: usize, user_pt: usize) {
    let count = unsafe { USER_OWNED_FRAME_COUNTS[task_idx] };
    for i in 0..count {
        let frame = unsafe { USER_OWNED_FRAMES[task_idx][i] };
        if frame != 0 {
            crate::frame_alloc::free_frame(frame);
        }
    }
    unsafe {
        for slot in 0..MAX_USER_OWNED_FRAMES {
            USER_OWNED_FRAMES[task_idx][slot] = 0;
        }
        USER_OWNED_FRAME_COUNTS[task_idx] = 0;
    }
    crate::frame_alloc::free_frame(user_pt);
}

fn release_task_resources(task_idx: usize) {
    let task = unsafe { &mut TASK_TABLE[task_idx] };
    if !task.is_user {
        return;
    }
    let released_frames = unsafe { USER_OWNED_FRAME_COUNTS[task_idx] };
    let released_pt = task.user_page_table;

    for i in 0..released_frames {
        let frame = unsafe { USER_OWNED_FRAMES[task_idx][i] };
        if frame != 0 {
            crate::frame_alloc::free_frame(frame);
        }
    }
    unsafe {
        for slot in 0..MAX_USER_OWNED_FRAMES {
            USER_OWNED_FRAMES[task_idx][slot] = 0;
        }
        USER_OWNED_FRAME_COUNTS[task_idx] = 0;
    }

    if task.user_page_table != 0 {
        crate::frame_alloc::free_frame(task.user_page_table);
        task.user_page_table = 0;
    }
    task.saved_elr = 0;
    task.saved_spsr = 0;
    task.saved_sp_el0 = 0;
    task.is_user = false;
    crate::println!(
        "[task] released user resources: task={} frames={} page_table={:#x}",
        task_idx,
        released_frames,
        released_pt
    );
}

/// Check if current task is a user task.
pub fn current_is_user() -> bool {
    unsafe { TASK_TABLE[CURRENT_TASK].is_user }
}

/// Get current task's user page table (TTBR0_EL1 value).
pub fn current_user_page_table() -> usize {
    unsafe { TASK_TABLE[CURRENT_TASK].user_page_table }
}

/// Get current task's saved ELR_EL1 (for user return).
pub fn current_saved_elr() -> u64 {
    unsafe { TASK_TABLE[CURRENT_TASK].saved_elr }
}

/// Get current task's saved SPSR_EL1 (for user return).
pub fn current_saved_spsr() -> u64 {
    unsafe { TASK_TABLE[CURRENT_TASK].saved_spsr }
}

/// Get current task's saved SP_EL0 (for user return).
pub fn current_saved_sp_el0() -> u64 {
    unsafe { TASK_TABLE[CURRENT_TASK].saved_sp_el0 }
}

/// Set current task's saved registers after syscall/trap.
pub fn set_current_saved_context(elr: u64, spsr: u64, sp_el0: u64) {
    unsafe {
        let idx = CURRENT_TASK;
        TASK_TABLE[idx].saved_elr = elr;
        TASK_TABLE[idx].saved_spsr = spsr;
        TASK_TABLE[idx].saved_sp_el0 = sp_el0;
    }
}

extern "C" {
    fn user_task_trampoline();
}

global_asm!(
    ".global user_task_trampoline",
    ".type user_task_trampoline, @function",
    ".align 4",
    "user_task_trampoline:",
    // Enter EL0 through trap.rs:user_return (sets ELR/SPSR/SP_EL0/TTBR0 then eret).
    "bl task_current_ptr",
    "bl user_return",
    // If user_return unexpectedly returns, terminate this task.
    "b task_exit",
);

#[no_mangle]
extern "C" fn task_current_ptr() -> *mut Task {
    unsafe { &mut TASK_TABLE[CURRENT_TASK] as *mut Task }
}

pub fn current_id() -> TaskId {
    unsafe { TASK_TABLE[CURRENT_TASK].id }
}

unsafe fn pick_next_task() -> usize {
    let mut view = [TaskView {
        state: SchedState::Unused,
        priority: 0,
    }; MAX_TASKS];
    for i in 0..MAX_TASKS {
        view[i] = TaskView {
            state: task_state_to_sched(TASK_TABLE[i].state),
            priority: TASK_TABLE[i].priority,
        };
    }
    pick_next_index(&view, CURRENT_TASK, TASK_TABLE[CURRENT_TASK].time_slice)
}

fn task_state_to_sched(s: TaskState) -> SchedState {
    match s {
        TaskState::Unused => SchedState::Unused,
        TaskState::Ready => SchedState::Ready,
        TaskState::Running => SchedState::Running,
        TaskState::Blocked => SchedState::Blocked,
        TaskState::Exited => SchedState::Exited,
    }
}

fn sched_to_task_state(s: SchedState) -> TaskState {
    match s {
        SchedState::Unused => TaskState::Unused,
        SchedState::Ready => TaskState::Ready,
        SchedState::Running => TaskState::Running,
        SchedState::Blocked => TaskState::Blocked,
        SchedState::Exited => TaskState::Exited,
    }
}

fn timer_slot_from_task(t: &Task) -> TimerSlot {
    TimerSlot {
        state: task_state_to_sched(t.state),
        time_slice: t.time_slice,
        wake_tick: t.wake_tick,
    }
}

fn apply_timer_slot_to_task(t: &mut Task, s: &TimerSlot) {
    t.state = sched_to_task_state(s.state);
    t.time_slice = s.time_slice;
    t.wake_tick = s.wake_tick;
}

pub fn schedule() {
    unsafe {
        let current = CURRENT_TASK;
        let next = pick_next_task();

        if next == current && TASK_TABLE[current].state == TaskState::Running {
            TASK_TABLE[current].time_slice = DEFAULT_TIME_SLICE;
            return;
        }

        if TASK_TABLE[current].state == TaskState::Running {
            TASK_TABLE[current].state = TaskState::Ready;
        }

        TASK_TABLE[next].state = TaskState::Running;
        TASK_TABLE[next].time_slice = DEFAULT_TIME_SLICE;
        CURRENT_TASK = next;

        let daif: u64;
        asm!("mrs {}, daif", out(reg) daif);
        TASK_TABLE[current].daif = daif;

        // Always switch via context_switch so the current task's kernel stack pointer
        // is saved in TASK_TABLE[current].sp before any user-mode transition.
        context_switch(&mut TASK_TABLE[current].sp, &TASK_TABLE[next].sp);

        asm!("msr daif, {}", in(reg) TASK_TABLE[CURRENT_TASK].daif);
    }
}

pub fn yield_now() {
    unsafe {
        if TASK_TABLE[CURRENT_TASK].state == TaskState::Running {
            TASK_TABLE[CURRENT_TASK].state = TaskState::Ready;
        }
        schedule();
    }
}

pub fn sleep(ms: u64) {
    let ticks = sleep_wait_ticks(ms, crate::timer::tick_ms());
    unsafe {
        let wake_tick = crate::timer::tick_count() + ticks;
        TASK_TABLE[CURRENT_TASK].state = TaskState::Blocked;
        TASK_TABLE[CURRENT_TASK].wake_tick = wake_tick;
        schedule();
    }
}

pub fn block() {
    unsafe {
        TASK_TABLE[CURRENT_TASK].state = TaskState::Blocked;
        schedule();
    }
}

pub fn unblock(id: TaskId) {
    let idx = id.0 as usize;
    if idx == 0 || idx >= MAX_TASKS {
        return;
    }
    unsafe {
        if TASK_TABLE[idx].state == TaskState::Blocked {
            TASK_TABLE[idx].state = TaskState::Ready;
            TASK_TABLE[idx].wake_tick = 0;
            NEED_RESCHEDULE = true;
        }
    }
}

pub fn tick() {
    unsafe {
        let current = CURRENT_TASK;
        let now = crate::timer::tick_count();
        let mut slots = [TimerSlot::ZERO; MAX_TASKS];
        for i in 0..MAX_TASKS {
            slots[i] = timer_slot_from_task(&TASK_TABLE[i]);
        }
        if timer_tick_step(&mut slots, current, now) {
            NEED_RESCHEDULE = true;
        }
        for i in 0..MAX_TASKS {
            apply_timer_slot_to_task(&mut TASK_TABLE[i], &slots[i]);
        }
    }
}

pub fn need_reschedule() -> bool {
    unsafe { NEED_RESCHEDULE }
}

pub fn clear_need_reschedule() {
    unsafe {
        NEED_RESCHEDULE = false;
    }
}
