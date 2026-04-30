# Hawthorn / 山楂 — Architecture

> **[中文](../架构.md)** — Chinese source of this document.

This document describes goals, layering, major subsystems, and roadmap for **Hawthorn**, a **Rust** embedded OS (**Chinese name: 山楂**; code name **hawthorn**), aimed at **robotics** and **smart hardware** that need predictability, isolation, and maintainability.

**Kernel shape:** Hawthorn is a **microkernel** — scheduling, address-space primitives, capabilities, IPC, interrupt/exception delivery, and timekeeping stay in-kernel; drivers, network stacks, and file systems run as **user services** over IPC. **Kernel modules, object model, and syscall boundary:** [KERNEL.md](./KERNEL.md).

**Tier-1 hardware:** **Orange Pi 5**, SoC **Rockchip RK3588**. Kernel and BSP target this board first; other platforms get tier lists later. Details: **§2.5** below.

---

## 1. Goals and non-goals

### 1.1 Goals

- **Deterministic real-time:** predictable scheduling and interrupt latency for motion control and sensor fusion.  
- **Memory & fault isolation:** layered services; microkernel **process-level** isolation for drivers/stacks; contain single-service faults.  
- **Rust-first:** kernel and user hot paths in Rust; types + bounded `unsafe` for memory/concurrency risks.  
- **Clear hardware abstraction:** board/SoC differences in HAL/BSP; **drivers live in user services**; HAL supplies register contracts and shared types where needed.  
- **Configurable:** enable services/middleware (network, FS, graphics, …); **long term** the spectrum may span MCU through high-end MPU. **Current engineering and Tier-1** focus on **Orange Pi 5 / RK3588 (AArch64 + MMU)** — see [PLATFORMS.md](./PLATFORMS.md).

### 1.2 Non-goals (not promised initially)

- Full Linux-compatible POSIX subset (optional partial alignment later).  
- Desktop-class graphics and multi-user time-sharing.  
- Replacing every incumbent RTOS; emphasis is **in-house control + unified Rust stack**.

---

## 2. Design principles

| Principle | Meaning |
|-----------|---------|
| Layering & dependency inversion | Upper layers depend on abstractions; **stable syscall/ABI** is the kernel/user boundary. |
| Least privilege | Tight defaults; capabilities grant MMIO/DMA/IRQ explicitly. |
| No hidden globals | Kernel objects and handles passed explicitly; easier testing and reasoning. |
| Real-time over throughput | Scheduling and IPC **fast paths** bounded by worst-case latency (see KERNEL doc). |
| Observable | Unified tracing, metrics, and crash hooks for field and lab use. |

### 2.1 Advanced ideas worth adopting (delivery-oriented)

1. **Capability-first security model**: object handles + least privilege, instead of identity/path-centric access.
2. **User-space driver services**: keep mechanisms in kernel; move protocol/policy to restartable, isolated services.
3. **IPC-first kernel interface**: prefer message/endpoint semantics and avoid syscall surface bloat.
4. **Minimal TCB**: keep the trusted base small (scheduler, mapping, IPC, capability checks) for auditability.
5. **Policy/mechanism split**: kernel provides mechanisms; policy stays in user services.
6. **Explicit resource lifecycle**: every object must have create/use/revoke/reclaim semantics.
7. **Determinism-first paths**: bound dynamic allocation and uncontrolled locks on critical paths.
8. **Built-in structured observability**: design trace/metrics/error-codes with features, not after incidents.
9. **ABI versioning from day one**: syscall/IPC protocols carry version and capability bits.
10. **Progressive complexity**: deliver verifiable MVPs first, then scale scope.

### 2.2 Existing OS limitations to avoid

- **All drivers in kernel**: driver bugs widen blast radius to whole system failures.
- **Excessive global mutable state**: concurrent behavior becomes hard to reason about.
- **Fragmented permission model**: ACL/UID/namespace layering becomes difficult to audit.
- **Syscall surface inflation**: interface maintenance and compatibility debt explode over time.
- **Poor error recovery model**: recoverable faults escalate to node-level failures.
- **No lifecycle closure**: handle/page-table/frame leaks appear in long-running systems.
- **Observability added too late**: debugging and performance diagnosis become expensive.

### 2.5 Tier-1 platform (Orange Pi 5 / RK3588)

