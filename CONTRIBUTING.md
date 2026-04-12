# 参与贡献

感谢你对 **山楂（hawthorn）** 的兴趣。使用 AI / Agent 协作时请先阅读根目录 **[AGENTS.md](AGENTS.md)**。提交前请阅读：

- **GitHub**：[Issue 模板](.github/ISSUE_TEMPLATE)、[Pull Request 模板](.github/pull_request_template.md)（新建 Issue/PR 时选用）
- [代码风格](docs/CODE_STYLE.md)
- [提交规范](docs/COMMIT_CONVENTIONS.md)
- [架构说明](docs/ARCHITECTURE.md) 与 [微内核设计](docs/KERNEL.md)（涉及内核行为或 ABI 的变更）

## Git pre-commit（推荐）

本仓库使用 [pre-commit](https://pre-commit.com/)：

- **`pre-commit` 阶段**（每次 `git commit` 前）：**`cargo fmt --check`**、**`cargo clippy -D warnings`**（与 CI 一致）。  
- **`commit-msg` 阶段**：**`scripts/commit_msg_bilingual.py`** 要求提交说明前两条非注释行为 **英文 Conventional Commits 标题行** + **单独一行中文**（语义对应、不得与英文同行），详见 [提交规范](docs/COMMIT_CONVENTIONS.md) §1.0。

```bash
pip install pre-commit          # 或: brew install pre-commit
cd /path/to/hawthorn            # 仓库根目录
pre-commit install              # 安装 pre-commit + commit-msg（见 .pre-commit-config.yaml）
```

可选：`git config commit.template .gitmessage`，在编辑器中显示提交说明提示（`#` 行不会进入最终提交）。

也可手动检查工作区（不含 commit-msg）：

```bash
pre-commit run --all-files
```

若钩子失败：先 **`cargo fmt --all`** 再提交；按 Clippy 提示修复或审慎添加 `#[allow(...)]`（见 [代码风格](docs/CODE_STYLE.md)）。若 **commit-msg** 报错，请按 [提交规范](docs/COMMIT_CONVENTIONS.md) §1.0 使用**两行标题**（英文 + 中文各占一行），例如：  
`docs: fix typo in PORTING` 下一行写 **`修正 PORTING 文档笔误。`**

## 许可证

本仓库（**山楂** / **hawthorn**）采用 **MIT OR Apache-2.0** 双许可（见根目录 `LICENSE-MIT`、`LICENSE-APACHE`）。  
你向本仓库提交的代码，在 **未另行书面约定** 的前提下，视为你在 **相同双许可** 下授权他人使用（与常见 Rust 开源项目惯例一致）。若需保留不同授权，请事先在 Issue 或 PR 中说明。

新源码文件建议标注：

```text
SPDX-License-Identifier: MIT OR Apache-2.0
```

## 安全漏洞

请勿在公开 Issue 中披露敏感安全问题；请按 [SECURITY.md](SECURITY.md) 指引联系维护者。

## 行为准则

在 Issue、PR 与讨论中请保持 **专业、尊重、就事论事**；若后续采用正式行为准则（如 Contributor Covenant），将在此更新链接。
