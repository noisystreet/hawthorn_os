// SPDX-License-Identifier: MIT OR Apache-2.0

//! Console abstraction over the PL011 UART.
//!
//! Provides [`print!`] and [`println!`] macros for formatted debug output.
//! The UART must be initialized via [`boot_qemu_virt::pl011_init()`] before
//! any write; otherwise the behaviour is undefined.

use crate::boot_qemu_virt;
use core::fmt;

/// Writes formatted arguments to the PL011 UART.
///
/// # Safety
///
/// [`boot_qemu_virt::pl011_init()`] must have been called beforehand.
pub unsafe fn _print(args: fmt::Arguments) {
    let mut uart = boot_qemu_virt::pl011();
    fmt::Write::write_fmt(&mut uart, args).unwrap();
}

/// Prints to the PL011 UART (no newline).
///
/// # Safety
///
/// UART must be initialized. Intended for use inside `unsafe` blocks or
/// after early init has completed.
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {{
        let args = format_args!($($arg)*);
        #[allow(unused_unsafe)]
        // SAFETY: caller guarantees UART is initialized.
        unsafe { $crate::console::_print(args) };
    }};
}

/// Prints to the PL011 UART (with newline).
///
/// # Safety
///
/// Same as [`print!`].
#[macro_export]
macro_rules! println {
    () => { $crate::print!("\n") };
    ($($arg:tt)*) => {{
        let args = format_args!($($arg)*);
        #[allow(unused_unsafe)]
        // SAFETY: caller guarantees UART is initialized.
        unsafe { $crate::console::_print(args) };
        $crate::print!("\n");
    }};
}
