// SPDX-License-Identifier: MIT OR Apache-2.0

//! IRQ (Interrupt Request) dispatch framework.
//!
//! Provides a simple table-based interrupt handler registration and dispatch.
//! Supports up to 1020 interrupts (GICv3 maximum).
//!
//! Usage:
//! - Register a handler: `irq::register(irq_num, handler)`
//! - In trap handler: `irq::dispatch()` to route to registered handler

use crate::gic;

/// Maximum number of interrupts supported by GICv3.
const MAX_IRQ: usize = 1020;

/// IRQ handler function type.
pub type IrqHandler = fn();

/// IRQ handler table (sparse array of function pointers).
#[allow(static_mut_refs)]
static mut HANDLERS: [Option<IrqHandler>; MAX_IRQ] = [None; MAX_IRQ];

/// Spurious interrupt ID (1023).
const SPURIOUS_IRQ: u32 = 1023;

/// Initialize IRQ framework.
///
/// Must be called after GIC is initialized but before enabling interrupts.
#[allow(static_mut_refs)]
pub fn init() {
    // Clear all handlers using raw pointer to avoid static_mut_refs lint
    // SAFETY: single-core init, no concurrent access
    unsafe {
        let ptr = HANDLERS.as_mut_ptr();
        for i in 0..MAX_IRQ {
            core::ptr::write(ptr.add(i), None);
        }
    }
}

/// Register an IRQ handler.
///
/// # Arguments
/// * `irq` - Interrupt number (0-1019)
/// * `handler` - Handler function to call when IRQ fires
///
/// # Safety
/// Must not be called concurrently with `dispatch()`.
#[allow(static_mut_refs)]
pub unsafe fn register(irq: u32, handler: IrqHandler) {
    if irq as usize >= MAX_IRQ {
        return;
    }
    HANDLERS[irq as usize] = Some(handler);

    // Enable the interrupt in GIC
    if irq < 16 {
        // SGI - software generated, no enable needed
    } else if (16..32).contains(&irq) {
        // PPI
        gic::enable_ppi(irq);
    } else {
        // SPI
        gic::enable_spi(irq);
    }
}

/// Unregister an IRQ handler.
///
/// # Safety
/// Must not be called concurrently with `dispatch()`.
#[allow(static_mut_refs)]
pub unsafe fn unregister(irq: u32) {
    if irq as usize >= MAX_IRQ {
        return;
    }
    HANDLERS[irq as usize] = None;

    // Disable the interrupt in GIC
    if (16..32).contains(&irq) {
        gic::disable_ppi(irq);
    } else if irq >= 32 {
        gic::disable_spi(irq);
    }
}

/// Dispatch IRQ to registered handler.
///
/// Called from trap handler when IRQ exception occurs.
/// Returns true if a handler was invoked, false for spurious.
pub fn dispatch() -> bool {
    let irq = gic::ack();

    if irq == SPURIOUS_IRQ {
        return false;
    }

    // SAFETY: handler table access is read-only during dispatch
    let handler = unsafe { HANDLERS[irq as usize] };

    if let Some(h) = handler {
        h();
    }

    // SAFETY: irq matches the acknowledged interrupt
    unsafe {
        gic::eoi(irq);
    }

    true
}

/// Handle IRQ exception from trap module.
///
/// This is the entry point called by `trap::handle_exception`.
pub fn handle_irq_exception() {
    dispatch();
}
