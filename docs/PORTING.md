# 山楂 / hawthorn 移植与运行环境

> **[English](./en/PORTING.md)** — English mirror of this document.

本文档描述 **山楂（hawthorn）** 在 **首发板卡（香橙派 5 / RK3588）** 上的启动假设、构建入口与尚未收敛项；与 [ARCHITECTURE.md](./ARCHITECTURE.md)、[KERNEL.md](./KERNEL.md) 配合阅读。

---

## 1. 构建前提

- 安装 **Rust**（推荐通过 [rustup](https://rustup.rs/)）；仓库根目录 [rust-toolchain.toml](../rust-toolchain.toml) 指定 **stable** 与目标 **`aarch64-unknown-none`**。
- 主机校验：

  ```bash
  cargo fmt --all -- --check
  cargo clippy -p hawthorn_kernel --all-targets --all-features
  cargo check -p hawthorn_kernel
  cargo check -p hawthorn_kernel --target aarch64-unknown-none
  ```

- 交叉目标由 `rustup` 根据 `rust-toolchain.toml` 安装；若缺失可执行：`rustup target add aarch64-unknown-none`。

### 1.1 Git pre-commit（可选）

仓库根目录 [`.pre-commit-config.yaml`](../.pre-commit-config.yaml) 定义 **`cargo fmt --check`**、**`cargo clippy -D warnings`**（与 CI 一致），以及 **`commit-msg`** 钩子（**`scripts/commit_msg_bilingual.py`**：英文 Conventional **第 1 行** + **第 2 行**中文、不同行）。安装 [pre-commit](https://pre-commit.com/) 后执行 `pre-commit install` 即可。详见 [CONTRIBUTING.md](../CONTRIBUTING.md) 与 [COMMIT_CONVENTIONS.md](./COMMIT_CONVENTIONS.md) §1.0。

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
