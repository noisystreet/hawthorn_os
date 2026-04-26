// SPDX-License-Identifier: MIT OR Apache-2.0

//! QEMU `virt` AArch64 bring-up: BSS, PL011 @ `0x9000_0000`, and `kernel_main`.

use core::arch::asm;
use core::fmt;

/// PL011 base on QEMU `virt` AArch64 (DT `pl011@9000000`).
pub const PL011_BASE: usize = 0x900_0000;
const PL011_DR: usize = PL011_BASE;
const PL011_IBRD: usize = PL011_BASE + 0x24;
const PL011_FBRD: usize = PL011_BASE + 0x28;
const PL011_LCR_H: usize = PL011_BASE + 0x2c;
const PL011_CR: usize = PL011_BASE + 0x30;

extern "C" {
    static __bss_start: u8;
    static __bss_end: u8;
}

/// Write a 32-bit value to a MMIO address.
///
/// Uses inline assembly to avoid `core::ptr::write_volatile` debug assertions
/// that may panic in bare-metal environments without a working panic handler.
///
/// # Safety
///
/// `addr` must be a valid, aligned MMIO address.
#[inline(always)]
unsafe fn mmio_write32(addr: usize, val: u32) {
    asm!("str {val:w}, [{addr}]",
         addr = in(reg) addr,
         val = in(reg) val,
         options(nostack, preserves_flags));
}

/// Write a byte to a MMIO address.
///
/// # Safety
///
/// `addr` must be a valid, aligned MMIO address.
#[inline(always)]
unsafe fn mmio_write8(addr: usize, val: u8) {
    asm!("strb {val:w}, [{addr}]",
         addr = in(reg) addr,
         val = in(reg) val,
         options(nostack, preserves_flags));
}

/// Zero `.bss` before using any static `mut` or `static` that lives in BSS.
///
/// # Safety
///
/// Linker symbols `__bss_start` / `__bss_end` must bound the BSS range in RAM.
pub unsafe fn zero_bss() {
    let start = core::ptr::addr_of!(__bss_start) as usize;
    let end = core::ptr::addr_of!(__bss_end) as usize;
    let mut p = start;
    while p < end {
        mmio_write8(p, 0);
        p += 1;
    }
}

/// Zero-sized token representing the PL011 UART on QEMU `virt`.
///
/// Obtain via [`pl011()`] after [`pl011_init()`]. Implements [`fmt::Write`]
/// so you can use `write!` / `writeln!` for formatted output.
pub struct Pl011 {
    _private: (),
}

/// Returns a [`Pl011`] writer handle.
///
/// # Safety
///
/// Caller must ensure [`pl011_init()`] has been called and the MMIO region is valid.
pub unsafe fn pl011() -> Pl011 {
    Pl011 { _private: () }
}

/// Minimal PL011 init (8n1, UART enabled). Safe after MMIO region is reachable.
///
/// # Safety
///
/// Caller must be on QEMU `virt` AArch64 (or compatible PL011 at [`PL011_BASE`]).
pub unsafe fn pl011_init() {
    mmio_write32(PL011_CR, 0);
    mmio_write32(PL011_IBRD, 1);
    mmio_write32(PL011_FBRD, 0);
    mmio_write32(PL011_LCR_H, 0x70);
    mmio_write32(PL011_CR, 0x301);
}

/// Write raw bytes to PL011 TX (blocking-ish: FIFO on QEMU is generous).
///
/// # Safety
///
/// UART must be initialized ([`pl011_init`]) and MMIO must be valid.
pub unsafe fn pl011_write_bytes(bytes: &[u8]) {
    for &b in bytes {
        pl011_putc(b);
    }
}

unsafe fn pl011_putc(byte: u8) {
    mmio_write32(PL011_DR, u32::from(byte));
    asm!("dsb sy", options(nostack, preserves_flags));
}

impl fmt::Write for Pl011 {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        // SAFETY: caller of `pl011()` guaranteed UART is initialized.
        unsafe { pl011_write_bytes(s.as_bytes()) };
        Ok(())
    }
}

/// Rust entry from `_start` (see `src/bin/qemu_virt.rs`): BSS → UART → trap → GIC → IRQ → banner → idle loop.
#[no_mangle]
pub extern "C" fn kernel_main() -> ! {
    // SAFETY: early boot on `virt`; linker defines BSS bounds.
    unsafe { zero_bss() };
    // SAFETY: fixed PL011 mapping for this platform.
    unsafe { pl011_init() };
    // Install exception vector table before any operation that may fault.
    crate::trap::init();
    // SAFETY: GICv3 MMIO base addresses are fixed on QEMU `virt`.
    unsafe { crate::gic::init() };
    // Initialize IRQ dispatch table (must follow GIC init).
    crate::irq::init();
    // Initialize ARM Generic Timer (must follow IRQ init; registers PPI 30 handler).
    crate::timer::init();
    // Initialize cooperative task scheduler.
    crate::task::init();
    // Initialize syscall dispatch table.
    crate::syscall::init();
    // Create demo tasks: task_d tests SVC syscall path from EL1.
    extern "C" fn task_a() {
        for i in 0..3 {
            crate::println!("[task A] round {}", i);
            crate::task::sleep(500);
        }
        crate::println!("[task A] done");
    }
    extern "C" fn task_b() {
        for i in 0..3 {
            crate::println!("[task B] round {}", i);
            crate::task::sleep(300);
        }
        crate::println!("[task B] done");
    }
    extern "C" fn task_d() {
        crate::println!("[task D] testing syscall via SVC...");

        let pid: u64;
        unsafe {
            asm!(
                "mov x8, #3",
                "svc #0",
                "mov {}, x0",
                out(reg) pid,
            );
        }
        crate::println!("[task D] SYS_getpid returned {}", pid);

        let msg = b"hello from SVC write!\n";
        let len = msg.len() as u64;
        let ptr = msg.as_ptr() as u64;
        let ret: u64;
        unsafe {
            asm!(
                "mov x8, #0",
                "mov x0, #1",
                "mov x1, {ptr}",
                "mov x2, {len}",
                "svc #0",
                "mov {ret}, x0",
                ptr = in(reg) ptr,
                len = in(reg) len,
                ret = out(reg) ret,
            );
        }
        crate::println!("[task D] SYS_write returned {}", ret);

        crate::println!("[task D] done");
    }
    crate::task::create(task_a, 1);
    crate::task::create(task_b, 1);
    crate::task::create(task_d, 1);
    crate::println!("[task] created tasks A, B, D (syscall test)");
    // Enable IRQ exceptions at EL1 (clear DAIF.I bit).
    unsafe { asm!("msr daifclr, #2") };
    // SAFETY: UART initialized above.
    crate::println!("Hawthorn: hawthorn_kernel on QEMU virt OK");
    loop {
        crate::task::yield_now();
    }
}
