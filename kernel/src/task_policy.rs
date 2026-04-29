// SPDX-License-Identifier: MIT OR Apache-2.0

//! Host-testable scheduling policy helpers extracted from `task`.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SchedState {
    Ready,
    Running,
    Blocked,
    Unused,
    Exited,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TaskView {
    pub state: SchedState,
    pub priority: u8,
}

/// Choose the next runnable task index.
///
/// `current_time_slice` is the running task's remaining slice (from the TCB). When it hits
/// zero, another **Ready** task at the **same** priority is chosen in round-robin order (cyclic
/// scan starting at `current + 1`). Lower `priority` values mean higher importance.
pub fn pick_next_index(tasks: &[TaskView], current: usize, current_time_slice: u64) -> usize {
    let Some(ct) = tasks.get(current) else {
        return 0;
    };
    let current_running = ct.state == SchedState::Running;
    let current_prio = ct.priority;

    let mut best_idx: Option<usize> = None;
    let mut best_prio: u8 = u8::MAX;
    for (idx, task) in tasks.iter().enumerate() {
        if task.state == SchedState::Ready && task.priority < best_prio {
            best_prio = task.priority;
            best_idx = Some(idx);
        }
    }

    let Some(b_idx) = best_idx else {
        return 0;
    };

    if !current_running {
        return b_idx;
    }

    if current_prio < best_prio {
        return current;
    }
    if current_prio > best_prio {
        return b_idx;
    }

    // Same priority as the best Ready contender.
    if current_time_slice > 0 {
        return current;
    }

    if let Some(rr) = pick_rr_same_prio_after(tasks, current, best_prio) {
        return rr;
    }

    current
}

/// Next Ready task at `prio` after `current` in cyclic index order (`current` is skipped).
fn pick_rr_same_prio_after(tasks: &[TaskView], current: usize, prio: u8) -> Option<usize> {
    let n = tasks.len();
    if n == 0 {
        return None;
    }
    for step in 1..=n {
        let i = (current + step) % n;
        let t = tasks.get(i)?;
        if t.state == SchedState::Ready && t.priority == prio {
            return Some(i);
        }
    }
    None
}

/// Per-task fields touched by the timer IRQ path (`task::tick`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimerSlot {
    pub state: SchedState,
    pub time_slice: u64,
    pub wake_tick: u64,
}

impl TimerSlot {
    pub const ZERO: Self = Self {
        state: SchedState::Unused,
        time_slice: 0,
        wake_tick: 0,
    };
}

/// Converts sleep duration to a **positive** tick count, matching `task::sleep`.
pub fn sleep_wait_ticks(ms: u64, tick_ms: u64) -> u64 {
    (if tick_ms > 0 { ms / tick_ms } else { 1 }).max(1)
}

/// One timer interrupt step: decrement current task time slice; wake sleepers whose
/// `wake_tick` is due. Returns whether the scheduler should reconsider the next task.
pub fn timer_tick_step(slots: &mut [TimerSlot], current: usize, now: u64) -> bool {
    let mut need = false;
    if current < slots.len() && slots[current].time_slice > 0 {
        slots[current].time_slice -= 1;
        if slots[current].time_slice == 0 {
            need = true;
        }
    }
    for slot in slots.iter_mut() {
        if slot.state == SchedState::Blocked && slot.wake_tick > 0 && now >= slot.wake_tick {
            slot.state = SchedState::Ready;
            slot.wake_tick = 0;
            need = true;
        }
    }
    need
}

