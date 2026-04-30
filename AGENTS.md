# AGENTS.md — Hawthorn (山楂) for coding agents

This file orients **AI assistants, CI bots, and contributors** using agent-style workflows. Canonical human docs live under `docs/` (Chinese) and `docs/en/` (English mirrors).

---

## 1. What this project is

| Item | Value |
|------|--------|
| Chinese name | **山楂** |
| English code name | **hawthorn** |
| Kind | **Rust** embedded OS, **microkernel** |
| Tier-1 hardware | [Orange Pi 5](https://www.orangepi.org/html/hardWare/computerAndMicrocontrollers/details/Orange-Pi-5.html), SoC **RK3588**, AArch64 + MMU |
| Kernel crate | **`hawthorn_kernel`** in `kernel/` (workspace member) |

Drivers, network stacks, and file systems are intended to run as **user services** (`servers/` — planned), not inside the microkernel. See `docs/内核.md` / `docs/en/KERNEL.md`.

---

## 2. Read first (before large edits)

1. `docs/架构.md` or `docs/en/ARCHITECTURE.md` — goals, layering, roadmap, open decisions.  
2. `docs/内核.md` or `docs/en/KERNEL.md` — kernel modules, IPC, capabilities, RK3588 notes.  
3. `docs/测试.md` or `docs/en/TESTING.md` — test layers (L1–L4), QEMU scripts vs `cargo test`, CI mapping.  
4. `docs/代码风格.md` — Rust / `no_std` / `unsafe` / Clippy expectations.  
5. `CONTRIBUTING.md` — license, pre-commit, security reporting.

**Bilingual rule:** substantive doc changes must update **both** the Chinese file under `docs/<中文名>.md` and the paired **`docs/en/<EnglishName>.md`** in the same change (see `.cursor/rules/hawthorn-docs-bilingual.mdc`). Hub pages: `docs/README.md` ↔ `docs/en/README.md`.

**PR / issue milestone (minimal kernel on QEMU `virt`):** see [`docs/PR与议题计划.md`](docs/PR与议题计划.md) (Chinese) and [`docs/en/PR_ISSUE_PLAN.md`](docs/en/PR_ISSUE_PLAN.md) (English) — ordered GitHub issues **#1–#4**, meta **#5**.

---

## 3. Repository layout (planned + current)

```
hawthorn/   # suggested clone name; crate is hawthorn regardless
├── kernel/              # hawthorn_kernel — microkernel (only on-disk code today)
├── qemu_minimal/        # hawthorn_qemu_minimal — QEMU virt PL011 smoke binary
├── docs/, docs/en/      # Chinese + English mirrors
├── .cursor/rules/       # Cursor agent rules (hawthorn-*.mdc)
├── .github/workflows/   # CI: fmt, clippy, test, cargo check (host + aarch64-unknown-none)
├── .pre-commit-config.yaml
├── Cargo.toml           # workspace
└── rust-toolchain.toml  # stable + aarch64-unknown-none + rustfmt + clippy
```

Planned (not necessarily present yet): `servers/`, `hal/`, `bsp/orangepi5-rk3588/`, `syscall_abi/`, `middleware/`, `examples/`, `tools/`.

**Hard rule:** `kernel` **must not** depend on `servers` or user crates; user ↔ kernel boundary is **syscall + stable ABI** only.

---

## 4. Verify locally (match CI)

```bash
typos                             # spell check; or: pre-commit run typos --all-files
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings -W clippy::cognitive_complexity
cargo test --workspace
cargo check -p hawthorn_kernel
cargo check -p hawthorn_kernel --target aarch64-unknown-none
cargo build -p hawthorn_kernel --features bare-metal --target aarch64-unknown-none
cargo build -p hawthorn_qemu_minimal --features bare-metal --target aarch64-unknown-none
bash scripts/verify_kernel_qemu_virt_serial.sh
bash scripts/verify_kernel_qemu_virt_el0_serial.sh
```

**L3 (QEMU):** requires `qemu-system-aarch64` and `socat` (see CI). Strategy: `docs/测试.md` / `docs/en/TESTING.md`.

Optional: `pre-commit install` then each `git commit` runs **typos** / **fmt** / **clippy** (with cognitive complexity lint) / **test** (`pre-commit` stage) and validates the **commit message first line** (**`commit-msg`**, Conventional Commits — see `docs/提交约定.md`). Optional: `git config commit.template .gitmessage` (see `CONTRIBUTING.md`).

---

## 5. Cursor-specific rules

Project-specific agent instructions: **`.cursor/rules/`** (`hawthorn-core.mdc`, `hawthorn-kernel-rust.mdc`, `hawthorn-docs.mdc`, `hawthorn-docs-bilingual.mdc`, `hawthorn-workspace.mdc`). Prefer them over generic guesses when they apply.

---

## 6. Communication & comments

- User-facing explanations for this repo are often **Simplified Chinese**; **public Rust `///` API docs** should stay **English** where possible (`docs/代码风格.md`).  
- Commit messages: follow `docs/提交约定.md` (Conventional Commits).

---

## 7. License

Dual **MIT OR Apache-2.0** — see `LICENSE-MIT`, `LICENSE-APACHE`. New source: `SPDX-License-Identifier: MIT OR Apache-2.0`.

---

## 8. GitHub workflows

- **Issue templates:** `.github/ISSUE_TEMPLATE/` — bug report, feature/design, documentation (Chinese).  
- **PR template:** `.github/pull_request_template.md` — checklist aligned with CI and bilingual docs.  
- **Commit messages:** first non-empty line = **English** Conventional Commits; second = **Chinese** same meaning, **separate line** (enforced by `scripts/commit_msg_bilingual.py` via pre-commit `commit-msg`).

## 9. Security

Do not post exploitable security issues in public issues; see `SECURITY.md`.
