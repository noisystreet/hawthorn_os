# PR and issue plan (current milestone)

> **[中文](../PR_ISSUE_PLAN.md)** — Chinese source of this document.

This page pins **GitHub issues** and the recommended **PR order** so branches and PR bodies can use `Closes #…` / `Refs #…`. The capability backlog remains [TODO.md](./TODO.md).

---

## Currently tracked: minimal `hawthorn_kernel` bring-up on QEMU `virt`

| Role | Link |
|------|------|
| **Meta (rollup)** | <https://github.com/noisystreet/hawthorn_os/issues/5> |

### Issues (suggested implementation order)

| Order | Issue | Title (summary) | Status |
|-------|--------|-----------------|--------|
| 1 | [#1](https://github.com/noisystreet/hawthorn_os/issues/1) | M1: `hawthorn_kernel` minimal boot (QEMU virt) + PL011 panic | ✅ Done |
| 2 | [#2](https://github.com/noisystreet/hawthorn_os/issues/2) | M1b: `qemu_minimal` starts via `hawthorn_kernel` public API | ✅ Done |
| 3 | [#3](https://github.com/noisystreet/hawthorn_os/issues/3) | M2: `VBAR_EL1` vectors + GICv3 + IRQ dispatch | ✅ Done |
| 4 | [#4](https://github.com/noisystreet/hawthorn_os/issues/4) | M3: cooperative scheduler MVP (TCB / ready queue / yield) | ✅ Done |

**Suggested PR sequence:** `#1 → #2 → #3 → #4`. **#3** may proceed in parallel with **#2** once M1 entry symbols are stable; resolve rebase conflicts if both touch boot/entry.

---

## PR conventions

1. **One PR per issue** when practical; split large work but each PR should still `Closes #n` or `Refs #n`.
2. Use [.github/pull_request_template.md](../../.github/pull_request_template.md); under **Related issues** add e.g. `Closes #1`.
3. **Commits:** `docs/COMMIT_CONVENTIONS.md` — English Conventional line 1, matching Chinese line 2.
4. **Labels:** kernel work uses `kernel` + `enhancement`; new issues should keep tags like **`[kernel]`**, `[IPC]` (same habit as [TODO.md](./TODO.md)).

---

## Local verification (CI / AGENTS)

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo check -p hawthorn_kernel
cargo check -p hawthorn_kernel --target aarch64-unknown-none
cargo build -p hawthorn_qemu_minimal --features bare-metal --target aarch64-unknown-none
```

After M1 lands, if new crate features or bins appear, update this file and the **acceptance criteria** on the relevant issue.

---

## Next (issues not created yet)

Examples for the next batch (from TODO): `syscall_abi` crate, unified SVC dispatch, minimal IPC (short messages). After meta **#5** closes, open a new `[meta]` issue for the next phase.

---

## Phase 2: Preemptive Scheduling + User-Space Boundary + IPC

M1–M3 are all complete. Below are the suggested Phase 2 milestones and corresponding issues:

| Order | Suggested Issue | Title (summary) | Dependencies |
|-------|-----------------|-----------------|--------------|
| 5 | **#6** (suggested) | M4: Preemptive scheduling + blocking primitives (time slice, sleep, wait queues) | Timer + TCB ready |
| 6 | **#7** (suggested) | M5: SVC entry + syscall dispatch + `syscall_abi` crate | M2 exception vectors have EL0 Sync slot |
| 7 | **#8** (suggested) | M6: User address space (EL0 page tables, user stack, `eret` to user mode) | syscall ABI |
| 8 | **#9** (suggested) | M7: Synchronous short-message IPC (call/reply, rendezvous, ≤128B copy) | User address space + TCB blocking |

### M4 Design Points

**Goal**: Upgrade from cooperative to **fixed-priority preemptive scheduling (FP)** with round-robin within the same priority; add blocking primitives.

- **Time slice**: Timer interrupt decrements current task's `time_slice` counter; when exhausted, set `need_reschedule` flag and call `schedule()` before returning from interrupt.
- **Preemptive scheduling**: `schedule()` picks the highest-priority ready task; if current task's priority is not higher than the next, force a switch.
- **Blocking primitives**:
  - `task::sleep(ms)` — insert into timer wait queue, wake back to ready queue on expiry
  - `task::block()` / `task::unblock(id)` — manual block/unblock for use by IPC and future modules
- **Wait queue**: sorted by expiry time (singly-linked list, or simplified fixed-array scan); Timer tick checks and wakes expired tasks.
- **Tickless idle** (optional): when no ready tasks, stop periodic tick and set a one-shot timer to the next wakeup (`CNTP_TVAL_EL0`).

### M5 Design Points

**Goal**: Establish a stable syscall ABI between kernel and user space.

- **SVC entry**: `ESR.EC = 0x15` branch parses `x8` = syscall number, `x0–x7` = arguments, dispatches to handler table
- **`syscall_abi` crate** (independent crate, shared by kernel and user space):
  - `SYS_*` number constants, `MAX_ARGS`, error code enum
  - Version probe syscall (returns `ABI_VERSION` + capability bitmask)
- **Register convention**: `x8` = syscall number, `x0–x5` = arguments, `x0` = return value + error code
- **Invalid syscall**: return `ENOSYS`

### M6 Design Points

**Goal**: Support EL0 user-space tasks; kernel manages independent address spaces.

- **Address space object**: page table root (TTBR0_EL1), ASID, reference count
- **map / unmap API**: bound to capability; `PROT_READ` / `PROT_WRITE` / `PROT_EXEC`, default W^X
- **User stack mapping**: pre-mapped + guard page
- **Context switch**: swap TTBR0_EL1 + ASID + TLB flush
- **User-space entry/return**: set `SP_EL0`, `ELR_EL1`, `SPSR_EL1` (EL0 AArch64), `eret`

### M7 Design Points

**Goal**: Implement the microkernel's core communication mechanism — synchronous short-message IPC.

- **call / reply**: client `call(endpoint, msg)` blocks until server `reply`; rendezvous semantics
- **Message payload**: ≤128B inline copy (register-passing optimization can be added later)
- **Endpoint object**: holds waiting-receiver queue; bound to capability
- **Blocking & scheduling coordination**: `recv` with no message → `block()`; `send` to blocked target → `unblock()`
- **Capability transfer**: move or copy capability slots with messages (MVP can support move semantics only)

### Suggested PR Sequence

`#6 (M4) → #7 (M5) → #8 (M6) → #9 (M7)`

M5 and M6 can proceed partially in parallel (SVC dispatch and address space design are independent), but M6 depends on M5's syscall ABI definitions, and M7 depends on M6's user-space context switch.

### Long-term Direction (Phase 3)

- Physical memory frame allocator → buddy system
- Capability system (unforgeable handles, generations, right derivation, cascading revocation)
- Notification / IrqControl (interrupt completion ↔ thread wakeup bridge)
- SMP multi-core support (AP startup, per-CPU data, IPI, load balancing)
- RK3588 / Orange Pi 5 real hardware BSP
- User-space driver services, root service init
