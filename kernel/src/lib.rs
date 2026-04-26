//! 山楂（hawthorn）微内核 crate。QEMU `virt` 最小引导见 `boot_qemu_virt` 模块（`aarch64-unknown-none`）；设计见 `docs/KERNEL.md` 与 `docs/ARCHITECTURE.md`。
#![cfg_attr(not(test), no_std)]

#[cfg(all(target_arch = "aarch64", target_os = "none"))]
pub mod boot_qemu_virt;

#[cfg(all(target_arch = "aarch64", target_os = "none"))]
pub mod console;

#[cfg(all(target_arch = "aarch64", target_os = "none"))]
pub mod trap;

/// 占位符号，便于空 crate 在 host 上通过 `cargo check`（裸机路径见 `boot_qemu_virt`）。
#[cfg(not(all(target_arch = "aarch64", target_os = "none")))]
pub const PLACEHOLDER: u32 = 0;
