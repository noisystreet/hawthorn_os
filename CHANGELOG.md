# Changelog

本文件遵循 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.0.0/) 精神，版本号在首次发布前可保持 **0.x** 语义。

## [Unreleased]

### Added

- 文档：`ARCHITECTURE`、`KERNEL`、`CODE_STYLE`、`COMMIT_CONVENTIONS`、`PORTING`、`BOOT`、`SYSCALL_ABI`、`PLATFORMS`、`GLOSSARY`、`API`（占位）
- 根工作区：`Cargo.toml`、`rust-toolchain.toml`、`.cargo/config.toml`、`kernel` 骨架 crate
- CI：`.github/workflows/ci.yml`（`fmt`、`clippy`、`check` host + `aarch64-unknown-none`）
- 根目录 **AGENTS.md**：供 AI / Agent 与协作者快速理解项目上下文
- GitHub：`.github/ISSUE_TEMPLATE/`（缺陷、功能、文档）与 **PR 模板**（`.github/pull_request_template.md`）
- Git：**pre-commit**（`cargo fmt --check`、`cargo clippy -D warnings`）；**commit-msg**：`scripts/commit_msg_bilingual.py`（英文 Conventional **第 1 行** + **第 2 行**中文、不同行）+ **`.gitmessage`** 模板
- Cursor：`.cursor/rules/*.mdc`（核心、内核 Rust、文档、工作区、**中英文文档同步**）
- 文档：`docs/en/*.md` 与 `docs/*.md` **对齐**（架构、内核、移植、启动、syscall、平台、术语、API、代码风格、提交规范）；各中文页顶增加英文镜像链接
- 贡献与安全：`CONTRIBUTING.md`、`SECURITY.md`
- 许可证：MIT + Apache-2.0 双许可

### Changed

- 项目中文名定为 **山楂**；英文代号由 **emb_os** 更名为 **hawthorn**；主内核 crate 更名为 **`hawthorn_kernel`**；`LICENSE-MIT` 著作权人为 **The Hawthorn contributors**
- 架构文档：澄清 Tier-1 与「MCU～MPU」长期愿景；机器人场景表增加里程碑列；开放决策链接改为文内引用；香橙派链接改为 HTTPS
- 提交规范：`scope` 与 `servers/` 等目录对齐
- 内核文档：引导阶段 MMU/MPU 表述与 RK3588 对齐；相关文档增加 BOOT、SYSCALL_ABI
