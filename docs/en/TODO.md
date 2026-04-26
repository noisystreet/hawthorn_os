# Hawthorn / 山楂 — New features and capability extensions (TODO)

> **[中文](../TODO.md)** — Chinese source of this document.

This page lists **new runtime features and capabilities** planned for the repo. Background and constraints: [ARCHITECTURE.md](../ARCHITECTURE.md), [KERNEL.md](../KERNEL.md), [PORTING.md](../PORTING.md). Sub-items should be **splittable into issues**; track with `- [ ]` / `- [x]` in PRs.

---

## Current implemented modules (M1–M3)

| Module | File | Functionality |
|--------|------|---------------|
| Boot | `boot_qemu_virt.rs` / `bin/qemu_virt.rs` | EL2→EL1 drop + MMU disable + BSS zero + PL011 init |
| Console | `console.rs` | `print!` / `println!` macros over PL011 UART |
| Exception/Vectors | `trap.rs` | `VBAR_EL1` 16-slot vector table, `TrapFrame`, `handle_exception` dispatch |
| GICv3 | `gic.rs` | Distributor / Redistributor / CPU Interface init, `ack()` / `eoi()` |
| IRQ dispatch | `irq.rs` | 1020-slot handler table, `register()` / `dispatch()` |
| Timer | `timer.rs` | ARM Generic Timer (PPI 30), periodic tick, frequency from `CNTFRQ_EL0` |
| Task scheduler | `task.rs` | Cooperative scheduler MVP: TCB / `create()` / `yield_now()` / `context_switch` asm / `task_exit` |

**Boot sequence**: `_start` (EL2→EL1) → `kernel_main`: BSS → UART → `trap::init()` → `gic::init()` → `irq::init()` → `timer::init()` → `task::init()` → enable IRQ → idle `yield_now()` loop

**QEMU verified**: Tasks A/B alternate + Timer tick every 10 ms + idle continues after task exit

---

## Kernel foundations and object model

### Module boundaries and dependency graph

- [ ] Split in-crate modules under `kernel/` (suggested names: `ids`, `caps`, `task`, `wait`, …) and draw **one-way dependencies** in `KERNEL.md` (forbid `kernel` → `servers`).
- [ ] Define the **public API surface** (`lib.rs` re-exports) vs **internal** visibility; avoid stabilising the wrong error types early.
- [ ] Reserve `cfg(target_arch)` / `feature` hooks for “future `hal` / `bsp` only behind arch or board shims” (may be empty at first).

### Capabilities and address-space handles

- [ ] **Capabilities**: forge-proof handles (index + generation), minimal **rights bitmap** (send, grant, read, write, …).
- [ ] **Endpoint / port IDs**: global uniqueness vs per-address-space allocation and recycle policy.
- [ ] **Root CSpace or namespace table**: rules for the first user task’s capability set at boot.
- [ ] **Revocation**: `revoke` and cascading invalidation on task exit — rules and data structures.

### Tests and invariants

- [ ] `#[cfg(test)]` coverage for **ID pools, bitmaps, generation checks**: empty, full, reuse, out-of-range, double free.
- [ ] Document **invariants** (e.g. cap index never exceeds pool size; generation monotonicity) and `panic` policy on violation.

### Boot, panic, and exception vectors

- [x] **Boot chain**: `_start` (asm) → stack / BSS → Rust `kernel_main` (or equivalent); align with `link-qemu_virt.ld` / future board scripts.
- [x] **`#[panic_handler]`**: path for formatted or minimal panic output (UART or in-memory ring).
- [x] **Vector table**: set `VBAR_ELx`; `sync` / `irq` / `fiq` / `SError` asm stubs; default hang or forward to Rust `handle_exception(reason)`.
- [x] **EL choice**: docs + code agree (e.g. long-term EL1 vs start at EL2 then drop); cross-link [BOOT.md](../BOOT.md).

---

## Scheduling and execution

### Cooperative scheduling (preferred first path)

- [x] **TCB**: states (ready, running, blocked, exited), priority field, kernel stack pointer.
- [x] **Ready queues**: FIFO within priority or multiple queues; `schedule()` / `yield()` entry points.
- [ ] **Voluntary block**: minimal wait queue wired to IPC or `wait_timeout`.

### Preemption and time

