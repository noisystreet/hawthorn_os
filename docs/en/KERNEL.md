# Hawthorn / 山楂 — Microkernel: modules and design

> **[中文](../KERNEL.md)** — Chinese source of this document.

Internal module split, object model, interface boundaries, and real-time notes for the **Hawthorn (山楂) microkernel**. System layering: [ARCHITECTURE.md](./ARCHITECTURE.md). Assumes familiarity with “small kernel + user services”.

---

## 1. Document scope

| Topic | This doc | Architecture overview |
|-------|------------|-------------------------|
| Why microkernel & product fit | Brief | Detailed |
| In-kernel subsystems & interaction | **Detailed** | Summary |
| Where drivers / net / FS live | **User services** + kernel contract | Diagram |

---

## 2. Goals and boundaries

### 2.1 Goals

- **Minimal TCB:** code that can read/write arbitrary memory or bypass isolation stays **small and auditable**.  
- **Fault isolation:** driver/stack crashes bounded by **process/AS**; restart one service without full reset (policy in a supervisor).  
- **Unified IPC:** user↔kernel and service↔service share one **message primitive** set (small orthogonal extensions OK).  
- **Predictable real-time:** **short paths** for hard-RT threads — scheduling, IPC fast path, IRQ→thread latency analyzable.

### 2.2 In-kernel vs out-of-kernel

**In kernel:**

- Threads + **preemptive scheduling** (+ basic priority policy).  
- **Address spaces** and **mapping** setup/switch (MPU/MMU per platform).  
- **Capabilities** (or equivalent): create, transfer, revoke, check rights.  
- **IPC:** endpoints, synchronous messages, optional async notify/semaphore minimal set.  
- **Interrupts/exceptions:** hardware entry and **delivery** to threads/services (no device protocol parsing).  
- **Ticks / timers** infrastructure (wall vs monotonic may involve user time service).  
- **Syscall entry** and **trap** context switch.

**In user services:**

- **Device drivers** (GPIO, UART, DMA, CAN, Ethernet MAC, …).  
- **Protocol stacks** (TCP/IP, USB, …) and **file systems**.  
- **Enumeration, power policy, OTA business logic**, … (split across services as needed).

**Gray areas (must be fixed per port):**

- **MMIO windows:** kernel maps and hands caps to drivers vs user pager maps — document one model in the porting guide.

### 2.3 Tier-1 hardware (Orange Pi 5 / RK3588)

Board **Orange Pi 5**, SoC **RK3588**. Align kernel modules with (details in **RK3588 TRM** + schematics):

| Area | Impact on kernel |
|------|------------------|
| **ISA** | **AArch64:** `trap`/`syscall` via **SVC/HVC/SMC**; SIMD/FP save per ABI / EL0 policy. |
| **MMU** | `mem` uses **page tables** (granule, TG0/TG1) for **EL0/EL1**; not Cortex-M MPU-first. |
| **IRQ** | `irq` talks **GIC** (SPI/PPI/SGI); SMP: **GIC redistributor** per core, **IPI** via SGI or equivalent. |
| **Time** | `time` binds **ARM Generic Timer** (`CNTPCT` / `CNTVCT`); tick/one-shot cooperates with `sched`. |
| **Multi-core** | `sched` plans **SMP** (4×A76 + 4×A55): topology, migration, **big.LITTLE** policy with product RT goals. |
| **DMA / coherency** | Device DMA + **D-cache** maintenance in **HAL + memory/driver services**; `mem` avoids **incoherent** CPU/device views. |

Bootloader / **TF-A** hands **EL**, entry PC, DT pointer, … as a **versioned boot info block** in `boot` + `bsp/orangepi5-rk3588/`, aligned with [ARCHITECTURE.md §2.5](./ARCHITECTURE.md).

---

## 3. Kernel module split

Logical modules (map to `kernel/` crates or submodules: `sched`, `ipc`, `mem`, `syscall`, …).

### 3.1 Boot (`boot`)

- **Contract:** bootloader passes **boot info block** (RAM layout, reserved regions, DT/board blob pointer, boot slot, …), versioned struct + magic.  
- **Phases:** minimal **interrupts-off** sequence (stack, BSS, CPU features, **MMU** min bring-up; **MPU** if present) → first kernel thread + **root capability space** → schedulable context. RK3588: **EL1 page tables**; see **§2.3**.  
- **First user task:** **init** (or equivalent) loads other services (naming, drivers, …).

**QEMU `virt` minimal bring-up (M2, current):** AArch64 bare-metal smoke lives in **`hawthorn_kernel`**. Entry **`_start`** (`.text.boot`), if QEMU enters at EL2 it drops to EL1 with MMU disabled and TLB flushed; initial **`SP = __stack_top`** from [`kernel/link-qemu_virt.ld`](../../kernel/link-qemu_virt.ld): **`RAM`** at **`0x4000_0000`**, **128 MiB**, **`__stack_top = ORIGIN(RAM) + LENGTH(RAM) - 16`**. Assembly **`bl kernel_main`**; **`kernel_main`** is in `hawthorn_kernel::boot_qemu_virt` (short interrupts-off sequence: zero BSS → **PL011 @ `0x9000_0000`** → **exception vector table** `trap::init()` → **GICv3** `gic::init()` → **IRQ dispatch** `irq::init()` → print banner). Build the **`hawthorn_kernel_qemu_virt`** binary with **`cargo build -p hawthorn_kernel --features bare-metal --target aarch64-unknown-none`**; **`hawthorn_qemu_minimal`** shares that linker script (see [PORTING.md](./PORTING.md) §1.2). **Panic** prints **`hawthorn_kernel: panic\n`** on the same UART (`#[panic_handler]` in the bin crate root). **EL:** `_start` explicitly handles EL2→EL1 downgrade, ensuring execution at EL1 with MMU off.

