# 山楂 / hawthorn 移植与运行环境

> **[English](./en/PORTING.md)** — English mirror of this document.

本文档描述 **山楂（hawthorn）** 在 **首发板卡（香橙派 5 / RK3588）** 上的启动假设、构建入口与尚未收敛项；与 [ARCHITECTURE.md](./ARCHITECTURE.md)、[KERNEL.md](./KERNEL.md) 配合阅读。

---

## 1. 构建前提

- 安装 **Rust**（推荐通过 [rustup](https://rustup.rs/)）；仓库根目录 [rust-toolchain.toml](../rust-toolchain.toml) 指定 **stable** 与目标 **`aarch64-unknown-none`**。
- 主机校验：

  ```bash
  cargo fmt --all -- --check
  cargo clippy --workspace --all-targets -- -D warnings
  cargo check -p hawthorn_kernel
  cargo check -p hawthorn_kernel --target aarch64-unknown-none
  cargo build -p hawthorn_kernel --features bare-metal --target aarch64-unknown-none
  cargo build -p hawthorn_qemu_minimal --features bare-metal --target aarch64-unknown-none
  ```

- 交叉目标由 `rustup` 根据 `rust-toolchain.toml` 安装；若缺失可执行：`rustup target add aarch64-unknown-none`。

### 1.1 Git pre-commit（可选）

仓库根目录 [`.pre-commit-config.yaml`](../.pre-commit-config.yaml) 定义 **`cargo fmt --check`**、**`cargo clippy --workspace -D warnings`**（与 CI 一致），以及 **`commit-msg`** 钩子（**`scripts/commit_msg_bilingual.py`**：英文 Conventional **第 1 行** + **第 2 行**中文、不同行）。安装 [pre-commit](https://pre-commit.com/) 后执行 `pre-commit install` 即可。详见 [CONTRIBUTING.md](../CONTRIBUTING.md) 与 [COMMIT_CONVENTIONS.md](./COMMIT_CONVENTIONS.md) §1.0。

### 1.2 QEMU `virt` 最小镜像（可选）

链接脚本 **`kernel/link-qemu_virt.ld`**（RAM **`0x4000_0000`** / 128 MiB、**`__stack_top`**、BSS 符号）为 **`hawthorn_kernel`** 与 **`hawthorn_qemu_minimal`** 共用，避免两处漂移。

- **`hawthorn_kernel`**：crate [`kernel/`](../kernel/) 在 **`--features bare-metal`** 且 **`--target aarch64-unknown-none`** 下可构建裸机二进制 **`hawthorn_kernel_qemu_virt`**（**`_start` → `kernel_main`**，PL011 **`0x9000_0000`**，panic 亦走 PL011）。校验：`cargo check -p hawthorn_kernel --target aarch64-unknown-none`（库）；完整镜像：`cargo build -p hawthorn_kernel --features bare-metal --target aarch64-unknown-none`。
- **`hawthorn_qemu_minimal`**（[`qemu_minimal/`](../qemu_minimal/)）：同上目标与 feature 下为独立冒烟 ELF；未启用 **`bare-metal`** 时两 crate 在主机上仅构建占位库，便于 **`cargo clippy --workspace`**。

- **构建（示例）：** `cargo build -p hawthorn_qemu_minimal --features bare-metal --target aarch64-unknown-none`（发布模式可加 `PROFILE=release` 配合下方脚本）。
- **运行：** 安装 **`qemu-system-aarch64`** 后执行 [`scripts/run_qemu_minimal.sh`](../scripts/run_qemu_minimal.sh)；脚本会在启动前向 **stderr** 打印进度（避免误以为「无反应」）。等价命令行使用 **`-machine virt -cpu cortex-a76 -nographic -kernel <ELF>`**（`-nographic` 将串口接到 stdio，**勿**再叠加 **`-serial stdio`**，否则易争用 stdio）。
- **一键自检（推荐）：** [`scripts/verify_qemu_minimal.sh`](../scripts/verify_qemu_minimal.sh) 会跑 **fmt / workspace clippy / 裸机 clippy+build / 短时 QEMU**，并检查是否出现 **`[hawthorn]`** 进度行；若串口行在捕获输出里不可见，脚本仍会报告「Rust 与脚本阶段已通过」（部分 CI/管道环境下 PL011 行不可见属常见现象）。
- **注意：** 在 **非交互式** 管道里跑 QEMU 时，可能看不到访客串口行；请在 **真实终端** 里执行脚本。若仍无输出，可改用 **`-serial pty`**，按 QEMU 提示在另一终端 **`cat /dev/pts/N`** 读取串口。

---

## 2. 启动链（待收敛）

当前 **开放决策**见 [ARCHITECTURE.md §10](./ARCHITECTURE.md)（启动与镜像格式、TF-A / U-Boot、FDT 等）。实现前需在 `bsp/orangepi5-rk3588/` 与 [BOOT.md](./BOOT.md) 中固定下列之一或组合：

- 由 **U-Boot**（或 Rockchip 既有流程）加载 **山楂（hawthorn）** 镜像的 **入口 EL、入口物理地址、设备树 blob 指针**；
- 或 **自研/极简 Boot stub** 与 TF-A 的移交契约。

---

## 3. 内存布局（占位）

| 区域 | 说明 |
|------|------|
| 内核镜像 | 加载地址与是否重定位：**TBD**（依赖 Boot 链） |
| 设备树 / 板级信息 | 指针是否经启动信息块传入：**TBD** |
| 早期栈与 BSS | 由链接脚本 `bsp/orangepi5-rk3588/` 定义 |

正式布局确定后，在本节或 **BOOT.md** 更新并链接 `linker.ld`。

---

## 4. 相关文档

| 文档 | 内容 |
|------|------|
| [BOOT.md](./BOOT.md) | 启动信息块与引导阶段（骨架） |
| [SYSCALL_ABI.md](./SYSCALL_ABI.md) | 系统调用 ABI（骨架） |
| [PLATFORMS.md](./PLATFORMS.md) | 平台 tier 列表 |
| [GLOSSARY.md](./GLOSSARY.md) | 术语 |
| [API.md](./API.md) | 对外 API 索引（占位） |

---

## 5. 公开 API 说明

用户态与内核之间的 **稳定接口** 以 `syscall_abi`（未来 crate）与 [SYSCALL_ABI.md](./SYSCALL_ABI.md) 为准；crate 级 API 索引见 [API.md](./API.md)。
