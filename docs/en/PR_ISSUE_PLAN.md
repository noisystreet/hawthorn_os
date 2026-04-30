# PR and issue plan (current milestone)

> **[中文](../PR与议题计划.md)** — Chinese source of this document.

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
3. **Commits:** `docs/提交约定.md` — English Conventional line 1, matching Chinese line 2.
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

---

## Status update (2026-04-29)

- M4 (preemptive scheduling) ✅: time slicing, `sleep/block/unblock`, and IRQ-driven reschedule are wired.
- M5 (SVC + syscalls) ✅: `syscall_abi` + dispatch table + `SYS_write/getpid/exit/sleep/yield` are working.
- M6 (EL0 user mode) ✅: minimum runnable loop is working; QEMU serial shows `hello from EL0!`.
- CI now includes two serial regressions:
  - `scripts/verify_kernel_qemu_virt_serial.sh`
  - `scripts/verify_kernel_qemu_virt_el0_serial.sh`
- M7-W2 (IPC MVP) 🔄: `endpoint` + `SYS_ENDPOINT_{CREATE,DESTROY,CALL,RECV,REPLY}`; current MVP carries **≤32-bit** payloads and `call/recv` return **`EAGAIN` (-11)** when the peer is not ready (callers poll; `verify_kernel_qemu_virt_el0_serial.sh` covers the EL1 `task E/D` serial roundtrip). True **≤128B** payloads and a **dual-EL0** roundtrip remain follow-up work.

> Note: current M6 is a runnable MVP; stronger isolation and lifecycle hardening are still pending (covered by the M7 split below).

---

## M7 two-week execution split (recommended)

### Week 1 (stability + correctness)

| Item | Goal | Acceptance criteria | Suggested issue |
|------|------|---------------------|-----------------|
| M7-W1-1 Full user-context save/restore | Complete EL0 trap context semantics (including GPR policy) | User task keeps running correctly across repeated syscalls + preemption | `#9-1` |
| M7-W1-2 Exit + resource reclaim | Reclaim user pages/page tables/TCB resources on `sys_exit` | Repeated create/exit cycles show no obvious resource leak | `#9-2` |
| M7-W1-3 User pointer validation baseline | Add minimal `copy_from_user` path for `sys_write` | Invalid user addresses return proper error instead of kernel fault | `#9-3` |
| M7-W1-4 Regression hardening | Add EL0 syscall/exit checks in CI | CI remains stable, failures are diagnosable from logs | `#9-4` |

### Week 2 (IPC MVP delivery)

| Item | Goal | Acceptance criteria | Suggested issue |
|------|------|---------------------|-----------------|
| M7-W2-1 Endpoint object + cap binding | Introduce endpoint object and basic rights checks | Endpoint create/destroy and deny paths are testable | `#9-5` |
| M7-W2-2 Synchronous short-message call/reply | Implement rendezvous IPC (<=128B) | Two user tasks complete a request/reply roundtrip | `#9-6` |
| M7-W2-3 Blocking queue + scheduler interaction | `recv` blocks, `reply` wakes up | Queue order and wakeup semantics remain stable | `#9-7` |
| M7-W2-4 Docs + ABI closure | Update `SYSCALL_ABI` / `KERNEL` / `TODO` | Docs and implementation are aligned, no stale interface notes | `#9-8` |

### M7 Definition of Done

1. At least two EL0 tasks can complete an end-to-end interaction via syscalls + IPC.
2. User tasks can be created/exited with resource reclaim; repeated regressions show no leak signs.
3. CI covers:
   - kernel banner regression
   - EL0 output regression
   - EL0 syscall + IPC smoke regression

---

## Six-month roadmap (draft)

### Scope boundaries

- Platform track: stabilize QEMU `virt` (AArch64) first, then move to RK3588 hardware bring-up.
- Architecture track: microkernel-first; keep drivers/services in user space whenever practical.
- Delivery standard: each month must produce a milestone that is demoable, regression-testable, and measurable.

### M+1 (Month 1): EL0 boundary hardening

- **Focus**: move from “runs” to “runs reliably”.
- **Deliverables**:
  - close EL0 context save/restore semantics (including multi-task switch scenarios)
  - user task lifecycle reclaim (`create/exit` for page tables/frames/TCB)
  - minimal safe `copy_from_user` / `copy_to_user`
  - EL0 syscall/exit stress regression scripts
- **Acceptance**:
  - 100+ repeated user task create/exit cycles without crashes
  - stable CI (base serial + EL0 serial)

### M+2 (Month 2): IPC MVP + service skeleton

- **Focus**: synchronous short-message IPC lands.
- **Deliverables**:
  - endpoint object + capability rights checks
  - rendezvous `call/reply` MVP (<=128B)
  - stable scheduler integration for block/wakeup
  - user-space service skeletons (init/name/driver template)
- **Acceptance**:
  - two EL0 processes complete request/reply roundtrip
  - 1000 IPC roundtrips without deadlock

### M+3 (Month 3): first usable device driver service

- **Focus**: move from kernel-direct to user-space driver service.
- **Deliverables**:
  - at least one user-space driver service (UART service or timer-event service)
  - minimal IRQ-to-user bridge path
  - basic device capability model (minimal rights set)
- **Acceptance**:
  - driver service handles events/data stably
  - service restart/recovery works after fault injection

### M+4 (Month 4): block-device path

- **Focus**: prerequisites for filesystem.
- **Deliverables**:
  - QEMU `virtio-blk` (or equivalent block device) MVP
  - block read/write API + simple cache layer
  - I/O timeout and error paths
- **Acceptance**:
  - stable block image read/write
  - regression coverage for data consistency

### M+5 (Month 5): filesystem MVP

- **Focus**: usable file abstraction.
- **Deliverables**:
  - minimal VFS abstraction (inode/file/ops)
  - first filesystem (RAMFS first, then simple disk-backed FS)
  - minimal syscall semantics: `open/read/write/close`
- **Acceptance**:
  - EL0 programs can read/write files
  - basic concurrent read/write paths stay stable

### M+6 (Month 6): engineering hardening + board-readiness

- **Focus**: from feature completeness to product engineering.
- **Deliverables**:
  - performance/stability baseline (boot latency, IPC latency, I/O throughput)
  - fault-injection and recovery policy (service restart/timeout/retry)
  - RK3588 bring-up blockers + minimal validation path
  - frozen docs for ABI, IPC model, driver model, FS constraints
- **Acceptance**:
  - stable one-command regression workflow
  - clear and actionable pre-hardware blocker list

### Cross-cutting guardrails (all 6 months)

- Every month enforces:
  1. regression expansion (new capability must ship with script + CI coverage)
  2. observability upgrades (logs, counters, error codes)
  3. bilingual doc sync (CN/EN docs move with implementation)
