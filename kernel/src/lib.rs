//! 山楂（hawthorn）微内核 crate。QEMU `virt` 最小引导见 `boot_qemu_virt` 模块（`aarch64-unknown-none`）；设计见 `docs/KERNEL.md` 与 `docs/ARCHITECTURE.md`。
#![cfg_attr(not(test), no_std)]

#[cfg(all(target_arch = "aarch64", target_os = "none"))]
pub mod boot_qemu_virt;

#[cfg(all(target_arch = "aarch64", target_os = "none"))]
pub mod console;

#[cfg(any(all(target_arch = "aarch64", target_os = "none"), test))]
pub mod endpoint;

#[cfg(any(all(target_arch = "aarch64", target_os = "none"), test))]
pub mod frame_alloc;

#[cfg(all(target_arch = "aarch64", target_os = "none"))]
pub mod gic;

#[cfg(all(target_arch = "aarch64", target_os = "none"))]
pub mod irq;

#[cfg(all(target_arch = "aarch64", target_os = "none"))]
pub mod mm;

#[cfg(all(target_arch = "aarch64", target_os = "none"))]
pub mod syscall;

#[cfg(all(target_arch = "aarch64", target_os = "none"))]
pub mod task;

pub mod task_policy;

/// Layout of the AArch64 trap frame; host-testable (see unit tests).
pub mod trap_frame;

#[cfg(all(target_arch = "aarch64", target_os = "none"))]
pub mod timer;

#[cfg(all(target_arch = "aarch64", target_os = "none"))]
pub mod trap;

#[cfg(all(target_arch = "aarch64", target_os = "none"))]
pub mod user_layout;

/// 占位符号，便于空 crate 在 host 上通过 `cargo check`（裸机路径见 `boot_qemu_virt`）。
#[cfg(not(all(target_arch = "aarch64", target_os = "none")))]
pub const PLACEHOLDER: u32 = 0;