/// Unblock a task by index (idle slot 0 is never unblocked here), matching `task::unblock`.
pub fn try_unblock(slots: &mut [TimerSlot], idx: usize) -> bool {
    if idx == 0 || idx >= slots.len() {
        return false;
    }
    if slots[idx].state == SchedState::Blocked {
        slots[idx].state = SchedState::Ready;
        slots[idx].wake_tick = 0;
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(state: SchedState, priority: u8) -> TaskView {
        TaskView { state, priority }
    }

    #[test]
    fn keeps_current_when_it_is_not_worse_than_ready_tasks() {
        let tasks = [
            t(SchedState::Running, 255),
            t(SchedState::Ready, 10),
            t(SchedState::Ready, 20),
            t(SchedState::Running, 5),
        ];
        assert_eq!(pick_next_index(&tasks, 3, 5), 3);
    }

    #[test]
    fn switches_to_higher_priority_ready_task() {
        let tasks = [
            t(SchedState::Running, 255),
            t(SchedState::Ready, 3),
            t(SchedState::Ready, 10),
            t(SchedState::Running, 20),
        ];
        assert_eq!(pick_next_index(&tasks, 3, 5), 1);
    }

    #[test]
    fn falls_back_to_idle_when_no_ready_task() {
        let tasks = [
            t(SchedState::Running, 255),
            t(SchedState::Blocked, 3),
            t(SchedState::Exited, 10),
            t(SchedState::Unused, 20),
        ];
        assert_eq!(pick_next_index(&tasks, 2, 5), 0);
    }

    #[test]
    fn same_priority_keeps_current_while_time_slice_positive() {
        let tasks = [
            t(SchedState::Running, 255),
            t(SchedState::Running, 10),
            t(SchedState::Ready, 10),
        ];
        assert_eq!(pick_next_index(&tasks, 1, 3), 1);
    }

    #[test]
    fn same_priority_round_robins_when_time_slice_zero() {
        let s1 = [
            t(SchedState::Running, 255),
            t(SchedState::Running, 10),
            t(SchedState::Ready, 10),
            t(SchedState::Ready, 10),
        ];
        assert_eq!(pick_next_index(&s1, 1, 0), 2);

        let s2 = [
            t(SchedState::Running, 255),
            t(SchedState::Ready, 10),
            t(SchedState::Running, 10),
            t(SchedState::Ready, 10),
        ];
        assert_eq!(pick_next_index(&s2, 2, 0), 3);

        let s3 = [
            t(SchedState::Running, 255),
            t(SchedState::Ready, 10),
            t(SchedState::Ready, 10),
            t(SchedState::Running, 10),
        ];
        assert_eq!(pick_next_index(&s3, 3, 0), 1);
    }

    #[test]
    fn exhausted_slice_stays_current_if_no_peer_at_same_priority() {
        let tasks = [
            t(SchedState::Running, 255),
            t(SchedState::Running, 10),
            t(SchedState::Ready, 20),
        ];
        assert_eq!(pick_next_index(&tasks, 1, 0), 1);
    }

    #[test]
    fn sleep_wait_ticks_matches_kernel_formula() {
        assert_eq!(sleep_wait_ticks(0, 10), 1);
        assert_eq!(sleep_wait_ticks(100, 10), 10);
        assert_eq!(sleep_wait_ticks(100, 0), 1);
    }

    #[test]
    fn timer_tick_decrements_slice_and_sets_need_when_zero() {
        let mut slots = [
            TimerSlot {
                state: SchedState::Running,
                time_slice: 2,
                wake_tick: 0,
            },
            TimerSlot::ZERO,
        ];
        assert!(!timer_tick_step(&mut slots, 0, 0));
        assert_eq!(slots[0].time_slice, 1);
        assert!(timer_tick_step(&mut slots, 0, 0));
        assert_eq!(slots[0].time_slice, 0);
    }

    #[test]
    fn timer_tick_wakes_blocked_when_due() {
        let mut slots = [
            TimerSlot::ZERO,
            TimerSlot {
                state: SchedState::Blocked,
                time_slice: 0,
                wake_tick: 5,
            },
        ];
        assert!(!timer_tick_step(&mut slots, 0, 4));
        assert_eq!(slots[1].state, SchedState::Blocked);
        assert!(timer_tick_step(&mut slots, 0, 5));
        assert_eq!(slots[1].state, SchedState::Ready);
        assert_eq!(slots[1].wake_tick, 0);
    }

    #[test]
    fn timer_tick_does_not_wake_blocked_with_zero_wake_tick() {
        let mut slots = [
            TimerSlot::ZERO,
            TimerSlot {
                state: SchedState::Blocked,
                time_slice: 0,
                wake_tick: 0,
            },
        ];
        assert!(!timer_tick_step(&mut slots, 0, 100));
        assert_eq!(slots[1].state, SchedState::Blocked);
    }

    #[test]
    fn try_unblock_idle_and_non_blocked_noop() {
        let mut slots = [
            TimerSlot {
                state: SchedState::Blocked,
                time_slice: 0,
                wake_tick: 1,
            },
            TimerSlot {
                state: SchedState::Ready,
                time_slice: 0,
                wake_tick: 0,
            },
        ];
        assert!(!try_unblock(&mut slots, 0));
        assert!(!try_unblock(&mut slots, 2));
        assert!(!try_unblock(&mut slots, 1));
        assert_eq!(slots[1].state, SchedState::Ready);
    }

    #[test]
    fn try_unblock_blocked_sets_ready() {
        let mut slots = [
            TimerSlot::ZERO,
            TimerSlot {
                state: SchedState::Blocked,
                time_slice: 0,
                wake_tick: 7,
            },
        ];
        assert!(try_unblock(&mut slots, 1));
        assert_eq!(slots[1].state, SchedState::Ready);
        assert_eq!(slots[1].wake_tick, 0);
    }
}
