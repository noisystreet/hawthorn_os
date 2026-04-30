# 山楂 / hawthorn 文档索引

> **英文索引：** [en/README.md](./en/README.md)（本页为中文总览；与 `docs/en/README.md` 成对维护。）

- **中文**正文位于本目录，文件名为 **中文**（如 `架构.md`、`内核.md`）。  
- **英文**镜像位于 **[en/](en/README.md)**，文件名保持 **英文**；与中文稿的对应关系见下表。

| 中文文档 | English |
|---------|---------|
| [架构](./架构.md) | [en/ARCHITECTURE.md](./en/ARCHITECTURE.md) |
| [测试](./测试.md) | [en/TESTING.md](./en/TESTING.md) |
| [内核](./内核.md) | [en/KERNEL.md](./en/KERNEL.md) |
| [移植](./移植.md) | [en/PORTING.md](./en/PORTING.md) |
| [引导](./引导.md) | [en/BOOT.md](./en/BOOT.md) |
| [系统调用 ABI](./系统调用ABI.md) | [en/SYSCALL_ABI.md](./en/SYSCALL_ABI.md) |
| [平台](./平台.md) | [en/PLATFORMS.md](./en/PLATFORMS.md) |
| [术语](./术语.md) | [en/GLOSSARY.md](./en/GLOSSARY.md) |
| [接口](./接口.md) | [en/API.md](./en/API.md) |
| [代码风格](./代码风格.md) | [en/CODE_STYLE.md](./en/CODE_STYLE.md) |
| [提交约定](./提交约定.md) | [en/COMMIT_CONVENTIONS.md](./en/COMMIT_CONVENTIONS.md) |
| [待办](./待办.md) | [en/TODO.md](./en/TODO.md) |
| [PR 与议题计划](./PR与议题计划.md) | [en/PR_ISSUE_PLAN.md](./en/PR_ISSUE_PLAN.md) |
| [M6 MMU 调试日志](./M6_MMU调试日志.md) | [en/M6_MMU_DEBUG_LOG.md](./en/M6_MMU_DEBUG_LOG.md) |
| [缺陷修复笔记](./缺陷修复笔记.md) | [en/BUGFIX_NOTES.md](./en/BUGFIX_NOTES.md) |

双语同步规则见 [`.cursor/rules/hawthorn-docs-bilingual.mdc`](../.cursor/rules/hawthorn-docs-bilingual.mdc)。

## Git pre-commit（可选）

在仓库根安装 [pre-commit](https://pre-commit.com/) 并执行 `pre-commit install` 后，`pre-commit` 钩子会运行与 CI 一致的 **`cargo fmt --check`**、**`cargo clippy --workspace -D warnings`**、**`cargo test --workspace`**；**`commit-msg`** 由 **`scripts/commit_msg_bilingual.py`** 校验双语标题（英文 Conventional **第 1 行** + **第 2 行**中文）。详见 [CONTRIBUTING.md](../CONTRIBUTING.md)、[提交约定.md](./提交约定.md) §1.0。

**编程 Agent：** 根目录 [AGENTS.md](../AGENTS.md)。