### 3.2 Objects (`objects`)

Suggested minimal object types (names may vary):

| Object | Role |
|--------|------|
| `Thread` | Execution context, priority, stack, register save area. |
| `Task` / `Process` | Optional grouping: threads share AS + cap table. |
| `AddressSpace` | ASID/domain; mapping table or page-table root. |
| `Endpoint` / `Port` | IPC endpoint; rights/quotas on capabilities. |
| `Notification` / `IrqControl` | IRQ completion ↔ thread wake (pick or combine). |
| `CNode` / cap table | Store and derive capabilities. |

**Derivation:** narrowed caps (e.g. read-only map); revoke = cascade vs refcount — **pick and document**.

### 3.3 Scheduling (`sched`)

- **Policy:** single-core first = **fixed-priority preemptive**; same priority **FIFO** or **round-robin** configurable.  
- **Ready queues:** priority buckets; no heavy scheduling inside ISR — **set flag / enqueue** for kernel thread / tail work.  
- **Migration (SMP):** document **worst-case migration** and lock hold time for pinned vs global queues.  
- **IPC interaction:** blocking recv → **blocked**; non-blocking send / timeouts coordinated with IPC.

### 3.4 IPC (`ipc`)

- **Sync messages:** `call`/`reply` or rendezvous; **bounded** payload (registers + optional cap-authorized shared pages).  
- **Capability transfer:** move or copy slots with messages — no forged handles.  
- **Fast path:** no extra copies/allocs on hot path; cache-friendly layout.  
- **Backpressure:** queue depth, outstanding `reply` caps — anti-DoS; matches resource caps in [ARCHITECTURE.md](./ARCHITECTURE.md).

### 3.5 Memory (`mem`)

- **Kernel image:** static or **one-shot** boot alloc; no general-purpose heap on hot paths.  
- **User AS:** map/unmap; **DMA coherency** hooks (flush / non-cacheable) when implemented in a **memory manager service** — kernel supplies **mapping primitives** + **cap checks**.  
- **Physical frames:** simplified in-kernel vs **pager service**; **W^X** and **double-map** policy must be explicit.

### 3.6 IRQ / trap (`irq` / `trap`)

- **Vectors:** save context, **ack/classify**, attach event to **Notification** or **IPC target**.  
- **RK3588:** **GIC**; SMP: **SPI target CPU** vs load balancing — kernel configures vs driver holds caps: **decide and document**.  
- **Bottom halves:** no long logic in ISR; wake high-priority thread or **message** driver service.  
- **Stacks / nesting:** document **per-exception stacks** and max nesting depth.

### 3.7 Time (`time`)

- **Tick source:** HW timer + optional tickless impact on **sched clock**.  
- **Timeouts:** relative/absolute on IPC/blocking; clock skew policy.  
- **Userland:** NTP/PTP in services; kernel may expose **read clock** + **timer objects** if in minimal object set.

### 3.8 Syscalls (`syscall`)

- **Stable ABI:** syscall numbers, reg convention, error enum; user stubs share headers or codegen with kernel path.  
- **Validation:** user pointers checked with **capability + range**; avoid **TOCTOU** (copy-in or HW protection).  
- **Debug:** optional trace points / slow-path logs (compile-time gated).

---

## 4. Contract with user services

- **init:** initial caps; **service registration, restart policy, watchdog** coordination.  
- **Driver services:** **IPC** “device sessions” to clients; DMA buffers via **cap-authorized maps** or **shared pages**.  
- **Naming:** name→endpoint in a **name service**; kernel may expose **boot endpoints** in a static table.

**Dependency:** user services **do not link** kernel symbols — only **syscall + stable ABI** (fuzzable, swappable kernel).

---

## 5. Real-time (microkernel context)

- **Control loops:** joint/current loops on **high-priority threads**; driver IPC = **short messages** or **pre-mapped shared + signal** — avoid big copies.  
- **IPC latency:** keep critical path within analyzable instruction + cache-miss budget; optional **fast path** per syscall class.  
- **Isolation / preemption:** low-priority services must not **disable preemption** for long; **partitioned** or **lock-free** kernel structures to limit priority inversion.

---

## 6. Testing suggestions

- **Scheduler + IPC state machines:** unit/property tests on host with mocked HW.  
- **Syscall fuzzing:** harden kernel against random user sequences.  
- **WCET:** measure + document at least Tier-1 boards per release.

---

## 7. Repo mapping (suggested)

```
kernel/
├── boot/
├── objects/
├── sched/
├── ipc/
├── mem/
├── irq/
├── time/
└── syscall/
```

User services under `servers/` (or equivalent) — **no** reverse Cargo dependency from `kernel/`.

---

## Related documents

- [Architecture](./ARCHITECTURE.md)
- [Boot skeleton](./BOOT.md)
- [Syscall ABI](./SYSCALL_ABI.md)
- [Code style](./CODE_STYLE.md)
- [Commit conventions](./COMMIT_CONVENTIONS.md)
