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
cargo test --workspace
cargo check -p hawthorn_kernel
  cargo check -p hawthorn_kernel --target aarch64-unknown-none
  cargo build -p hawthorn_kernel --features bare-metal --target aarch64-unknown-none
  cargo build -p hawthorn_qemu_minimal --features bare-metal --target aarch64-unknown-none
```

`rustup` installs the target per toolchain file; if missing: `rustup target add aarch64-unknown-none`.

### 1.1 Git pre-commit (optional)

Root [`.pre-commit-config.yaml`](../../.pre-commit-config.yaml) runs **`cargo fmt --check`**, **`cargo clippy --workspace -D warnings`**, **`cargo test --workspace`** (same as CI), and **`commit-msg`** via **`scripts/commit_msg_bilingual.py`** (English Conventional **line 1** + **line 2** Chinese, separate lines). Install [pre-commit](https://pre-commit.com/), then `pre-commit install`. See [CONTRIBUTING.md](../../CONTRIBUTING.md) and [COMMIT_CONVENTIONS.md](./COMMIT_CONVENTIONS.md) §1.0.

### 1.2 QEMU `virt` minimal image (optional)

The linker script **`kernel/link-qemu_virt.ld`** (RAM **`0x4000_0000`** / 128 MiB, **`__stack_top`**, BSS symbols) is shared by **`hawthorn_kernel`** and **`hawthorn_qemu_minimal`** so the layout stays single-sourced.

- **`hawthorn_kernel`** ([`kernel/`](../../kernel/)): with **`--features bare-metal`** and **`--target aarch64-unknown-none`**, builds the bare-metal binary **`hawthorn_kernel_qemu_virt`** (**`_start` → `kernel_main`**, PL011 **`0x9000_0000`**, panic uses the same UART). Check: `cargo check -p hawthorn_kernel --target aarch64-unknown-none` (library); full image: `cargo build -p hawthorn_kernel --features bare-metal --target aarch64-unknown-none`.
- **`hawthorn_qemu_minimal`** ([`qemu_minimal/`](../../qemu_minimal/)): same target + feature for a standalone smoke ELF. Without **`bare-metal`**, both crates build stub libraries on the host so **`cargo clippy --workspace`** stays fast. The stub **`no_std`** library has no unit tests, and building a host lib test harness **SIGSEGV**s, so that crate sets **`[lib] test = false`** in **`Cargo.toml`**. **`cargo test --workspace`** still covers the rest of the workspace; smoke coverage remains AArch64 builds + QEMU scripts.

- **Build (example):** `cargo build -p hawthorn_qemu_minimal --features bare-metal --target aarch64-unknown-none` (for release, set `PROFILE=release` when using the script below).
- **Run:** install **`qemu-system-aarch64`**, then run [`scripts/run_qemu_minimal.sh`](../../scripts/run_qemu_minimal.sh); the script prints progress to **stderr** before QEMU starts (so it does not look "stuck"). Equivalent CLI: **`-machine virt,gic-version=3 -cpu cortex-a76 -nographic -kernel <ELF>`** (`-nographic` wires the UART to stdio; **do not** also pass **`-serial stdio`** or chardevs may fight over stdio). **`gic-version=3` is mandatory**: QEMU `virt` defaults to GICv2, but the kernel GIC driver only supports GICv3.
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

## 4. QEMU `virt` GICv3 Integration Key Points

Critical bugs encountered during GICv3 driver and IRQ dispatch framework integration, documented here for reference.

### 4.1 QEMU `virt` Default GIC Version

**Problem**: QEMU `virt` machine defaults to **GICv2** (`compatible = "arm,cortex-a15-gic"`). GICv2 has no Redistributor (GICR); GICD @ `0x0800_0000`, GICC @ `0x0801_0000`. The kernel GIC driver is written for GICv3, so accessing GICR @ `0x080A_0000` causes a **Translation fault (level 0)**.

**Fix**: QEMU must be run with **`-machine virt,gic-version=3`**. Both [`scripts/run_qemu_minimal.sh`](../../scripts/run_qemu_minimal.sh) and [`scripts/verify_kernel_qemu_virt_serial.sh`](../../scripts/verify_kernel_qemu_virt_serial.sh) have been updated.

### 4.2 GICv3 Redistributor SGI Base Page

**Problem**: Each GICv3 Redistributor consists of **two 64 KiB pages**:
- **RD base page** (`GICR_BASE + 0x0000_0000`): control registers (GICR_CTLR, GICR_TYPER, GICR_WAKER)
- **SGI base page** (`GICR_BASE + 0x0001_0000`): SGI/PPI configuration registers (GICR_IGROUPR0, GICR_ISENABLER0, GICR_IPRIORITYR0, etc.)

The original code placed SGI/PPI register offsets on the RD base page (e.g. `GICR_BASE + 0x0100`), causing **Data Abort (ESR.EC=0x25, DFSC=Translation fault)**.

**Fix**: Change all SGI/PPI configuration register base addresses to `GICR_SGI_BASE = GICR_BASE + 0x1_0000`, with offsets corrected per the GICv3 specification:
- `GICR_IGROUPR0` = `SGI_BASE + 0x0080` (not `0x0100`)
- `GICR_ISENABLER0` = `SGI_BASE + 0x0100`
- `GICR_ICENABLER0` = `SGI_BASE + 0x0180`
- `GICR_IPRIORITYR0` = `SGI_BASE + 0x0400`

### 4.3 GICR_WAKER Wake-up Sequence

**Problem**: If `GICR_WAKER.ProcessorSleep` is set when accessing Redistributor registers, all SGI base page reads/writes will fault.

**Fix**: Add a wake-up sequence at the start of `gicv3_redist_init()`:
```rust
let waker = mmio_read32(GICR_WAKER);
mmio_write32(GICR_WAKER, waker & !0x2);  // Clear ProcessorSleep
while mmio_read32(GICR_WAKER) & 0x4 != 0 {}  // Wait for ChildrenAsleep to clear
```

### 4.4 Wrong Register for Interrupt Disable

**Problem**: `disable_spi()` wrote `GICD_ICACTIVER` (interrupt deactivation) and `disable_ppi()` wrote `GICR_ICACTIVER0` (same). These registers clear the *active* state, not the *enabled* state. Disabling requires writing **ICENABLER**.

**Fix**:
- `disable_spi()` → write `GICD_ICENABLER` (offset `0x0180`)
- `disable_ppi()` → write `GICR_ICENABLER0` (offset `SGI_BASE + 0x0180`)

### 4.5 QEMU `-kernel` Entry EL and MMU State

**Problem**: QEMU's `-kernel` mode may start the kernel at EL2 with MMU page tables already configured. Under an active MMU, accessing unmapped MMIO addresses (such as GICR) triggers a Translation fault, even if subsequent code assumes MMU is off.

**Fix**: Add an EL2→EL1 downgrade and MMU disable sequence in the `_start` assembly entry:
1. Check `CurrentEL`; if at EL2, downgrade to EL1 (set `HCR_EL2.RW=1`, `SPSR_EL2`, `ELR_EL2`, then `eret`)
2. Zero `SCTLR_EL1` before downgrading (disables EL1 MMU)
3. Flush TLBs while still at EL2 (`tlbi vmalle1is` + `tlbi alle2is`)
4. At EL1, confirm MMU is off and flush EL1 TLB

---

## 5. Related documents

| Doc | Content |
|-----|---------|
| [BOOT.md](./BOOT.md) | Boot info block & phases (skeleton) |
| [SYSCALL_ABI.md](./SYSCALL_ABI.md) | Syscall ABI (skeleton) |
| [PLATFORMS.md](./PLATFORMS.md) | Platform tiers |
| [GLOSSARY.md](./GLOSSARY.md) | Glossary |
| [API.md](./API.md) | Public API index (placeholder) |

---

## 6. Public API

Stable user/kernel boundary: future `syscall_abi` crate + [SYSCALL_ABI.md](./SYSCALL_ABI.md); crate-level index: [API.md](./API.md).
