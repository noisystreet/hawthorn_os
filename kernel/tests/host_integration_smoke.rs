// SPDX-License-Identifier: MIT OR Apache-2.0

//! Host integration tests: link `hawthorn_kernel` as a library (crate-root `tests/`),
//! distinct from in-module `#[cfg(test)]` unit tests.
//!
//! Layering: see `docs/TESTING.md` / `docs/en/TESTING.md` (L2).

use hawthorn_kernel::task_policy::{pick_next_index, SchedState, TaskView};

#[test]
fn pick_next_prefers_higher_priority_ready() {
    let tasks = [
        TaskView {
            state: SchedState::Running,
            priority: 10,
        },
        TaskView {
            state: SchedState::Ready,
            priority: 5,
        },
    ];
    // Lower `priority` value means higher importance; Ready task 1 wins over Running task 0.
    assert_eq!(pick_next_index(&tasks, 0, 1), 1);
}