- [ ] **Preempt disable flag**: critical sections (preemption vs interrupt masking — document granularity).
- [ ] **Time slices**: fixed quantum or configurable; tie to timer IRQ.
- [ ] **Tickless**: no periodic tick when idle; wakeups from one-shot timers (doc first, code later).

### SMP

- [ ] **Boot CPU**: finish GIC, timers, global data before releasing APs.
- [ ] **AP entry**: `spin-table` or PSCI — pick one and align with real-board assumptions in [PORTING.md](../PORTING.md).
- [ ] **IPI**: spinlocks, cross-core TLB shootdown (later), migration (later).
- [ ] **Per-core idle**: `wfi` / low-power placeholder and **load balancing** (later).

### Sleep and timers

- [ ] **Relative timeouts**: kernel timer queue sorted by expiry (linked list or min-heap).
- [ ] **Wall vs monotonic** clock policy (docs); read **CNTVCT_EL0** or board timer.
- [ ] **IPC integration**: `recv` with timeout, `sleep` syscall draft.

---

## IPC and messaging

### Synchronous short messages (MVP)

- [ ] **Send / blocking recv**: small fixed payload (e.g. ≤128B) copy semantics; integrate with scheduler wait queues.
- [ ] **Call / reply**: client blocks until server `reply`; match `request_id`.
- [ ] **Timeouts and cancel**: return codes on timeout; interaction with capability revocation (documented).

### Ports and queues

- [ ] **Port object**: holds waiters for receivers; bound to a capability.
- [ ] **Queue depth cap** and **back-pressure**: `EAGAIN` vs block on full `send` — pick one and document.

### Bulk and streaming

- [ ] **Grant / map**: temporarily map sender physical pages into receiver with capability checks.
- [ ] **Ring buffer**: SPSC first; locking or lock-free MPSC (later).

### Capability transfer on IPC

- [ ] **Move** capabilities with `send` vs **duplicate** (needs `Grant` right).
- [ ] **Kernel checks**: whether the target address space may receive that cap type.

---

## Syscalls and user/kernel boundary

### ABI versioning and numbering

- [ ] In [SYSCALL_ABI.md](../SYSCALL_ABI.md): **number space**, **DRAFT-x.y**, and **STABLE-x** naming.
- [ ] **Register convention**: args in `x0–x7`, syscall number in `x8`, `ret`/`errno` model; differences vs AArch64 PCS explained.
- [ ] **Version probe syscall**: returns `ABI_VERSION` and a feature bitmask (may be all zero at first).

### `syscall_abi` crate

- [ ] Constants: `SYS_*` numbers, `MAX_ARGS`, error enum and `From<u64>`.
- [ ] **User-side wrappers** (optional sub-crate): inline asm or `no_std`-friendly stubs.

### Trap and return

- [ ] **SVC dispatcher**: one kernel entry; illegal number → `ENOSYS`.
- [ ] **User stack and TLS**: `TPIDR_EL0` or equivalent; init on thread create.
- [ ] **Trampoline**: restore **PSTATE / SP_EL0 / ELR_EL1** when returning to user mode.

### Faults and thread lifetime

- [ ] **Data / instruction abort**: user fault vs kernel bug; user fault → signal or kill thread (policy doc).
- [ ] **Illegal syscall args**: range checks; user-pointer validation (`copy_from_user` style, later).

---

## Memory and address spaces

### Physical memory

- [ ] **RAM discovery**: from FDT / static table / boot info block (QEMU vs RK3588 paths).
- [ ] **Frame allocator**: bump first → buddy; locking (coarse spinlock → per-CPU later).
- [ ] **Physical hotplug**: explicitly out of scope or one-line placeholder in docs.

### Kernel virtual memory

- [ ] **Identity or fixed-offset map**: matches linker script; device regions as **Device-nGnRnE** (or equivalent).
- [ ] **Kernel heap** (optional): `kmalloc`-style or slabs later; static pools at first.

### User address spaces

- [ ] **Address-space object**: page-table root, ASID (if used), refcount.
- [ ] **map / unmap API**: capability-bound; `PROT_READ` / `PROT_WRITE` / `PROT_EXEC` and **W^X** default (document).
- [ ] **User stack mapping**: pre-map or on-demand fault; guard page (later).

---

## QEMU and minimal runnable images

