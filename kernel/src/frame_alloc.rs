// SPDX-License-Identifier: MIT OR Apache-2.0

//! Bump frame allocator for 4 KiB physical pages.
//!
//! Manages physical memory frames from RAM start to RAM end using a
//! bitmap. Each bit represents one 4 KiB frame; 1 = allocated, 0 = free.
//! Allocation is a linear scan (bump) from the last allocated position.

const PAGE_SIZE: usize = 4096;

const RAM_START: usize = 0x4000_0000;

const RAM_SIZE: usize = 128 * 1024 * 1024;

const TOTAL_FRAMES: usize = RAM_SIZE / PAGE_SIZE;

const BITMAP_WORDS: usize = (TOTAL_FRAMES + 63) / 64;

static mut BITMAP: [u64; BITMAP_WORDS] = [0; BITMAP_WORDS];

static mut NEXT_FRAME: usize = 0;

extern "C" {
    static __kernel_end: u8;
}

pub fn init() {
    let kernel_end = core::ptr::addr_of!(__kernel_end) as usize;
    let kernel_end_aligned = (kernel_end + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);

    let first_free_frame = (kernel_end_aligned - RAM_START) / PAGE_SIZE;

    for i in 0..first_free_frame {
        set_bit(i);
    }

    unsafe {
        NEXT_FRAME = first_free_frame;
    }

    let used_kb = first_free_frame * 4;
    crate::println!(
        "[frame_alloc] kernel end={:#x}, first free frame={}, used {} KiB",
        kernel_end_aligned,
        first_free_frame,
        used_kb
    );
}

fn set_bit(idx: usize) {
    let word = idx / 64;
    let bit = idx % 64;
    unsafe {
        BITMAP[word] |= 1u64 << bit;
    }
}

fn clear_bit(idx: usize) {
    let word = idx / 64;
    let bit = idx % 64;
    unsafe {
        BITMAP[word] &= !(1u64 << bit);
    }
}

fn is_set(idx: usize) -> bool {
    let word = idx / 64;
    let bit = idx % 64;
    unsafe { (BITMAP[word] >> bit) & 1 == 1 }
}

pub fn alloc_frame() -> Option<usize> {
    unsafe {
        let start = NEXT_FRAME;
        for i in 0..TOTAL_FRAMES {
            let idx = (start + i) % TOTAL_FRAMES;
            if !is_set(idx) {
                set_bit(idx);
                NEXT_FRAME = idx + 1;
                let paddr = RAM_START + idx * PAGE_SIZE;
                return Some(paddr);
            }
        }
        None
    }
}

pub fn alloc_zeroed_frame() -> Option<usize> {
    let paddr = alloc_frame()?;
    let ptr = paddr as *mut u8;
    unsafe {
        core::ptr::write_bytes(ptr, 0, PAGE_SIZE);
    }
    Some(paddr)
}

pub fn free_frame(paddr: usize) {
    if paddr < RAM_START || paddr >= RAM_START + RAM_SIZE {
        return;
    }
    if paddr % PAGE_SIZE != 0 {
        return;
    }
    let idx = (paddr - RAM_START) / PAGE_SIZE;
    clear_bit(idx);
}
