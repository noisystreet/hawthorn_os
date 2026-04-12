# Hawthorn / 山楂 — Code style

> **[中文](../CODE_STYLE.md)** — Chinese source of this document.

Rust style, static checks, and review focus for **Hawthorn (山楂)** — kernel, HAL, drivers, tooling. If this doc disagrees with **CI** / `rustfmt` / `clippy` config, **CI wins**; update this doc when config changes.

---

## 1. Toolchain & automation

- **Rust:** [rust-toolchain.toml](../../rust-toolchain.toml) is authoritative (**stable**, `aarch64-unknown-none`, `rustfmt`, `clippy`).  
- **Format:** run `cargo fmt` before commit. **pre-commit** (root `.pre-commit-config.yaml`, [CONTRIBUTING.md](../../CONTRIBUTING.md)) runs `cargo fmt --check` on commit. Do not fight `rustfmt`; use `#[rustfmt::skip]` only in tiny scopes with a reason.  
- **Lint:** `cargo clippy --workspace --all-targets` (or CI equivalent; **avoid** `--all-features` on this workspace—it enables **`bare-metal`** on **`hawthorn_kernel`** and **`hawthorn_qemu_minimal`** and tries to build bare-metal `bin` targets on the host). New code should not add **warn**-level issues; `#[allow(...)]` needs a short comment.  
- **Build:** `cargo build` / `cargo check` must pass; bare-metal target: [PORTING.md](./PORTING.md), [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml).

---

## 2. Naming

- **Identifiers:** `snake_case` modules/functions/vars; `PascalCase` types/traits/enum variants; `SCREAMING_SNAKE_CASE` constants.  
- **Semantics:** avoid cryptic abbreviations; register names may match the datasheet.  
- **Visibility:** default `pub(crate)`; `pub` only for stable cross-crate API.

---

## 3. Docs & comments

- **Public API** (`pub` cross-crate): `///` with summary; note safety, errors, panic conditions.  
- **Language:** `///` in **English** for ecosystem tooling; `//` may use Chinese for tricky invariants.  
- **`unsafe`:** each block needs **`// SAFETY:`** (or project prefix); safe wrappers document contracts.

---

## 4. `unsafe` & invariants

- Keep `unsafe` **small**; wrap in functions with contracts.  
- PRs touching `unsafe` deserve **extra review**; add tests, Miri where applicable, or HIL notes.  
- **FFI:** isolate C boundaries; newtypes / thin wrappers; avoid raw pointers in business logic.

---

## 5. Errors & robustness

- **Libraries:** return `Result` / typed errors; no bare `unwrap`/`expect` without justification; `expect("…")` states **why** it is unreachable.  
- **Examples / tests:** `unwrap` allowed.  
- **Real-time paths:** no **alloc**, blocking locks, or unbounded waits in ISR or hard-RT paths; document **call context** in comments.

---

## 6. Modules & dependencies

- **Crates:** follow layering in architecture docs; **no** cycles.  
- **Features:** optional behavior via `Cargo.toml` `feature`; defaults stay **minimal but buildable**.  
- **`use` order:** `std`/`core` → external → `crate::` → `super::`/`self::`, blank line between groups (`rustfmt`).

---

## 7. Tests

- **Unit:** `mod tests` in file or `tests/`; pure logic on host with `cargo test`.  
- **Integration / HIL:** document hardware in `tests/` or separate dir; CI-skipped tests use `#[ignore]` + how to enable.

---

## 8. Other

- **Logging:** project log facade when present; avoid `println!` on hot paths in libraries.  
- **License / SPDX:** new source files:

  `SPDX-License-Identifier: MIT OR Apache-2.0`

  (matches root `LICENSE-*`; follow `REUSE` if adopted.)

---

## Related documents

- [Architecture](./ARCHITECTURE.md)
- [Microkernel design](./KERNEL.md)
- [Porting](./PORTING.md)
- [Commit conventions](./COMMIT_CONVENTIONS.md)
- [Contributing](../../CONTRIBUTING.md)
