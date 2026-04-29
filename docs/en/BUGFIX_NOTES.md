# Important bugfix notes (engineering log)

> **[中文](../BUGFIX_NOTES.md)** — Chinese source of this document.

This page records **landed fixes that affect behaviour or the syscall ABI**, for reviews, regressions, and later IPC/isolation work. Line references drift; treat the source tree as authoritative.

---

## 2026-04-29: syscall return encoding, `SYS_write` pointer rules, user-task teardown

### 1. `Errno::as_u64` did not match the documented `x0` error convention

| Item | Detail |
|------|--------|
| **Symptom** | For errors like `EFAULT`, `x0` sometimes held a **small positive** value (e.g. `14`), inconsistent with `hawthorn_syscall_abi` docs and with `is_error()` / `errno_from_ret()` which assume **negative errno** in `x0`. |
| **Cause** | `#[repr(i64)]` enum discriminants are **positive errno numbers**; the old `as_u64` path effectively published `self as i64 as u64`, i.e. a positive value, not two's-complement **`-errno`**. |
| **Fix** | `Errno::as_u64()`: `Ok` → `0`; otherwise **`(-(self as i64)) as u64`** (Linux-style **negative errno** in `x0`). `as_i64()` remains the **POSIX errno number** (docs clarify the split). |
| **Files** | `syscall_abi/src/lib.rs` |

---

### 2. `SYS_write` treated EL1 kernel buffers as user pointers

| Item | Detail |
|------|--------|
| **Symptom** | EL1 tasks passing a valid **kernel stack** buffer to `SYS_write` failed if validation only allowed a **user VA window** (e.g. `0x1000..0x8000`). |
| **Cause** | User-style `copy_from_user` rules were applied even when **`current_is_user()` was false**. |
| **Fix** | Branch on **`current_is_user()`**: **EL0** keeps the user VA range + safe read; **EL1** copies only from the **identity-mapped RAM window** aligned with `mm` / `frame_alloc` (`0x4000_0000..0x4800_0000`), otherwise returns `EFAULT` without dereferencing garbage. |
| **Files** | `kernel/src/syscall.rs` |

---

### 3. EL1 `SYS_write` on a bad pointer caused a Data Abort

| Item | Detail |
|------|--------|
| **Symptom** | Addresses like `0xdeadbeef` led to a **synchronous exception** (e.g. `EC=0x25` in logs) instead of `EFAULT`. |
| **Cause** | The EL1 fast path used `copy_nonoverlapping` **without** a prior range check. |
| **Fix** | Same RAM-window guard as above: reject with **`Errno::EFAULT`** before touching the mapping. |
| **Files** | `kernel/src/syscall.rs` |

---

### 4. User task exit did not free user frames / user page-table root

| Item | Detail |
|------|--------|
| **Symptom** | After `sys_exit`, user code/stack frames and the user PGD could stay allocated in the bump allocator, leaking on repeated `create_user`. |
| **Cause** | No central teardown; partial failure in `create_user` also lacked rollback. |
| **Fix** | Per-task side tables record **user-owned frames**; `exit_current` / `task_exit` call **`release_task_resources`**; failed `create_user` uses **`cleanup_user_allocation`**. |
| **Note** | Avoid embedding **large arrays inside `Task`**: the compiler may emit **SIMD / bulk memcpy**; with FP not enabled at EL1 this can fault (e.g. `EC=0x07`-class issues). Current code uses a **separate `static` table + scalar loops**. |
| **Files** | `kernel/src/task.rs` |

---

### 5. Regression coverage

| Item | Detail |
|------|--------|
| **Script** | `scripts/verify_kernel_qemu_virt_el0_serial.sh` checks banner, EL0 output, EL1 successful `SYS_write` length, bad pointer **`EFAULT (-14)`**, `sys_exit(0)` log line, and **`[task] released user resources:`**. |
| **Demo** | `kernel/src/boot_qemu_virt.rs` `task D` issues a bad-pointer `SYS_write` and prints a **signed** return for easy log diffing. |

---

## Related documents

- [EL0_BUGS.md](./EL0_BUGS.md) (historical EL0 bring-up bugs)
- [SYSCALL_ABI.md](./SYSCALL_ABI.md) (calling convention and return values)
- [KERNEL.md](./KERNEL.md) (kernel modules)
