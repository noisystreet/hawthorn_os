// SPDX-License-Identifier: MIT OR Apache-2.0

//! ARM Generic Timer driver for QEMU `virt` AArch64.
//!
//! Provides the EL1 Physical Timer (PPI IRQ 30) as the scheduling tick.
//! On QEMU `virt`, the counter frequency is typically ~24 MHz but is read
//! from `CNTFRQ_EL0` at init time.
//!
//! System registers used:
//! - `CNTFRQ_EL0`  — counter frequency (read-only at EL1, set by firmware)
//! - `CNTPCT_EL0`  — physical counter value (read-only)
//! - `CNTP_CTL_EL0` — timer enable / IMASK / ISTATUS
//! - `CNTP_TVAL_EL0` — timer value (write-only: sets comparator = CNTPCT + TVAL)
//!
//! Reference: ARM DDI 0487, D13.2 (Generic Timer)

use core::arch::asm;

use crate::irq;

/// EL1 Physical Timer PPI interrupt number.
const TIMER_IRQ: u32 = 30;

/// Default tick interval in microseconds.
const DEFAULT_TICK_US: u64 = 10_000;

/// Counter frequency in Hz (read from CNTFRQ_EL0 at init).
static mut TIMER_FREQ: u64 = 0;

/// Tick interval in counter ticks.
static mut TICK_INTERVAL: u64 = 0;

/// Number of timer ticks since boot (monotonic, wraps on overflow).
static mut TICK_COUNT: u64 = 0;

/// Read counter frequency from CNTFRQ_EL0.
fn read_freq() -> u64 {
    let freq: u64;
    unsafe { asm!("mrs {}, cntfrq_el0", out(reg) freq) };
    freq
}

/// Read current physical counter value from CNTPCT_EL0.
pub fn read_counter() -> u64 {
    let cnt: u64;
    unsafe { asm!("mrs {}, cntpct_el0", out(reg) cnt) };
    cnt
}

/// Disable the EL1 Physical Timer (CNTP_CTL_EL0.ENABLE = 0).
fn disable() {
    unsafe { asm!("msr cntp_ctl_el0, {}", in(reg) 0u64) };
}

/// Enable the EL1 Physical Timer (CNTP_CTL_EL0.ENABLE = 1, IMASK = 0).
fn enable() {
    unsafe { asm!("msr cntp_ctl_el0, {}", in(reg) 1u64) };
}

/// Set the timer to fire after `ticks` counter cycles from now.
///
/// Writes to CNTP_TVAL_EL0 which sets the comparator to CNTPCT + ticks.
fn set_interval(ticks: u64) {
    let tval = ticks as u32;
    unsafe { asm!("msr cntp_tval_el0, {}", in(reg) tval as u64) };
}

/// Timer interrupt handler (called from IRQ dispatch).
fn handle_irq() {
    unsafe {
        TICK_COUNT += 1;
    }

    let count = unsafe { TICK_COUNT };
    if count % 100 == 0 {
        crate::println!("[timer] tick #{}", count);
    }

    set_interval(unsafe { TICK_INTERVAL });
}

/// Initialize the ARM Generic Timer.
///
/// Must be called after GIC and IRQ framework init. Registers the timer
/// IRQ handler and starts the periodic tick.
pub fn init() {
    unsafe {
        TIMER_FREQ = read_freq();
    }

    let freq = unsafe { TIMER_FREQ };
    if freq == 0 {
        crate::println!("[timer] WARNING: CNTFRQ_EL0 = 0, assuming 24 MHz");
        unsafe {
            TIMER_FREQ = 24_000_000;
        }
    }

    let freq = unsafe { TIMER_FREQ };
    unsafe {
        TICK_INTERVAL = freq / 1_000_000 * DEFAULT_TICK_US;
    }

    let interval = unsafe { TICK_INTERVAL };
    crate::println!(
        "[timer] freq={} Hz, tick={} ms, interval={} ticks",
        freq,
        DEFAULT_TICK_US / 1000,
        interval
    );

    disable();
    set_interval(interval);

    unsafe {
        irq::register(TIMER_IRQ, handle_irq);
    }

    enable();
}

/// Get the current tick count (monotonic since boot).
pub fn tick_count() -> u64 {
    unsafe { TICK_COUNT }
}

/// Get the counter frequency in Hz.
pub fn frequency() -> u64 {
    unsafe { TIMER_FREQ }
}
