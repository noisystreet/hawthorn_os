# M6 Remaining Work: User Tasks and EL0 Execution

> This document supplements [M6_PLAN.md](./M6_PLAN.md), focusing on remaining tasks after MMU enable.

## Current Status (M6 Completed)

| Component | Status | Description |
|-----------|--------|-------------|
| Frame Allocator | ✅ | `frame_alloc.rs` — bump allocator, bitmap for 32768 frames |
| 4-Level Page Tables | ✅ | `mm.rs` — 2MiB block mappings, identity map kernel RAM + devices |
| MMU Enable | ✅ | `enable_mmu()` — MAIR/TCR/TTBR0/SCTLR correctly configured, M/C/I all on |
| Syscall Dispatch | ✅ | `syscall.rs` + `trap.rs` — EL0/EL1 SVC both handled |

**Missing Core Capability**: Creating EL0 user tasks, context switch to user mode, return from user mode.

---

## Goal

Run the first EL0 user program on QEMU `virt`:
1. User program calls `svc #0` for `SYS_write` to output `"hello from EL0!"`
2. Calls `SYS_yield` to yield CPU
3. Calls `SYS_exit` to exit

---

## Module Change Plan

### 1. task.rs — TCB Extension and User Task Creation

#### 1.1 New TCB Fields

```rust
struct Task {
    // ... existing fields ...
    is_user: bool,           // Whether this is an EL0 user task
    user_page_table: usize,  // TTBR0_EL1 value (user page table physical address)
    saved_elr: u64,          // User return address (ELR_EL1)
    saved_spsr: u64,         // User PSTATE (SPSR_EL1)
    saved_sp_el0: u64,       // User stack pointer
}
```

#### 1.2 New Interface

```rust
/// Create an EL0 user task
/// 
/// # Arguments
/// - `entry`: User program entry virtual address
/// - `stack_top`: User stack top virtual address
/// 
/// # Returns
/// - `Some(TaskId)`: Success
/// - `None`: Task table full or out of memory
pub fn create_user(entry: usize, stack_top: usize) -> Option<TaskId>;
```

#### 1.3 Creation Flow

1. Allocate slot from `TASK_TABLE`
2. Allocate kernel stack (4K), set `TaskContext` (x19=entry, lr=`user_trap_return`)
3. `is_user = true`
4. `saved_elr = entry`
5. `saved_spsr = 0x0000_0000` (EL0t, AArch64, IRQ/FIQ/SError unmasked)
6. `saved_sp_el0 = stack_top`
7. Call `mm::create_user_page_table()` to create user page table
8. Map user code page + user stack page (identity map, can be changed to separate mapping later)
9. Set state to `Ready`

---

### 2. mm.rs — User Page Table Support

#### 2.1 New Interfaces

```rust
/// Create an empty user page table (copy kernel identity mapping)
pub fn create_user_page_table() -> Option<usize>;

/// Map a page in user page table
/// 
/// # Safety
/// Caller must ensure page table and physical frame are valid
pub unsafe fn map_user_page(table: usize, vaddr: usize, paddr: usize, attr: u64);
```

#### 2.2 Implementation Notes

- `create_user_page_table()`: Allocate new page table root, copy kernel PGD[0]'s L1 table pointer (share kernel mapping)
- User code/stack mappings in separate L2/L3 tables, not conflicting with kernel
- Attributes: `PTE_AP_RW_ALL` (EL0+EL1 RW), `PTE_UXN` (user cannot execute kernel code), `PTE_PXN` (kernel cannot execute user code, optional)

---

### 3. trap.rs — User Mode Return Path

#### 3.1 Problem Analysis

Current exception return paths share the same `eret` logic, but:
- **Kernel tasks**: `eret` back to EL1 (current implementation)
- **User tasks**: `eret` needs to return to EL0, and must restore `SP_EL0`, `ELR_EL1`, `SPSR_EL1`, switch `TTBR0_EL1`

#### 3.2 Solution

After `handle_exception` returns, choose return path based on current task type:

**Option A: Dispatch in Rust (recommended)**

```rust
// At end of handle_exception
if crate::task::current_is_user() {
    // Jump to assembly user_return
} else {
    // Normal kernel return (current path)
}
```

**Option B: Pure assembly check**

In `el0_sync_a64` / `el0_irq_a64` return path, check `CURRENT_TASK.is_user` flag, branch to different restore logic.

#### 3.3 User Mode Return Assembly (`user_return`)

