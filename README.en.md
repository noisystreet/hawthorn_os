# Hawthorn (山楂)

**Hawthorn** is an embedded OS for **robotics** and **smart hardware**, written in **Rust** and built as a **microkernel**.

**Chinese name: 山楂**; **English code name: hawthorn** (Rust crates, `-p` flags, and doc shorthand refer to the same project).  
Using **`hawthorn`** for the Git remote and clone directory is recommended; other local path names do not affect `cargo` builds.

**Tier-1 hardware:** [Orange Pi 5](https://www.orangepi.org/html/hardWare/computerAndMicrocontrollers/details/Orange-Pi-5.html) (**RK3588**, AArch64).

This repository currently contains **design documentation**, a **minimal Rust workspace** (the `kernel` crate and a bootable AArch64 ELF under **`qemu_minimal/`** for QEMU `virt`), and **CI**. The full kernel and BSP are still in active development.

> **[中文](README.md)** — Chinese README (default on GitHub for this repo).

---

## Documentation

Full index: **[docs/en/README.md](docs/en/README.md)** (English pages under `docs/en/`, paired with Chinese sources under `docs/`).

| Doc | Description |
|-----|-------------|
| [docs/en/ARCHITECTURE.md](docs/en/ARCHITECTURE.md) | Goals, layering, tier-1 platform, roadmap, security |
| [docs/en/TESTING.md](docs/en/TESTING.md) | Test layers (L1–L4) and QEMU verification |
| [docs/en/KERNEL.md](docs/en/KERNEL.md) | Microkernel modules, object model, IPC, RK3588 notes |
| [docs/en/PORTING.md](docs/en/PORTING.md) | Porting, build prerequisites, boot and memory layout |
| [docs/en/BOOT.md](docs/en/BOOT.md) | Boot info block and phases (skeleton) |
| [docs/en/SYSCALL_ABI.md](docs/en/SYSCALL_ABI.md) | Syscall ABI (skeleton) |
| [docs/en/PLATFORMS.md](docs/en/PLATFORMS.md) | Platform tier list |
| [docs/en/GLOSSARY.md](docs/en/GLOSSARY.md) | Glossary |
| [docs/en/API.md](docs/en/API.md) | Public API index (placeholder) |
| [docs/en/CODE_STYLE.md](docs/en/CODE_STYLE.md) | Rust style guide |
| [docs/en/COMMIT_CONVENTIONS.md](docs/en/COMMIT_CONVENTIONS.md) | Git commits and PRs |
| [docs/en/TODO.md](docs/en/TODO.md) | Feature and capability backlog |
| [docs/en/PR_ISSUE_PLAN.md](docs/en/PR_ISSUE_PLAN.md) | Milestone PR / GitHub issue order |
| [CHANGELOG.md](CHANGELOG.md) | Changelog |
| [docs/README.md](docs/README.md) | Chinese documentation hub |

---

## Build

Install [Rust / rustup](https://rustup.rs/). From the repository root:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check -p hawthorn_kernel
cargo check -p hawthorn_kernel --target aarch64-unknown-none
cargo build -p hawthorn_kernel --features bare-metal --target aarch64-unknown-none
cargo build -p hawthorn_qemu_minimal --features bare-metal --target aarch64-unknown-none
```

`rust-toolchain.toml` pins **stable** and the **`aarch64-unknown-none`** target.

**Pre-commit (recommended):** install [pre-commit](https://pre-commit.com/) and run `pre-commit install` at the repo root so **`cargo fmt --check`**, **Clippy**, and **`cargo test --workspace`** run on commit, aligned with CI. See [CONTRIBUTING.md](CONTRIBUTING.md).

For image flashing, linker scripts, and the **minimal QEMU image** ([`scripts/run_qemu_minimal.sh`](scripts/run_qemu_minimal.sh)), see [docs/en/PORTING.md](docs/en/PORTING.md).

`hawthorn_kernel` serial regression scripts:

- Basic boot: [`scripts/verify_kernel_qemu_virt_serial.sh`](scripts/verify_kernel_qemu_virt_serial.sh)  
- With EL0 user path (expect `hello from EL0!`): [`scripts/verify_kernel_qemu_virt_el0_serial.sh`](scripts/verify_kernel_qemu_virt_el0_serial.sh)

---

## Repository layout (planned)

As in [Architecture §8](docs/en/ARCHITECTURE.md), the tree is expected to grow into:

- `kernel/` — microkernel (crate in place today)  
- `servers/` — user-space drivers and services  
- `hal/`, `bsp/orangepi5-rk3588/` — hardware abstraction and tier-1 board support  
- `syscall_abi/`, `middleware/`, `examples/`, `tools/` — incrementally as needed  

---

## Contributing and security

See [CONTRIBUTING.md](CONTRIBUTING.md) and [SECURITY.md](SECURITY.md). Before submitting changes, read [Code style](docs/en/CODE_STYLE.md) and [Commit conventions](docs/en/COMMIT_CONVENTIONS.md). For **Cursor** or other coding agents, start with **[AGENTS.md](AGENTS.md)**; rules live under [`.cursor/rules/`](.cursor/rules/).

---

## License

Dual-licensed under **Apache License 2.0** and **MIT** (choose either).

- Full texts: [`LICENSE-APACHE`](LICENSE-APACHE), [`LICENSE-MIT`](LICENSE-MIT).

This matches common **Rust** projects; **Apache-2.0** includes a patent grant. Contributions are licensed under the same terms unless stated otherwise; see [CONTRIBUTING.md](CONTRIBUTING.md).
