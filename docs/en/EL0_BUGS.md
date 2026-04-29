# EL0 User Mode Support — Bug List (Fixed)

> **[中文](../EL0_BUGS.md)** — 中文版文档。

This page preserves the EL0 bring-up defect history for traceability.

**Status update (2026-04-29):** Items 1–8 are fixed in order and validated in baseline runs (including `hello from EL0!` output).

---

## Current Status

- EL1 kernel functionality works: MMU ✅, GIC ✅, Timer ✅, Task Scheduling ✅, EL1 SVC syscall ✅
- EL0 user-mode path is enabled and validated: EL0 task creation works and prints `hello from EL0!`

---

## Bug List

### 🔴 Bug 1: `TABLE_APTABLE0` denies EL0 access to entire user page table subtree

- **File**: `kernel/src/mm.rs` (`make_table_desc` function, lines 131–134)
- **Symptom**: All intermediate page table descriptors (L0/L1/L2 table descriptors) have
  `TABLE_APTABLE0` (bit 61) set, which means "deny EL0 access to the entire subtree".
  Even if the L3 PTE has `PTE_AP_RW_ALL`, the hardware detects APTable[0]=1 in a parent
  descriptor during page table walk and generates a Permission fault on EL0 access.
- **Reference**: `aarch64_kernel`'s `identity.rs` also sets `TABLE_APTABLE0` for kernel
  page tables, but it has no user-mode requirements. Hawthorn needs to differentiate
  kernel and user page table descriptor attributes.
- **Fix direction**:
  1. Add `make_user_table_desc()` that does not set `TABLE_APTABLE0` (bit 61 cleared)
  2. Use user table descriptors in `map_user_page` / `create_user_page_table`
  3. Optional: also clear `TABLE_UXNTABLE` (bit 60) to allow EL0 execution on code pages

### 🔴 Bug 2: `clone_kernel_mappings` inherits kernel's `TABLE_APTABLE0` permissions

- **File**: `kernel/src/mm.rs` (`clone_kernel_mappings` function, lines 253–263)
- **Symptom**: The user page table directly copies kernel PGD entries, which contain
  `TABLE_APTABLE0` (bit 61). Even if the user page table maps its own code/stack pages,
  EL0 still cannot access them (APTable permissions inherited from kernel).
- **Fix direction**: When copying PGD entries, clear the `TABLE_APTABLE0` bit
  (`entry & !(1 << 61)`), or create independent user-mode PGD entries per Bug 1.

### 🔴 Bug 3: `user_return` assembly has incorrect `Task` struct offsets (off by +8)

- **File**: `kernel/src/trap.rs` (`user_return` assembly block, lines 405–475)
- **Symptom**: The assembly uses hardcoded offsets to access `Task` struct fields, but
  they don't match the actual `#[repr(C)]` layout:

  | Field              | Assembly Offset | Actual Offset | Delta |
  |--------------------|----------------|---------------|-------|
  | `user_page_table`  | `#56`          | **48**        | ❌ +8 |
  | `saved_elr`        | `#64`          | **56**        | ❌ +8 |
  | `saved_spsr`       | `#72`          | **64**        | ❌ +8 |
  | `saved_sp_el0`     | `#80`          | **72**        | ❌ +8 |

  Root cause: the assembly comments assumed `is_user: bool` (offset 40) needs 8 bytes
  of padding before `user_page_table: usize` (8-byte aligned), but only 7 bytes of
  padding are needed to reach offset 48.
- **Impact**: `eret` jumps to garbage address → synchronous exception → infinite loop;
  `TTBR0_EL1` loaded with wrong page table.
- **Fix direction**: Correct assembly offsets to `#48`/`#56`/`#64`/`#72`; or use Rust
  exported constants (`core::mem::offset_of!`) in assembly to avoid hardcoding.

### 🟡 Bug 4: `USER_PROGRAM` machine code `ldr x1, =msg` loads wrong address

- **File**: `kernel/src/boot_qemu_virt.rs` (`USER_PROGRAM` constant, line 30)
- **Symptom**: `0x58000001` decodes as `LDR X1, [PC, #0]`, loading 8 bytes from the
  current PC address. But PC contains the next instruction `mov x2, #16`
  (`0xd2800202`), not the message string address.
- **Impact**: EL0 `SYS_write` syscall receives a garbage pointer in `x1`, writing
  garbage data or triggering a fault.
- **Fix direction**: Use `ADR X1, msg` (PC-relative address calculation) instead of
  `LDR X1, =msg`, or place a correct literal pool entry after the instructions.

