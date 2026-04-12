# 山楂 / hawthorn 代码风格

> **[English](./en/CODE_STYLE.md)** — English mirror of this document.

本文档约定 **山楂（hawthorn）** 项目中 **Rust** 代码风格、静态检查与评审关注点，适用于内核、HAL、驱动与工具链。若与本文冲突，以 **CI 与 `rustfmt`/`clippy` 实际配置** 为准；配置变更时需同步更新本文。

---

## 1. 工具链与自动化

- **Rust 版本**：以仓库根目录 [rust-toolchain.toml](../rust-toolchain.toml) 为准（当前为 **stable**，并包含 `aarch64-unknown-none` 目标与 `rustfmt`、`clippy`）。
- **格式化**：提交前运行 `cargo fmt`。推荐安装 **pre-commit**（见根目录 `.pre-commit-config.yaml` 与 [CONTRIBUTING.md](../CONTRIBUTING.md)），在 `git commit` 时自动执行 `cargo fmt --check`。不手工与 `rustfmt` 对抗；若需例外，在极小范围内使用 `#[rustfmt::skip]` 并写明原因。
- **静态分析**：提交前运行 `cargo clippy --workspace --all-targets`（或 CI 等价命令；**勿**对当前工作区使用 `--all-features`，否则会打开 `hawthorn_qemu_minimal` 的 `bare-metal` 并在主机上误编裸机 bin）。新增代码不应引入 **warn** 级别告警；若确需允许某条 lint，使用 **模块级或行级** `#![allow(...)]` / `#[allow(...)]` 并附简短注释说明理由。
- **构建**：`cargo build` / `cargo check` 应能通过；首发裸机目标见 [PORTING.md](./PORTING.md) 与根目录 [`.github/workflows/ci.yml`](../.github/workflows/ci.yml)。

---

## 2. 语言与命名

- **标识符**：模块、函数、变量使用 `snake_case`；类型、trait、枚举变体使用 `PascalCase`；常量使用 `SCREAMING_SNAKE_CASE`。
- **命名语义**：避免过度缩写；硬件寄存器名可与数据手册一致（可略长）。
- **可见性**：默认 `pub(crate)`；仅确需跨 crate 的稳定接口使用 `pub`，并视为 API 承诺，谨慎变更。

---

## 3. 文档与注释

- **公共 API**（`pub` 且跨 crate）：使用 `///` 文档注释，首行简短摘要，必要时附 **安全性、错误条件、panic 条件** 说明。
- **语言**：公共 `///` 建议使用 **英文**，便于与 Rust 生态工具与下游对接；行内 `//` 可对复杂不变量用中文补充，避免冗长复述代码行为。
- **`unsafe`**：每个 `unsafe` 块旁须有 **`// SAFETY:`**（或项目统一前缀）说明调用方需满足的不变量；封装函数须在文档中写明安全契约。

---

## 4. `unsafe` 与不变量

- **范围**：`unsafe` 块尽量 **小**；优先封装为带契约的函数，而非在大段逻辑中散落。
- **评审**：含 `unsafe` 的 PR 默认需要 **额外关注**；优先补充测试、 Miri（若适用）或硬件在环说明。
- **FFI**：与 C 的边界集中在专用模块；类型与生命周期用 **新类型** 或 thin wrapper 约束，避免裸指针在业务层扩散。

---

## 5. 错误处理与健壮性

- **库代码**：对外返回 `Result` / 自定义错误类型；避免无说明的 `unwrap`/`expect`；确属编程错误可用 `expect("…")` 并写清 **为何不应发生**。
- **示例与测试**：允许 `unwrap`；`#[cfg(test)]` 中的宽松度可高于生产路径。
- **实时路径**：硬实时或中断上下文避免 **分配**、**可能阻塞的锁**、**未bounded 的等待**；相关函数在文档中标注 **调用上下文约束**。

---

## 6. 模块组织与依赖

- **crate 边界**：遵循架构文档中的分层与依赖方向；禁止循环依赖。
- **特性（features）**：可选功能用 `Cargo.toml` 的 `feature` 控制；默认 feature 集应保持 **可构建且合理最小**。
- **`use` 顺序**：可按 `std`/`core` → 外部 crate → `crate::` → `super::`/`self::` 分组，组间空一行（与 `rustfmt` 一致即可）。

---

## 7. 测试

- **单元测试**：与实现同文件 `mod tests` 或 `tests/` 目录；纯逻辑优先在 host 上 `cargo test`。
- **集成 / HIL**：在 `tests/` 或单独目录说明硬件前置条件；无法在 CI 跑通的测试须标注 `#[ignore]` 并写明启用方式。

---

## 8. 其他

- **日志**：使用项目统一的日志门面（若已引入）；避免 `println!` 进入库的热路径。
- **版权与 SPDX**：新源码文件建议在文件首行或紧接 crate 文档处标注  
  `SPDX-License-Identifier: MIT OR Apache-2.0`  
  （与根目录 `LICENSE-MIT`、`LICENSE-APACHE` 一致；若启用 `REUSE` 则从其规定）。

---

## 相关文档

- [架构说明](./ARCHITECTURE.md)
- [微内核设计](./KERNEL.md)
- [移植指南](./PORTING.md)
- [提交规范](./COMMIT_CONVENTIONS.md)
- [贡献指南](../CONTRIBUTING.md)
