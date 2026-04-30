# English documentation (`docs/en/`)

> **[中文总览](../README.md)** — Chinese hub (`docs/README.md`); keep this file and that hub updated together.

This directory mirrors **`docs/*.md`** (except the parent hub `docs/README.md`) with the **same basename**. Each technical English page links to its **Chinese** source at the top (`> **[中文](../…)**`).

Keep **Chinese and English in sync** on every substantive edit (see `.cursor/rules/hawthorn-docs-bilingual.mdc`).

**AI / coding agents:** read **[AGENTS.md](../../AGENTS.md)** for orientation.

## Document index

| English | Chinese |
|---------|---------|
| [ARCHITECTURE.md](./ARCHITECTURE.md) | [../ARCHITECTURE.md](../ARCHITECTURE.md) |
| [TESTING.md](./TESTING.md) | [../TESTING.md](../TESTING.md) |
| [KERNEL.md](./KERNEL.md) | [../KERNEL.md](../KERNEL.md) |
| [PORTING.md](./PORTING.md) | [../PORTING.md](../PORTING.md) |
| [BOOT.md](./BOOT.md) | [../BOOT.md](../BOOT.md) |
| [SYSCALL_ABI.md](./SYSCALL_ABI.md) | [../SYSCALL_ABI.md](../SYSCALL_ABI.md) |
| [PLATFORMS.md](./PLATFORMS.md) | [../PLATFORMS.md](../PLATFORMS.md) |
| [GLOSSARY.md](./GLOSSARY.md) | [../GLOSSARY.md](../GLOSSARY.md) |
| [API.md](./API.md) | [../API.md](../API.md) |
| [CODE_STYLE.md](./CODE_STYLE.md) | [../CODE_STYLE.md](../CODE_STYLE.md) |
| [COMMIT_CONVENTIONS.md](./COMMIT_CONVENTIONS.md) | [../COMMIT_CONVENTIONS.md](../COMMIT_CONVENTIONS.md) |
| [TODO.md](./TODO.md) | [../TODO.md](../TODO.md) |
| [PR_ISSUE_PLAN.md](./PR_ISSUE_PLAN.md) | [../PR_ISSUE_PLAN.md](../PR_ISSUE_PLAN.md) |
| [M6_MMU_DEBUG_LOG.md](./M6_MMU_DEBUG_LOG.md) | [../M6_MMU_DEBUG_LOG.md](../M6_MMU_DEBUG_LOG.md) |
| [BUGFIX_NOTES.md](./BUGFIX_NOTES.md) | [../BUGFIX_NOTES.md](../BUGFIX_NOTES.md) |

## Pre-commit

Install [pre-commit](https://pre-commit.com/) and run `pre-commit install` at the repo root. Hooks run **`cargo fmt --check`**, **`cargo clippy --workspace -D warnings`**, **`cargo test --workspace`**, and **`commit-msg`** ([`scripts/commit_msg_bilingual.py`](../../scripts/commit_msg_bilingual.py): **English line 1** + **Chinese line 2**, separate lines), matching [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml). Details: [CONTRIBUTING.md](../../CONTRIBUTING.md), [COMMIT_CONVENTIONS.md §1.0](./COMMIT_CONVENTIONS.md).

[← Parent `docs/`](../)