```asm
user_return:
    // Restore user-specific registers
    ldr x0, [current_task + offset_saved_sp_el0]
    msr sp_el0, x0
    
    ldr x0, [current_task + offset_saved_elr]
    msr elr_el1, x0
    
    ldr x0, [current_task + offset_saved_spsr]
    msr spsr_el1, x0
    
    // Switch user page table
    ldr x0, [current_task + offset_user_page_table]
    msr ttbr0_el1, x0
    isb
    tlbi vmalle1is
    isb
    
    // Restore general registers (x0-x30 already in TrapFrame)
    // ... restore x0-x30 from stack ...
    
    eret  // Return to EL0
```

#### 3.4 Key Modification Points

- `el0_sync_a64` and `el0_irq_a64` return paths need to distinguish kernel/user
- Add `user_return` global assembly symbol
- `handle_exception` no longer directly `eret`, but jumps based on task type

---

### 4. boot_qemu_virt.rs — Integration Test

#### 4.1 Embedded User Program

Use `global_asm!` to define user program, linker script exports symbols to get byte range:

```rust
global_asm!(
    ".section .user_program, \"ax\"",
    ".global __user_program_start",
    ".global __user_program_end",
    "__user_program_start:",
    // mov x0, #1              // fd = stdout
    "mov x0, #1",
    // adr x1, msg             // msg address (PC-relative)
    "adr x1, 1f",
    // mov x2, #16             // len = 16
    "mov x2, #16",
    // mov x8, #0              // SYS_write
    "mov x8, #0",
    // svc #0
    "svc #0",
    // mov x8, #2              // SYS_yield
    "mov x8, #2",
    // svc #0
    "svc #0",
    // mov x8, #1              // SYS_exit
    "mov x8, #1",
    // mov x0, #0              // exit code = 0
    "mov x0, #0",
    // svc #0
    "svc #0",
    "1:",
    ".asciz \"hello from EL0!\\n\"",
    "__user_program_end:",
);

extern "C" {
    static __user_program_start: u8;
    static __user_program_end: u8;
}
```

#### 4.2 Initialization Flow

```rust
pub extern "C" fn kernel_main() -> ! {
    // ... existing init: frame_alloc, mm, trap, gic, irq, timer, task, syscall ...
    
    // Create kernel demo tasks (existing)
    // ...
    
    // Create EL0 user task
    let user_entry = unsafe { &__user_program_start as *const _ as usize };
    let user_stack_top = alloc_user_stack();  // Allocate user stack (4K)
    
    if let Some(user_id) = task::create_user(user_entry, user_stack_top) {
        crate::println!("[kernel] created user task {:?}", user_id);
    }
    
    // Start scheduling
    task::yield_now();
    
    // idle loop
    loop {
        task::yield_now();
    }
}
```

---

## Implementation Order

```
Step 1: mm.rs — Add create_user_page_table() and map_user_page()
Step 2: task.rs — Extend TCB, implement create_user()
Step 3: trap.rs — Add user_return assembly, modify el0_* return paths
Step 4: link-qemu_virt.ld — Add .user_program section
Step 5: boot_qemu_virt.rs — Embed user program, create user task
Step 6: QEMU verification — Expect "hello from EL0!" output
Step 7: Code cleanup — Remove debug output, eliminate warnings
```

---

## Verification Criteria

```bash
cargo build -p hawthorn_kernel --features bare-metal --target aarch64-unknown-none
qemu-system-aarch64 -machine virt,gic-version=3 -cpu cortex-a57 -display none \
    -serial stdio -kernel target/aarch64-unknown-none/debug/hawthorn_kernel_qemu_virt
```

**Expected Output**:
```
[kernel] created user task TaskId(4)
hello from EL0!
[task A] round 0
[task B] round 0
...
```

---

## Risks and Notes

1. **Page Table Switch Timing**: After `TTBR0_EL1` switch, must `tlbi vmalle1is` + `isb`, otherwise TLB cache causes wrong translation
2. **SPSR Setting**: `saved_spsr` must set `M[3:0] = 0b0000` (EL0t), otherwise `eret` goes to wrong exception level
3. **User Stack Guard Page**: MVP can skip, but suggest keeping 4K unmapped as guard to detect stack overflow
4. **Kernel/User Page Table Isolation**: Current scheme shares L1 table (kernel mapping), user code/stack in separate L2. Should fully separate TTBR0/TTBR1 later
5. **Debug Difficulty**: EL0 exceptions (e.g., Data Abort) enter `el0_serror_a64`, ensure that path can print diagnostic info

---

## Related Documents

- [M6_PLAN.md](./M6_PLAN.md) — M6 overall plan (including completed parts)
- [M6_MMU_DEBUG_LOG.md](./M6_MMU_DEBUG_LOG.md) — MMU debugging and fix record
- [PR_ISSUE_PLAN.md](./PR_ISSUE_PLAN.md) — M7 IPC planning
- [KERNEL.md](./KERNEL.md) — Kernel design document
