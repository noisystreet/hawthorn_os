# Hawthorn / 山楂 — Testing strategy & layers

> **[中文](../测试.md)** — Chinese source of this document.

This document defines **test layering** for the Hawthorn repo, how it maps to CI, and conventions for bare-metal / QEMU end-to-end checks. Kernel behavior and module boundaries: [KERNEL.md](./KERNEL.md), [ARCHITECTURE.md](./ARCHITECTURE.md).

---

## 1. Layer overview

| Layer | Purpose | Typical location | Where it runs |
|--------|---------|------------------|---------------|
| **L1 Unit** | Single-module invariants, pure logic, data structures | `#[cfg(test)] mod tests { ... }` inside each crate | Host (`cargo test`) |
| **L2 Integration** | **Public API** of a crate across modules, link smoke | `kernel/tests/*.rs`, etc. — crate-root `tests/` | Host |
| **L3 E2E (bare metal / QEMU)** | Boot, `trap`, serial output, EL0 paths | `scripts/verify_kernel_qemu_virt_*.sh` | Linux + QEMU + `socat` (same as CI) |
| **L4 HIL** | Board timing, drivers, real IRQs | **Planned**; scripts/gates when Tier-1 hardware joins CI or manual regressions | Target hardware |

**Principles**

- **Match CI:** before merge, pass the same checks as [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml) locally (including QEMU scripts when boot/serial paths change).  
- **Default: no `cargo test` on `aarch64-unknown-none`:** bare-metal targets use `cargo check` / `cargo build`; keep runnable logic on the host via `#[cfg(test)]` or portable submodules where possible.  
- **E2E assertions:** QEMU scripts judge success via **serial substrings** (and exit codes); update expected strings when milestone output changes.

---

## 2. L1: Unit tests

- **Kernel crate (`hawthorn_kernel`):** `no_std` when `not(test)`; `cargo test -p hawthorn_kernel` builds testable modules on **host** (e.g. `task_policy`, `endpoint`, `frame_alloc`, `trap_frame`). Modules tied to privilege/board are only built for `target_arch = "aarch64", target_os = "none"` — **unit tests do not replace** L3 QEMU runs.  
- **ABI crate (`hawthorn_syscall_abi`):** constants, encodings, and simple properties covered on the host.  
- **`qemu_minimal`:** add `#[cfg(test)]` as needed (may stay thin).

**Conventions**

- Prefer extracting pure policy into **host-buildable** modules for L1 (see `task_policy.rs`).  
- `unsafe` contracts: unit tests + documented invariants; **Miri** optional later, not required in CI by default.

---

## 3. L2: Integration tests

- **Location:** `kernel/tests/*.rs` (crate-root `tests/`, distinct from in-`src` unit tests).  
- **Role:** import `hawthorn_kernel` as a library via **public** items; ensure linking and gating; **do not** duplicate all L1 coverage.  
- **Future:** when `servers/` or user-side `syscall_abi` crates grow, add host integration tests per crate; cross-process / real IPC belongs in L3/L4.

---

## 4. L3: QEMU E2E

| Script | Rough coverage |
|--------|----------------|
| [`scripts/verify_kernel_qemu_virt_serial.sh`](../../scripts/verify_kernel_qemu_virt_serial.sh) | `hawthorn_kernel_qemu_virt` image boot, PL011 expected output |
| [`scripts/verify_kernel_qemu_virt_el0_serial.sh`](../../scripts/verify_kernel_qemu_virt_el0_serial.sh) | Regression including **EL0 / user** path (per kernel user-task bootstrap contract) |

**Local run** (needs `qemu-system-aarch64`, `socat`; see CI `install qemu test deps`):

```bash
bash scripts/verify_kernel_qemu_virt_serial.sh
bash scripts/verify_kernel_qemu_virt_el0_serial.sh
```

Optional: `PROFILE=release` for release builds.

When changing **boot flow, early prints, user entry, ABI probe output**, update expected substrings in these scripts and re-run both.

---

## 5. L4: HIL & beyond

- **RK3588 / Orange Pi 5:** behavior that QEMU cannot model (timers, GIC, DMA, …) is tracked via an **HIL checklist** (issues, release notes, or future `scripts/` per board); automate when self-hosted CI or lab gates exist.  
- **Contract tests:** syscall numbers, `abi_info` bits — keep [SYSCALL_ABI.md](./SYSCALL_ABI.md) aligned with unit/integration tests; major changes benefit from an ADR if `docs/adr/` is adopted.

---

## 6. AGENTS & contributing

- Local verification: [AGENTS.md](../../AGENTS.md) §4 — **`cargo test --workspace`** covers L1 + per-crate L2; the two QEMU scripts cover L3.  
- When adding tests, label the layer (§1 table) and avoid pushing all logic into bash or QEMU-only checks.

---

## Document maintenance

- This file pairs with [ARCHITECTURE.md §5.3](./ARCHITECTURE.md). When CI jobs change (new jobs, QEMU skipped), update **§4** and [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml) together.
