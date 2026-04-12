# Hawthorn / 山楂 — Porting and runtime

> **[中文](../PORTING.md)** — Chinese source of this document.

Bring-up assumptions, build entry points, and open items for **Hawthorn (山楂)** on **Orange Pi 5 / RK3588** (Tier 1). Read with [ARCHITECTURE.md](./ARCHITECTURE.md) and [KERNEL.md](./KERNEL.md).

---

## 1. Prerequisites

- Install **Rust** ([rustup](https://rustup.rs/)); [rust-toolchain.toml](../../rust-toolchain.toml) pins **stable** and **`aarch64-unknown-none`**.

Host checks:

```bash
cargo fmt --all -- --check
cargo clippy -p hawthorn_kernel --all-targets --all-features
cargo check -p hawthorn_kernel
cargo check -p hawthorn_kernel --target aarch64-unknown-none
```

`rustup` installs the target per toolchain file; if missing: `rustup target add aarch64-unknown-none`.

### 1.1 Git pre-commit (optional)

Root [`.pre-commit-config.yaml`](../../.pre-commit-config.yaml) runs **`cargo fmt --check`**, **`cargo clippy -D warnings`** (same as CI), and **`commit-msg`** via **`scripts/commit_msg_bilingual.py`** (English Conventional **line 1** + **line 2** Chinese, separate lines). Install [pre-commit](https://pre-commit.com/), then `pre-commit install`. See [CONTRIBUTING.md](../../CONTRIBUTING.md) and [COMMIT_CONVENTIONS.md](./COMMIT_CONVENTIONS.md) §1.0.

---

## 2. Boot chain (TBD)

Open decisions: [ARCHITECTURE.md §10](./ARCHITECTURE.md) (image format, TF-A / U-Boot, FDT). Before implementation, fix in `bsp/orangepi5-rk3588/` and [BOOT.md](./BOOT.md):

- **U-Boot** (or Rockchip flow) loads **Hawthorn (山楂)** image with **entry EL**, **entry PA**, **DTB pointer**; or  
- A **minimal custom stub** and TF-A handoff contract.

---

## 3. Memory map (placeholder)

| Region | Notes |
|--------|--------|
| Kernel image | Load address / reloc: **TBD** (boot-chain dependent) |
| DT / board info | Pointer via boot block: **TBD** |
| Early stack & BSS | From `bsp/orangepi5-rk3588/` linker script |

When fixed, update here or **BOOT.md** and link `linker.ld`.

---

## 4. Related documents

| Doc | Content |
|-----|---------|
| [BOOT.md](./BOOT.md) | Boot info block & phases (skeleton) |
| [SYSCALL_ABI.md](./SYSCALL_ABI.md) | Syscall ABI (skeleton) |
| [PLATFORMS.md](./PLATFORMS.md) | Platform tiers |
| [GLOSSARY.md](./GLOSSARY.md) | Glossary |
| [API.md](./API.md) | Public API index (placeholder) |

---

## 5. Public API

Stable user/kernel boundary: future `syscall_abi` crate + [SYSCALL_ABI.md](./SYSCALL_ABI.md); crate-level index: [API.md](./API.md).
