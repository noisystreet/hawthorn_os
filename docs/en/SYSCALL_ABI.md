# Syscall ABI (skeleton)

> **[中文](../SYSCALL_ABI.md)** — Chinese source of this document.

User services and the **Hawthorn (山楂) microkernel** interact through a **stable syscall surface**; implementation lives under `kernel/syscall/` and a future `syscall_abi` crate. This page tracks **pre-freeze** direction.

---

## 1. Calling convention (AArch64, TBD)

| Item | Direction |
|------|-----------|
| Trap mechanism | **`SVC`** into EL1 kernel (immediate vs `x8` syscall number: **TBD**) |
| Argument registers | **Subset** of AAPCS64 + Linux aarch64 syscall style, or **custom minimal set** (pick one) |
| Return value | `x0` = result or error code; errno vs POSIX subset: **TBD** |
| Volatile / saved | Document per AAPCS64 callee rules |

---

## 2. Syscall number space (placeholder)

Suggested partitions (numbers not assigned):

| Range use | Description |
|-----------|-------------|
| Threads & scheduling | exit, yield, priority, … |
| IPC | send, recv, reply, endpoint admin, … |
| Memory & capabilities | map, revoke, derive, … |
| IRQs & notifications | IRQ bind, notification signal, … |
| Debug | Optional; stripped in release |

When ABI freezes, fill this section and generate `syscall_abi` constants.

---

## 3. User stubs

- User code calls only through the **`syscall_abi` crate** or generated stubs; **no** link to private kernel symbols.  
- Matches dependency rules in [ARCHITECTURE.md §8](./ARCHITECTURE.md).

---

## Related documents

- [Microkernel design](./KERNEL.md)
- [Architecture](./ARCHITECTURE.md)
- [Porting](./PORTING.md)