### 🟡 Bug 5: `user_task_trampoline` directly calls user code at EL1

- **File**: `kernel/src/task.rs` (`user_task_trampoline` assembly, lines 335–342)
- **Symptom**: `user_task_trampoline` jumps to the user entry point via `blr x19`,
  but the code is running at EL1 and should not directly execute EL0 code.
  The correct approach is to set `SPSR_EL1`/`ELR_EL1`/`SP_EL0` then `eret` to EL0.
- **Note**: In the current code path, `schedule()` calls `user_return()` instead of
  `context_switch()` when `is_user` is detected, so `user_task_trampoline`'s `blr x19`
  is effectively **dead code**. However, it's still a logic error that could cause
  issues if the scheduling path changes in the future.
- **Fix direction**: Modify `user_task_trampoline` to set EL0 state and `eret`,
  or remove this trampoline and unify initial entry via `user_return`.

### 🟡 Bug 6: `user_return` path skips `context_switch`, doesn't save current task sp

- **File**: `kernel/src/task.rs` (`schedule` function, lines 408–411)
- **Symptom**: When switching to a user task, `schedule()` calls
  `user_return(&mut TASK_TABLE[next])` directly instead of `context_switch()`. This causes:
  1. The current task's kernel stack pointer `sp` is not saved to `TASK_TABLE[current].sp`
  2. When switching back, the kernel stack is lost
  3. If switching from one EL0 task to another, both tasks' `sp` values are lost
- **Fix direction**: `schedule()` should always save the current task's `sp` via
  `context_switch()` first, then detect `is_user` in the context_switch recovery path
  and call `user_return`.

### 🟡 Bug 7: EL0 exception return path doesn't restore saved EL0 context

- **File**: `kernel/src/trap.rs` (return paths of `el0_irq_a64` / `el0_sync_a64`)
- **Symptom**: After EL0 exception handling completes, the assembly simply executes
  `eret`, but:
  - `handle_exception` calls `set_current_saved_context(elr, spsr, sp_el0)` to save old values
  - But it doesn't write back the saved values from Task to `ELR_EL1`/`SPSR_EL1`/`SP_EL0`
  - If `schedule()` switched tasks, `eret` returns to the wrong user task
- **Reference**: `aarch64_kernel`'s `vectors.S` explicitly restores `spsr_el1` and
  `elr_el1` in both `exception_exit` and `exit` paths
  (`msr spsr_el1, x2; msr elr_el1, x3`).
- **Fix direction**: In the EL0 exception return path, restore system registers from
  the current task's `saved_elr`/`saved_spsr`/`saved_sp_el0` before `eret`;
  or write back these registers before `handle_exception` returns.

### 🟠 Bug 8: `saved_spsr = 0x0000_0000` overwrites NZCV condition flags

- **File**: `kernel/src/task.rs` (`create_user` function, line 288)
- **Symptom**: SPSR value `0x0000_0000` clears NZCV condition flags (bits [31:28]),
  overwriting any condition flags the user program may have set.
- **Impact**: Acceptable for initial entry (EL0 program starts from scratch), but should
  be noted in comments. The real issue is whether SPSR is correctly restored on
  subsequent exception returns (see Bug 7).
- **Fix direction**: Low priority; improve comments only.

---

## Suggested Fix Priority

1. **Bug 1 + Bug 2** (page table permissions): most fundamental; without fix, any EL0 access faults
2. **Bug 3** (assembly offsets): without fix, `eret` jumps to invalid address
3. **Bug 4** (USER_PROGRAM machine code): without fix, syscall receives garbage pointer
4. **Bug 6** (schedule saving sp): without fix, user tasks cannot be correctly switched back
5. **Bug 7** (EL0 exception return path): without fix, `eret` returns to wrong task after scheduling
6. **Bug 5** (user_task_trampoline): currently dead code, lower priority
7. **Bug 8** (saved_spsr flag overwrite): minimal impact

---

## Verification Method

After each bug fix, run:

```bash
cargo build -p hawthorn_kernel --features bare-metal --target aarch64-unknown-none
timeout 10 qemu-system-aarch64 -machine virt,gic-version=3 -cpu cortex-a76 \
  -display none -serial file:/tmp/hawthorn_serial.log \
  -kernel target/aarch64-unknown-none/debug/hawthorn_kernel_qemu_virt
cat /tmp/hawthorn_serial.log
```

Ensure:
1. Existing EL1 functionality (task scheduling, SVC syscall) is not broken
2. New EL0 functionality gradually works (starting from "hello from EL0!" output)
