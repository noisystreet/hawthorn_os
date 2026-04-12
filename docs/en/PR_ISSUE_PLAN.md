# PR and issue plan (current milestone)

> **[中文](../PR_ISSUE_PLAN.md)** — Chinese source of this document.

This page pins **GitHub issues** and the recommended **PR order** so branches and PR bodies can use `Closes #…` / `Refs #…`. The capability backlog remains [TODO.md](./TODO.md).

---

## Currently tracked: minimal `hawthorn_kernel` bring-up on QEMU `virt`

| Role | Link |
|------|------|
| **Meta (rollup)** | <https://github.com/noisystreet/hawthorn_os/issues/5> |

### Issues (suggested implementation order)

| Order | Issue | Title (summary) |
|-------|--------|-----------------|
| 1 | [#1](https://github.com/noisystreet/hawthorn_os/issues/1) | M1: `hawthorn_kernel` minimal boot (QEMU virt) + PL011 panic |
| 2 | [#2](https://github.com/noisystreet/hawthorn_os/issues/2) | M1b: `qemu_minimal` starts via `hawthorn_kernel` public API |
| 3 | [#3](https://github.com/noisystreet/hawthorn_os/issues/3) | M2: `VBAR_EL1` vectors + sync/IRQ stubs |
| 4 | [#4](https://github.com/noisystreet/hawthorn_os/issues/4) | M3: cooperative scheduler MVP (TCB / ready queue / yield) |

**Suggested PR sequence:** `#1 → #2 → #3 → #4`. **#3** may proceed in parallel with **#2** once M1 entry symbols are stable; resolve rebase conflicts if both touch boot/entry.

---

## PR conventions

1. **One PR per issue** when practical; split large work but each PR should still `Closes #n` or `Refs #n`.
2. Use [.github/pull_request_template.md](../../.github/pull_request_template.md); under **Related issues** add e.g. `Closes #1`.
3. **Commits:** `docs/COMMIT_CONVENTIONS.md` — English Conventional line 1, matching Chinese line 2.
4. **Labels:** kernel work uses `kernel` + `enhancement`; new issues should keep tags like **`[kernel]`**, `[IPC]` (same habit as [TODO.md](./TODO.md)).

---

## Local verification (CI / AGENTS)

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo check -p hawthorn_kernel
cargo check -p hawthorn_kernel --target aarch64-unknown-none
cargo build -p hawthorn_qemu_minimal --features bare-metal --target aarch64-unknown-none
```

After M1 lands, if new crate features or bins appear, update this file and the **acceptance criteria** on the relevant issue.

---

## Next (issues not created yet)

Examples for the next batch (from TODO): `syscall_abi` crate, unified SVC dispatch, minimal IPC (short messages). After meta **#5** closes, open a new `[meta]` issue for the next phase.
