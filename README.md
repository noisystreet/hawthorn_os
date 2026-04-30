# 山楂

面向 **机器人** 与 **智能硬件** 的嵌入式操作系统，以 **Rust** 实现，采用 **微内核** 架构。

**中文名：山楂**；**英文代号：hawthorn**（Rust crate、`-p` 参数与文档简称；与「山楂」指同一项目）。  
建议 Git 远程仓库与克隆目录也使用 **`hawthorn`**；若本地路径仍为旧名，不影响 `cargo` 构建。

**首发硬件**：[香橙派 5](https://www.orangepi.org/html/hardWare/computerAndMicrocontrollers/details/Orange-Pi-5.html)（**RK3588**，AArch64）。

当前仓库包含 **设计文档**、**最小 Rust 工作区**（`kernel` 占位 crate、**`qemu_minimal/`** 下 QEMU `virt` 可启动的最小 ELF）与 **CI**；完整内核与 BSP 仍在实现中。

---

## 文档

完整对照表见 **[docs/README.md](docs/README.md)**（`docs/` 中文文件名 + `docs/en/` 英文镜像）。

| 文档 | 说明 |
|------|------|
| [docs/架构.md](docs/架构.md) | 总体目标、分层、首发平台、演进与安全 |
| [docs/测试.md](docs/测试.md) | 测试分层（L1–L4）与 QEMU 验证约定 |
| [docs/内核.md](docs/内核.md) | 微内核模块、对象模型、IPC、RK3588 约束 |
| [docs/移植.md](docs/移植.md) | 移植、构建前提、启动与内存布局占位 |
| [docs/引导.md](docs/引导.md) | 启动信息块与引导阶段（骨架） |
| [docs/系统调用ABI.md](docs/系统调用ABI.md) | 系统调用 ABI（骨架） |
| [docs/平台.md](docs/平台.md) | 平台 Tier 列表 |
| [docs/术语.md](docs/术语.md) | 术语表 |
| [docs/接口.md](docs/接口.md) | 对外 API 索引（占位） |
| [docs/代码风格.md](docs/代码风格.md) | Rust 代码风格 |
| [docs/提交约定.md](docs/提交约定.md) | Git 提交与 PR 约定 |
| [docs/待办.md](docs/待办.md) | 新功能与能力扩展（TODO 列表） |
| [docs/PR与议题计划.md](docs/PR与议题计划.md) | 当前里程碑：PR 与 GitHub Issue 顺序（含链接） |
| [CHANGELOG.md](CHANGELOG.md) | 变更记录 |
| [docs/en/README.md](docs/en/README.md) | 英文文档索引（与 `docs/` 中文正文成对，见索引表） |

---

## 构建

需安装 [Rust / rustup](https://rustup.rs/)。在仓库根目录执行：

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check -p hawthorn_kernel
cargo check -p hawthorn_kernel --target aarch64-unknown-none
cargo build -p hawthorn_kernel --features bare-metal --target aarch64-unknown-none
cargo build -p hawthorn_qemu_minimal --features bare-metal --target aarch64-unknown-none
```

`rust-toolchain.toml` 已指定 **stable** 与目标 **`aarch64-unknown-none`**。

**提交前检查（推荐）**：安装 [pre-commit](https://pre-commit.com/) 后在仓库根执行 `pre-commit install`，与 CI 相同的 **格式化检查**、**Clippy** 与 **工作区单元测试**（`cargo test --workspace`）会在 `git commit` 时运行。说明见 [CONTRIBUTING.md](CONTRIBUTING.md)。

镜像烧录、链接脚本与 **QEMU 最小镜像**（[`scripts/run_qemu_minimal.sh`](scripts/run_qemu_minimal.sh)）见 [docs/移植.md](docs/移植.md)。

`hawthorn_kernel` 串口回归脚本：

- 基础启动回归：[`scripts/verify_kernel_qemu_virt_serial.sh`](scripts/verify_kernel_qemu_virt_serial.sh)
- 含 EL0 用户态回归（需出现 `hello from EL0!`）：[`scripts/verify_kernel_qemu_virt_el0_serial.sh`](scripts/verify_kernel_qemu_virt_el0_serial.sh)

---

## 仓库规划（实现阶段）

与 [架构文档 §8](docs/架构.md) 一致，预期目录包括：

- `kernel/` — 微内核（已建占位 crate）  
- `servers/` — 用户态驱动与服务  
- `hal/`、`bsp/orangepi5-rk3588/` — 硬件抽象与首发板级支持  
- `syscall_abi/`、`middleware/`、`examples/`、`tools/` — 按需增量  

---

## 参与贡献与安全

请参阅 [CONTRIBUTING.md](CONTRIBUTING.md) 与 [SECURITY.md](SECURITY.md)；提交前请阅读 [代码风格](docs/代码风格.md) 与 [提交规范](docs/提交约定.md)。使用 **Cursor** 或其它编程 Agent 时：先读 **[AGENTS.md](AGENTS.md)**；Cursor 规则见 [`.cursor/rules/`](.cursor/rules/)。

---

## 许可证

本仓库采用 **Apache License 2.0** 与 **MIT** 之 **双许可**：你可任选其一适用。

- 全文见 [`LICENSE-APACHE`](LICENSE-APACHE)、[`LICENSE-MIT`](LICENSE-MIT)。

该组合与 **Rust 项目**常用策略一致；**Apache-2.0** 另含专利授权条款。贡献默认在相同双许可下授权，详见 [CONTRIBUTING.md](CONTRIBUTING.md)。
