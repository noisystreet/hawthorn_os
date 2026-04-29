# Syscall ABI (skeleton)

> **[中文](../SYSCALL_ABI.md)** — Chinese source of this document.

User services and the **Hawthorn (山楂) microkernel** interact through a **stable syscall surface**; implementation lives under `kernel/syscall/` and a future `syscall_abi` crate. This page tracks **pre-freeze** direction.

---

## 1. Calling convention (AArch64, TBD)

| Item | Direction |
|------|-----------|
| Trap mechanism | **`SVC`** into EL1 kernel (immediate vs `x8` syscall number: **TBD**) |
| Argument registers | **Subset** of AAPCS64 + Linux aarch64 syscall style, or **custom minimal set** (pick one) |
| Return value | `x0` = non-negative result on success; **errors as negative errno** (Linux aarch64 style; see `hawthorn_syscall_abi::Errno::as_u64`). See [BUGFIX_NOTES.md](./BUGFIX_NOTES.md), section 1. |
| Volatile / saved | Document per AAPCS64 callee rules |

---

## 2. Syscall numbers (draft, evolving)

The table below matches **`hawthorn_syscall_abi` today** (not frozen); semantics follow `kernel/src/syscall.rs`.

| # | Constant | Args (`x0`–`x5`) | `x0` return |
|---|----------|------------------|-------------|
| 0 | `SYS_WRITE` | `fd`, `buf`, `len` | bytes written; errors are negative errno |
| 1 | `SYS_READ` | (reserved) | `ENOSYS` |
| 2 | `SYS_YIELD` | — | `0` |
| 3 | `SYS_GETPID` | — | current task id |
| 4 | `SYS_EXIT` | `code` | does not return |
| 5 | `SYS_SLEEP` | `ms` | `0` |
| 6 | `SYS_ENDPOINT_CREATE` | — | endpoint id; table full → `ENOMEM` |
| 7 | `SYS_ENDPOINT_DESTROY` | `id` | `0` on success; `EINVAL` / `ENOENT` / `EPERM` |
| 8 | `SYS_ENDPOINT_CALL` | `id`, `msg` (MVP: low **32** bits) | success: **32**-bit reply; not ready → **`EAGAIN` (-11)** |
| 9 | `SYS_ENDPOINT_RECV` | `id` | success: `(client_id << 32) \| request` (each **32** bits); no message → **`EAGAIN` (-11)** |
| 10 | `SYS_ENDPOINT_REPLY` | `id`, `client_id`, `msg` (MVP: low **32** bits) | `0` on success |

Numbers may still change before freeze; track [TODO.md](./TODO.md) and the issue plan.

---

## 3. User stubs

- User code calls only through the **`syscall_abi` crate** or generated stubs; **no** link to private kernel symbols.  
- Matches dependency rules in [ARCHITECTURE.md §8](./ARCHITECTURE.md).

---

## Related documents

- [Important bugfix notes](./BUGFIX_NOTES.md)
- [Microkernel design](./KERNEL.md)
- [Architecture](./ARCHITECTURE.md)
- [Porting](./PORTING.md)
