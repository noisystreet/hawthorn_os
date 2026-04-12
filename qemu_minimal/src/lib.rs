//! `hawthorn_qemu_minimal`：主机上为 **空库**（供 `cargo clippy --workspace`）；裸机 ELF 由同名 binary 构建。
//!
//! 构建镜像：`cargo build -p hawthorn_qemu_minimal --features bare-metal --target aarch64-unknown-none`

#![no_std]
#![deny(missing_docs)]

/// 提示如何构建 QEMU 镜像（文档与 IDE 悬停用）。
pub const QEMU_BUILD_HINT: &str =
    "cargo build -p hawthorn_qemu_minimal --features bare-metal --target aarch64-unknown-none";
