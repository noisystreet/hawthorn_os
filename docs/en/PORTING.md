# Hawthorn / 山楂 — Porting and runtime

> **[中文](../PORTING.md)** — Chinese source of this document.

Bring-up assumptions, build entry points, and open items for **Hawthorn (山楂)** on **Orange Pi 5 / RK3588** (Tier 1). Read with [ARCHITECTURE.md](./ARCHITECTURE.md) and [KERNEL.md](./KERNEL.md).

---

## 1. Prerequisites

- Install **Rust** ([rustup](https://rustup.rs/)); [rust-toolchain.toml](../../rust-toolchain.toml) pins **stable** and **`aarch64-unknown-none`**.

Host checks:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo check -p hawthorn_kernel
  cargo check -p hawthorn_kernel --target aarch64-unknown-none
  cargo build -p hawthorn_kernel --features bare-metal --target aarch64-unknown-none
  cargo build -p hawthorn_qemu_minimal --features bare-metal --target aarch64-unknown-none
```

`rustup` installs the target per toolchain file; if missing: `rustup target add aarch64-unknown-none`.

### 1.1 Git pre-commit (optional)

Root [`.pre-commit-config.yaml`](../../.pre-commit-config.yaml) runs **`cargo fmt --check`**, **`cargo clippy --workspace -D warnings`** (same as CI), and **`commit-msg`** via **`scripts/commit_msg_bilingual.py`** (English Conventional **line 1** + **line 2** Chinese, separate lines). Install [pre-commit](https://pre-commit.com/), then `pre-commit install`. See [CONTRIBUTING.md](../../CONTRIBUTING.md) and [COMMIT_CONVENTIONS.md](./COMMIT_CONVENTIONS.md) §1.0.

### 1.2 QEMU `virt` minimal image (optional)

The linker script **`kernel/link-qemu_virt.ld`** (RAM **`0x4000_0000`** / 128 MiB, **`__stack_top`**, BSS symbols) is shared by **`hawthorn_kernel`** and **`hawthorn_qemu_minimal`** so the layout stays single-sourced.

- **`hawthorn_kernel`** ([`kernel/`](../../kernel/)): with **`--features bare-metal`** and **`--target aarch64-unknown-none`**, builds the bare-metal binary **`hawthorn_kernel_qemu_virt`** (**`_start` → `kernel_main`**, PL011 **`0x9000_0000`**, panic uses the same UART). Check: `cargo check -p hawthorn_kernel --target aarch64-unknown-none` (library); full image: `cargo build -p hawthorn_kernel --features bare-metal --target aarch64-unknown-none`.
- **`hawthorn_qemu_minimal`** ([`qemu_minimal/`](../../qemu_minimal/)): same target + feature for a standalone smoke ELF. Without **`bare-metal`**, both crates build stub libraries on the host so **`cargo clippy --workspace`** stays fast.

- **Build (example):** `cargo build -p hawthorn_qemu_minimal --features bare-metal --target aarch64-unknown-none` (for release, set `PROFILE=release` when using the script below).
- **Run:** install **`qemu-system-aarch64`**, then run [`scripts/run_qemu_minimal.sh`](../../scripts/run_qemu_minimal.sh); the script prints progress to **stderr** before QEMU starts (so it does not look “stuck”). Equivalent CLI: **`-machine virt -cpu cortex-a76 -nographic -kernel <ELF>`** (`-nographic` wires the UART to stdio; **do not** also pass **`-serial stdio`** or chardevs may fight over stdio).
- **Self-check (recommended):** [`scripts/verify_qemu_minimal.sh`](../../scripts/verify_qemu_minimal.sh) runs **fmt / workspace clippy / bare-metal clippy+build / short QEMU**, and asserts **`[hawthorn]`** progress lines appear. If the guest UART line is not visible in captured output, the script still reports that the Rust + script stages passed (common in some CI/pipe setups).
- **Note:** in **non-interactive** pipelines you may not see guest serial lines; run the script in a **real TTY**. If there is still no output, try **`-serial pty`** and **`cat /dev/pts/N`** on the PTY path QEMU prints.

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
