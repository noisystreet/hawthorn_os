# 系统调用 ABI（骨架）

> **[English](./en/SYSCALL_ABI.md)** — English mirror of this document.

用户态服务与 **山楂（hawthorn）微内核** 通过 **稳定 syscall 界面** 交互；实现细节在 `kernel/syscall/` 与未来 `syscall_abi` crate 中落地。本文跟踪 **冻结前** 的约定方向。

---

## 1. 调用约定（AArch64，待冻结）

| 项目 | 方向 |
|------|------|
| 陷入方式 | **`SVC`** 进入 EL1 内核（具体立即数或 x8 传 syscall 号：**TBD**） |
| 参数寄存器 | 遵循 AAPCS64 与 Linux aarch64 syscall 惯例的 **子集** 或 **自定义最小集**（二选一拍板） |
| 返回值 | `x0` = 成功时为非负结果；**错误时为负 errno**（与 Linux aarch64 惯例一致，见 `hawthorn_syscall_abi::Errno::as_u64`）。说明见 [BUGFIX_NOTES.md](./BUGFIX_NOTES.md) 第 1 节。 |
| 易失 / 保存寄存器 | 按 AAPCS64 被调用方保存约定文档化 |

---

## 2. 系统调用号（草稿，随实现演进）

以下为 `hawthorn_syscall_abi` 当前已分配编号（**未冻结**）；语义以 `kernel/src/syscall.rs` 为准。

| 编号 | 常量 | 参数（`x0`–`x5`） | 返回值 `x0` |
|------|------|-------------------|-------------|
| 0 | `SYS_WRITE` | `fd`, `buf`, `len` | 写入字节数；错误为负 errno |
| 1 | `SYS_READ` | （预留） | `ENOSYS` |
| 2 | `SYS_YIELD` | — | `0` |
| 3 | `SYS_GETPID` | — | 当前任务 id |
| 4 | `SYS_EXIT` | `code` | 不返回 |
| 5 | `SYS_SLEEP` | `ms` | `0` |
| 6 | `SYS_ENDPOINT_CREATE` | — | endpoint id；满表 `ENOMEM` |
| 7 | `SYS_ENDPOINT_DESTROY` | `id` | 成功 `0`；`EINVAL`/`ENOENT`/`EPERM` |
| 8 | `SYS_ENDPOINT_CALL` | `id`, `msg`（MVP：低 **32** 位） | 成功为 `reply` 的 **32** 位值；无对手就绪 **`EAGAIN`（-11）** |
| 9 | `SYS_ENDPOINT_RECV` | `id` | 成功为 `(client_id << 32) \| request`（各 **32** 位）；无消息 **`EAGAIN`（-11）** |
| 10 | `SYS_ENDPOINT_REPLY` | `id`, `client_id`, `msg`（MVP：低 **32** 位） | 成功 `0` |

冻结前仍可按 [TODO.md](./TODO.md) 与 issue 计划调整编号与参数。

---

## 3. 与用户态存根

- 用户态仅通过 **`syscall_abi` crate** 或生成代码发起调用，**不链接**内核私有符号。  
- 与 [ARCHITECTURE.md §8](./ARCHITECTURE.md) 依赖约束一致。

---

## 相关文档

- [重要 Bug 修复说明](./BUGFIX_NOTES.md)
- [微内核设计](./KERNEL.md)
- [架构说明](./ARCHITECTURE.md)
- [移植指南](./PORTING.md)
