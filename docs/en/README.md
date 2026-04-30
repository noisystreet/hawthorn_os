# English documentation (`docs/en/`)

> **[中文总览](../README.md)** — Chinese hub (`docs/README.md`); keep this file and that hub updated together.

This directory holds **English** technical pages. Each file pairs with a **Chinese** source under **`docs/`** using a **Chinese filename** (see **`docs/README.md`** and the table below).  
Each English page links to its Chinese source at the top: `> **[中文](../….md)**`.

Keep **Chinese and English in sync** on every substantive edit (see `.cursor/rules/hawthorn-docs-bilingual.mdc`).

**AI / coding agents:** read **[AGENTS.md](../../AGENTS.md)** for orientation.

## Document index

| English | Chinese |
|---------|---------|
| [ARCHITECTURE.md](./ARCHITECTURE.md) | [../架构.md](../架构.md) |
| [TESTING.md](./TESTING.md) | [../测试.md](../测试.md) |
| [KERNEL.md](./KERNEL.md) | [../内核.md](../内核.md) |
| [PORTING.md](./PORTING.md) | [../移植.md](../移植.md) |
| [BOOT.md](./BOOT.md) | [../引导.md](../引导.md) |
| [SYSCALL_ABI.md](./SYSCALL_ABI.md) | [../系统调用ABI.md](../系统调用ABI.md) |
| [PLATFORMS.md](./PLATFORMS.md) | [../平台.md](../平台.md) |
| [GLOSSARY.md](./GLOSSARY.md) | [../术语.md](../术语.md) |
| [API.md](./API.md) | [../接口.md](../接口.md) |
| [CODE_STYLE.md](./CODE_STYLE.md) | [../代码风格.md](../代码风格.md) |
| [COMMIT_CONVENTIONS.md](./COMMIT_CONVENTIONS.md) | [../提交约定.md](../提交约定.md) |
| [TODO.md](./TODO.md) | [../待办.md](../待办.md) |
| [PR_ISSUE_PLAN.md](./PR_ISSUE_PLAN.md) | [../PR与议题计划.md](../PR与议题计划.md) |
| [M6_MMU_DEBUG_LOG.md](./M6_MMU_DEBUG_LOG.md) | [../M6_MMU调试日志.md](../M6_MMU调试日志.md) |
| [BUGFIX_NOTES.md](./BUGFIX_NOTES.md) | [../缺陷修复笔记.md](../缺陷修复笔记.md) |

## Pre-commit

Install [pre-commit](https://pre-commit.com/) and run `pre-commit install` at the repo root. Hooks run **`cargo fmt --check`**, **`cargo clippy --workspace -D warnings`**, **`cargo test --workspace`**, and **`commit-msg`** ([`scripts/commit_msg_bilingual.py`](../../scripts/commit_msg_bilingual.py): **English line 1** + **Chinese line 2**, separate lines), matching [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml). Details: [CONTRIBUTING.md](../../CONTRIBUTING.md), [COMMIT_CONVENTIONS.md §1.0](./COMMIT_CONVENTIONS.md).

[← Parent `docs/`](../)
