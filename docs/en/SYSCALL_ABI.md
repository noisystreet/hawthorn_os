# Syscall ABI (DRAFT-1.0 · M7 Phase 1 tighten)

> **[中文](../系统调用ABI.md)** — Chinese source.

User code and the **Hawthorn (山楂) microkernel** interact through a **stable syscall surface**. This document matches crate **`hawthorn_syscall_abi`** (`syscall_abi/`) and **`kernel/src/syscall.rs`**, labelled **DRAFT-1.0**; the numeric epoch is **`hawthorn_syscall_abi::ABI_VERSION`** (currently **1**). No **long-term** compatibility freeze yet; a **STABLE-x** line will be announced here and in release notes when adopted.

---

## 1. Number space and naming

| Tag | Meaning |
|-----|---------|
| **DRAFT-x.y** | Matches the tree today; syscall layout may still change before release (bump `ABI_VERSION` + edit this doc). |
| **STABLE-x** | (Future) no **breaking** number or layout changes within the declared major line; new numbers / capability bits may be added. |

**Number ranges** (same constants as `hawthorn_syscall_abi`):

| Range | Use |
|-------|-----|
| `0..=SYSCALL_NR_CORE_MAX` (63) | **Kernel core fixed table** — `hawthorn_kernel::syscall` dispatch |
| `64..=255` | **Reserved** — future fixed assignment |
| `≥256` | **Reserved** — dynamic policy **TBD** |

**Unimplemented** core slots and **out-of-range** numbers return **`ENOSYS`** (`x0 = -38`).

---

## 2. AArch64 trap and registers (fixed direction)

| Item | Rule |
|------|------|
| Trap | **`SVC #0`** (EL0 or EL1; the **immediate is ignored** for dispatch) |
| Syscall number | **`x8`** |
| Arguments | **`x0`–`x5`**, up to **6** scalars (`hawthorn_syscall_abi::SYSCALL_MAX_ARGS == 6`) |
| Return | **`x0`**: success is non-negative / `0`; errors are **negative errno** (Linux aarch64 style, see `Errno::as_u64`) |
| Volatile | As AAPCS64 / Linux syscall habit: **`x0`–`x17`** may be clobbered across `SVC`; the user saves what it needs. |

**vs generic PCS**: treat `SVC` as a **C ABI boundary**; after return **`x0`** and callee-saved **`x19`–`x28`**, **`x29`**, **`x30`** follow the toolchain contract (MVP focuses on single EL0 tasks).

---

## 3. `SYS_ABI_INFO` and capability bits

| # | Constant | Args | `x0` return |
|---|----------|------|-------------|
| **11** | **`SYS_ABI_INFO`** | ignored | **non-negative**: low **32** bits = `ABI_VERSION`; high **32** bits = OR of `ABI_CAP_*` (**not** errno) |

Current flags:

- **`ABI_CAP_EL0_USER_AS`**: kernel maintains an **EL0 user address space** (fixed low VA window + user pointer checks), aligned with `user_layout` / `mm`.

---

## 4. Syscall table (core `0..=11`)

Semantics follow **`kernel/src/syscall.rs`**.

| # | Constant | Args (`x0`–`x5`) | `x0` return |
|---|----------|------------------|-------------|
| 0 | `SYS_WRITE` | `fd`, `buf`, `len` | bytes written; errors negative errno (`fd` currently **1** only; user buffer validated) |
| 1 | `SYS_READ` | (reserved) | **`ENOSYS`** — explicit stub until implemented |
| 2 | `SYS_YIELD` | — | `0` |
| 3 | `SYS_GETPID` | — | current task id |
| 4 | `SYS_EXIT` | `code` | does not return |
| 5 | `SYS_SLEEP` | `ms` | `0` |
| 6 | `SYS_ENDPOINT_CREATE` | — | endpoint id; table full → `ENOMEM` |
| 7 | `SYS_ENDPOINT_DESTROY` | `id` | `0` on success; `EINVAL` / `ENOENT` / `EPERM` |
| 8 | `SYS_ENDPOINT_CALL` | `id`, `msg` (**Phase 1:** kernel stores `msg & ENDPOINT_INLINE_REQ_MASK`; see §4.1) | **`u64`** value from `reply`; **blocks** when no rendezvous peer yet |
| 9 | `SYS_ENDPOINT_RECV` | `id` | success: `hawthorn_syscall_abi::endpoint_recv_pack(client_id, request)` (**32**-bit task id + **32**-bit request); **blocks** when no pending `call` |
| 10 | `SYS_ENDPOINT_REPLY` | `id`, `client_id`, `msg` | `0` on success; `msg` is a full **`u64`** delivered to the woken `call` |
| 11 | `SYS_ABI_INFO` | ignored | §3 |

`12..=63`: **unassigned** → **`ENOSYS`**.

### 4.1 M7 Phase 1: inline endpoint scalar

- **`ENDPOINT_INLINE_REQ_MASK`** (`0xFFFF_FFFF`): the request seen after `call` / in `recv` is **`msg & ENDPOINT_INLINE_REQ_MASK`** (same definition in `hawthorn_syscall_abi`).
- **`endpoint_recv_pack` / `endpoint_recv_unpack`**: decode a successful **`SYS_ENDPOINT_RECV`** return (high **32** bits `client_id`, low **32** bits request).
- **User AArch64**: `hawthorn_syscall_abi::user` provides thin `SVC #0` wrappers (`raw_syscall6`, `sys_*`).

---

## 5. Errors and decoding

- **`hawthorn_syscall_abi::Errno`**: shared; `as_u64()` encodes **`x0 = -errno`**.
- Helpers: `is_error`, `errno_from_ret` (covers the errno values listed in the crate).
- Notes: [BUGFIX_NOTES.md](./BUGFIX_NOTES.md), section 1.

---

## 6. User stubs

- Call only through **`hawthorn_syscall_abi`** (`user` submodule + constants / `pack` helpers); **do not** link private kernel symbols.  
- Matches [ARCHITECTURE.md §8](./ARCHITECTURE.md).

---

## Related documents

- [BUGFIX_NOTES.md](./BUGFIX_NOTES.md)
- [KERNEL.md](./KERNEL.md)
- [ARCHITECTURE.md](./ARCHITECTURE.md)
- [PORTING.md](./PORTING.md)
- [TRAP.md](./TRAP.md) (`SVC` / `x8` path)