### Integration with `hawthorn_kernel`

- [x] `qemu_minimal` calls **`hawthorn_kernel::...` public API** for a second line or noop task; clear `Cargo.toml` `feature` edges.
- [ ] **Optional**: second `bin` under `examples/` for integration-only builds.

### Interrupts and time base

- [x] **GICv3** (default on `virt`) or GICv2: enable PPI **generic physical timer**; route IRQs to the current handler.
- [x] **Minimal IRQ handler**: count or placeholder "pet"; hook for future time-slice preemption.

### Device tree (FDT)

- [ ] **Parse**: `/memory` reg, `/chosen` `stdout-path`, `bootargs` (print-only OK at first).
- [ ] **vs PORTING**: table of field differences between `virt` and OPi5 DTBs (doc).

---

## Platform and BSP (RK3588 / Orange Pi 5)

### Layout and linking

- [ ] `bsp/orangepi5-rk3588/`: `README`, **`link.ld`** / **`memory.x`**, **RAM / device** placeholders aligned with `PORTING` §3.
- [ ] **Entry PA**: matches U-Boot / TF-A handoff assumptions (`ASSUME-*` ids in `BOOT.md`).

### Early hardware bring-up

- [ ] **UART**: PL011 or board debug UART per TRM/vendor; share driver code with QEMU path via `hal` abstraction.
- [ ] **GIC**: SPI numbers consistent with DT; non-`virt` IRQ map in docs.
- [ ] **Arch timer**: read `CNTFRQ` and convert to ticks.

### Clock, reset, power

- [ ] **CRU / PMU** minimal register tables: UART clock enable, bus resets (offsets from mainline Linux DT with licence note).
- [ ] **DVFS / thermal**: placeholder APIs; reading sensors later.

---

## User-space services and driver shape

### Root services and naming

- [ ] `servers/` layout: `init`, optional `name_server`; **boot order** (who starts whom) in `KERNEL.md` or `BOOT.md`.
- [ ] **First kernel→user message**: e.g. hand caps to `init` (MVP hard-coded path).

### User-space driver samples

- [ ] **PL011 service**: `DeviceMmio` cap + IRQ cap (later); polled version first.
- [ ] **virtio-console** on QEMU: virtio-mmio register layout for console (and block) — document `virt` fixed offsets.

### Middleware

- [ ] `middleware/`: placeholder **robot control message** schema (IDL or Rust types); dependency direction vs `servers/`.

---

## Robotics and productisation

### Real-time and determinism

- [ ] Define **RT0–RT3** (example names) in `ARCHITECTURE`: jitter budget, deadline miss policy.
- [ ] **Annotate critical paths**: which syscall paths must not heap-allocate, which may.

### OTA and A/B

- [ ] Boot info block fields: `slot`, `rollback`, `image_hash` placeholders and verify stubs.
- [ ] **Cooperation points** with U-Boot / vendor tools (doc-only OK at first).

### Telemetry, logging, debug

- [ ] **Kernel ring log**: size, overwrite policy, export syscall (draft).
- [ ] **Rate limiting** against user-space log floods.
- [ ] **JTAG / semihosting**: default off; security warning when enabled via `feature` or build flag.

---

## Larger follow-ups (depend on the foundations above)

### Networking and storage (mostly user space)

- [ ] **Ethernet / Wi-Fi**: driver service + protocol stack process; kernel is queues + capabilities only.
- [ ] **Block**: virtio-blk or eMMC service; VFS vs raw block interface — decide in docs.

### Virtualisation and I/O

- [ ] **Virtio** common path: mmio probe, IRQs, feature negotiation; layered under `hal`.
- [ ] **DMA coherence**: cache-maintenance ops wrapper ( AArch64 `dc cvac`, etc.) and capability rules.

### Security and trust

- [ ] **M3 / secure world**: SCM call placeholder; normal-world kernel assumptions.
- [ ] **Secure boot**: whether signature verify lives in boot or kernel — decision + stub path.

---

## Note (non-feature)

- This list **does not replace** issue prioritisation; prefix issue titles with section tags (e.g. `[IPC]`, `[BSP]`) for filtering.
- **Current milestone PR / issue order** (with GitHub links): [PR_ISSUE_PLAN.md](./PR_ISSUE_PLAN.md).