| Item | Description |
|------|-------------|
| Board | [Orange Pi 5](https://www.orangepi.org/html/hardWare/computerAndMicrocontrollers/details/Orange-Pi-5.html) |
| SoC | **RK3588**: **big.LITTLE** octa-core (4× Cortex-A76 + 4× Cortex-A55), **AArch64** / ARMv8-A |
| Privilege | Typical **EL1** kernel + **EL0** user services; exact EL contract with bootloader / **TF-A** is fixed in BSP |
| Memory & I/O | **MMU** paging, device **MMIO**; DMA **cache coherency** via HAL + memory/driver services (RK3588 details per TRM) |
| Interrupts | **GIC** (variant/routing per datasheet), SPI/PPI/SGI wired to kernel `irq` |
| Time | **ARM Generic Timer** for tick and monotonic clock |
| Rust target | Bare metal: `aarch64-unknown-none` or `aarch64-unknown-none-softfloat` (unify FP ABI); see `rust-toolchain.toml` and `.cargo/config.toml` |
| BSP path | `bsp/orangepi5-rk3588/` (clocks, reset, debug UART, memory layout, device table entry) |

**Note:** RK3588 is rich (PCIe, GMAC, USB3, NPU, …). **M0/M1** can trim to **UART + timers + minimal interrupt glue**; other controllers come up in user services by priority. How **Rockchip boot chain** (e.g. U-Boot loading the kernel image) connects is an open item under **§10** below.

---

## 3. Logical layering

```
┌─────────────────────────────────────────────────────────────┐
│  Applications: motion planning, SLAM, policies, OTA, …      │
├─────────────────────────────────────────────────────────────┤
│  Middleware: message bus, params, logging, time sync, …      │
├─────────────────────────────────────────────────────────────┤
│  System services: device mgr, power, watchdog policy, net …   │
├─────────────────────────────────────────────────────────────┤
│  User services: drivers, bus hosts, FS, network stacks, init   │
│                 (IPC to kernel — see KERNEL.md)                │
├─────────────────────────────────────────────────────────────┤
│  Microkernel: sched, caps, IPC, AS/map, IRQ delivery, time, …  │
├─────────────────────────────────────────────────────────────┤
│  HAL / board: register contracts, boot, linker scripts, glue   │
└─────────────────────────────────────────────────────────────┘
```

**Typical robotics data path:** IRQ → kernel **delivers** to driver service or high-priority thread → shared buffers + IPC/notification → control loop → actuator service; telemetry/OTA use separate services with **priority and quotas** vs hard-RT paths.

---

## 4. Kernel summary (details in KERNEL.md)

The microkernel is **policy-light mechanism**: threads/scheduling, capability table, synchronous IPC, address-space switch, traps, ticks. Everything else composes in **user services**.

| Topic | Summary |
|-------|---------|
| Isolation | Process/task + private AS; capabilities name objects/resources. |
| Drivers | In user space; kernel does not parse device protocols — mapping, IRQ bind, IPC only. |
| Real-time | FP preemptive scheduling; IPC + IRQ path analyzable; [KERNEL.md §5](./KERNEL.md). |

**Boot:** Bootloader → kernel early init → root task → **init** starts drivers and services (contract in KERNEL doc).

---

## 5. Rust stack & engineering

### 5.1 Language subset

- Kernel / `no_std`: **no implicit heap** on hard-RT paths; centralize `unsafe` with invariants.  
- User boundary: `core` + optional `alloc`; FFI via **`cbindgen`/`bindgen`** with safe wrappers. User/kernel split by **stable ABI**.

### 5.2 Build & targets

- **Tier-1 triple:** `aarch64-unknown-none` or `aarch64-unknown-none-softfloat` (match FP ABI) for **Orange Pi 5 / RK3588**.  
- **Other platforms:** tier 2/3 (e.g. `thumbv7em-none-eabihf`, `riscv32imac-unknown-none-elf`) — not mixed into default features with Tier-1 BSP.  
- **Cargo features:** subsystem-sized; avoid linking everything.

### 5.3 Test & quality

Full layering (**L1 unit / L2 integration / L3 QEMU E2E / L4 HIL**), layout, and CI mapping: **[TESTING.md](./TESTING.md)**. Summary:

- **L1 (unit):** `#[cfg(test)]` inside each crate; `hawthorn_kernel` runs `cargo test` on the host for portable subsets (`no_std` only when `not(test)`).  
- **L2 (integration):** crate-root `tests/*.rs` for **public API** linking and composition; keep detail in L1.  
- **L3 (E2E):** `scripts/verify_kernel_qemu_virt_*.sh` on QEMU `virt` + PL011 assert serial output; matches [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml).  
- **L4 (HIL):** Tier-1 board/timing via checklist/future scripts; **Miri** optional for some `unsafe` contracts (not default CI).  
- **Static:** Clippy (incl. cognitive complexity cap), `unsafe` audit checklist, optional MISRA-style mapping.

---

## 6. Security & reliability

- **Secure boot:** root of trust → bootloader → signed kernel/images.  
- **OTA:** A/B, version negotiation, power-fail-safe writes, rollback.  
- **Resilience:** tiered watchdogs, explicit timeouts in state machines; microkernel may **restart one failed service** (policy-defined).  
- **Resource caps:** stack depth, IPC queue depth, capability derivations — anti-DoS defaults.

---

## 7. Mapping to robotics scenarios

| Need | Architecture hook | Milestone (rough) |
|------|---------------------|-------------------|
| 1–10 kHz joint/current loops | High-priority threads, HW timers, short IPC/shared regions; drivers in services | M0: sched/timer path; M2–M3: hard numbers |
| IMU / lidar bulk data | DMA + double buffering; maps/caps in memory/driver services; kernel validates/delivers | M1+ when mapping exists |
| Multi-process / “high-end” | AS + capabilities; supervision & quotas in services | M1 multi-service; M2 isolation/observability |
| CAN-FD / EtherCAT | Dedicated driver services + IRQ budget vs scheduler | M1+ per board I/O |
| Functional safety (if pursued) | Small TCB + documented partitioning; redundancy/diagnostics split kernel vs services | M3 with audit / WCET docs |

---

## 8. Repository layout (planned)

```
hawthorn/                  # repo root (Chinese: 山楂; code name hawthorn)
├── docs/
├── kernel/                # microkernel (see KERNEL.md)
├── servers/               # user: drivers, init, device mgr, stacks, …
├── hal/
├── bsp/orangepi5-rk3588/
├── middleware/
├── examples/
├── syscall_abi/           # optional syscall numbers / stubs
└── tools/
```

**Dependencies:** `kernel` **must not** depend on `servers`; user code uses **syscall + ABI crate** only. `examples` → `middleware` → `servers` → (ABI only) → `hal` / `bsp` — no user crate links private kernel symbols. Main kernel crate: **`hawthorn_kernel`** (`kernel/Cargo.toml`).

---

## 9. Roadmap (suggested)

1. **M0:** RK3588 **single core** (or fixed small core set), microkernel + minimal IPC + root task + **UART** + Generic Timer demo; driver-service sketch.  
2. **M1:** Capability sketch, **MMU** mappings + user AS, multi-service; one or two buses (e.g. **SPI/I2C** per board).  
3. **M2:** **SMP** (A76+A55 subset) or **AMP** sketch, **IPI**, global/per-CPU scheduling v0; network/OTA in services, observability, **IPC fast path** measured on RK3588.  
4. **M3:** Secure boot, **MMU** audit hardening, WCET docs on RK3588, **hot service restart** policy fixed.

---

## 10. Open decisions (resolve before coding)

- **Boot & image format:** U-Boot (or Rockchip flow) loading Hawthorn image, entry, **FDT** mainline vs board dtb; **TF-A** EL handoff.  
- **Memory service model:** full user pager vs simplified in-kernel frame book — see [KERNEL.md §2.2](./KERNEL.md).  
- **big.LITTLE scheduling:** single domain vs migration A76↔A55; **PSCI** vs real-time threads.  
- **Tickless** / low-power impact on RT.  
- **Embassy / Tock / Zephyr:** reuse HAL/driver shapes vs all-custom services.

---

## Document maintenance

- Update [KERNEL.md](./KERNEL.md), [BOOT.md](./BOOT.md), [SYSCALL_ABI.md](./SYSCALL_ABI.md) when kernel objects or ABI change.  
- **Porting:** [PORTING.md](./PORTING.md); **tiers:** [PLATFORMS.md](./PLATFORMS.md); **glossary:** [GLOSSARY.md](./GLOSSARY.md); **API index:** [API.md](./API.md).  
- Review axes: **WCET**, **memory budget**, **boot timing**, **recovery**.  
- Style / commits: [CODE_STYLE.md](./CODE_STYLE.md), [COMMIT_CONVENTIONS.md](./COMMIT_CONVENTIONS.md); testing: [TESTING.md](./TESTING.md); contributing / security: [CONTRIBUTING.md](../../CONTRIBUTING.md), [SECURITY.md](../../SECURITY.md).
