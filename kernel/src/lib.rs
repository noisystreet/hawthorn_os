//! 山楂（hawthorn）微内核 crate（占位骨架）。设计见仓库根目录 `docs/KERNEL.md` 与 `docs/ARCHITECTURE.md`。
#![cfg_attr(not(test), no_std)]

/// 占位符号，便于空 crate 在 host 与 `aarch64-unknown-none` 上通过 `cargo check`。
pub const PLACEHOLDER: u32 = 0;
