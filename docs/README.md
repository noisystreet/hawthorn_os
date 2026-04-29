# 山楂 / hawthorn 文档索引

> **英文索引：** [en/README.md](./en/README.md)（本页为中文总览；与 `docs/en/README.md` 成对维护。）

- **中文**正文位于本目录各 `*.md` 文件。  
- **英文**镜像位于 **[en/](en/README.md)**，与中文 **同名文件** 一一对应。

| 中文 | English |
|------|---------|
| [ARCHITECTURE.md](./ARCHITECTURE.md) | [en/ARCHITECTURE.md](./en/ARCHITECTURE.md) |
| [KERNEL.md](./KERNEL.md) | [en/KERNEL.md](./en/KERNEL.md) |
| [PORTING.md](./PORTING.md) | [en/PORTING.md](./en/PORTING.md) |
| [BOOT.md](./BOOT.md) | [en/BOOT.md](./en/BOOT.md) |
| [SYSCALL_ABI.md](./SYSCALL_ABI.md) | [en/SYSCALL_ABI.md](./en/SYSCALL_ABI.md) |
| [PLATFORMS.md](./PLATFORMS.md) | [en/PLATFORMS.md](./en/PLATFORMS.md) |
| [GLOSSARY.md](./GLOSSARY.md) | [en/GLOSSARY.md](./en/GLOSSARY.md) |
| [API.md](./API.md) | [en/API.md](./en/API.md) |
| [CODE_STYLE.md](./CODE_STYLE.md) | [en/CODE_STYLE.md](./en/CODE_STYLE.md) |
| [COMMIT_CONVENTIONS.md](./COMMIT_CONVENTIONS.md) | [en/COMMIT_CONVENTIONS.md](./en/COMMIT_CONVENTIONS.md) |
| [TODO.md](./TODO.md) | [en/TODO.md](./en/TODO.md) |
| [PR_ISSUE_PLAN.md](./PR_ISSUE_PLAN.md) | [en/PR_ISSUE_PLAN.md](./en/PR_ISSUE_PLAN.md) |
| [M6_MMU_DEBUG_LOG.md](./M6_MMU_DEBUG_LOG.md) | [en/M6_MMU_DEBUG_LOG.md](./en/M6_MMU_DEBUG_LOG.md) |
| [BUGFIX_NOTES.md](./BUGFIX_NOTES.md) | [en/BUGFIX_NOTES.md](./en/BUGFIX_NOTES.md) |

双语同步规则见 [`.cursor/rules/hawthorn-docs-bilingual.mdc`](../.cursor/rules/hawthorn-docs-bilingual.mdc)。

## Git pre-commit（可选）

在仓库根安装 [pre-commit](https://pre-commit.com/) 并执行 `pre-commit install` 后，`pre-commit` 钩子会运行与 CI 一致的 **`cargo fmt --check`**、**`cargo clippy --workspace -D warnings`**、**`cargo test --workspace`**；**`commit-msg`** 由 **`scripts/commit_msg_bilingual.py`** 校验双语标题（英文 Conventional **第 1 行** + **第 2 行**中文）。详见 [CONTRIBUTING.md](../CONTRIBUTING.md)、[COMMIT_CONVENTIONS.md](./COMMIT_CONVENTIONS.md) §1.0。

**编程 Agent：** 根目录 [AGENTS.md](../AGENTS.md)。
