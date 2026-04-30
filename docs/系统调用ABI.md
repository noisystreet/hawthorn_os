# 系统调用 ABI（DRAFT-1.0 · M5 收束）

> **[English](./en/SYSCALL_ABI.md)** — English mirror.

用户态与 **山楂（hawthorn）微内核** 通过 **稳定 syscall 界面** 交互。本文档与 crate **`hawthorn_syscall_abi`**（`syscall_abi/`）及 **`kernel/src/syscall.rs`** **对齐**，标识为 **DRAFT-1.0**；数字版本见 **`hawthorn_syscall_abi::ABI_VERSION`**（当前为 **1**）。**尚未**宣称长期稳定兼容；升级为 **STABLE-x** 时将在本文与 Release note 中声明。

---

## 1. 编号空间与演进命名

| 标签 | 含义 |
|------|------|
| **DRAFT-x.y** | 与代码一致的当前稿；允许在下一发行前调整 syscall（会同步 bump `ABI_VERSION` 并改本文）。 |
| **STABLE-x** | （未来）承诺在所声明主版本内 **不破坏性** 变更编号与参数布局；仅可增加新号或新开 capability 位。 |

**系统调用号段**（与 `hawthorn_syscall_abi` 常量一致）：

| 范围 | 用途 |
|------|------|
| `0..=SYSCALL_NR_CORE_MAX`（63） | **内核核心固定表** — `hawthorn_kernel::syscall` 按号分派 |
| `64..=255` | **预留** — 未来固定分配 |
| `≥256` | **预留** — 动态或离板分配策略 **TBD** |

表中 **未实现** 的核心槽位与 **越界** 号均返回 **`ENOSYS`（`x0 = -38`）**。

---

## 2. AArch64 陷入与寄存器约定（已定方向）

| 项目 | 约定 |
|------|------|
| 陷入 | **`SVC #0`**（EL0 / EL1 均可触发；**立即数不参与** 分派） |
| 系统调用号 | **`x8`** |
| 参数 | **`x0`–`x5`**，至多 **6** 个标量参数（`hawthorn_syscall_abi::SYSCALL_MAX_ARGS == 6`） |
| 返回值 | **`x0`**：成功为非负/`0`；错误为 **负 errno**（Linux aarch64 风格，见 `Errno::as_u64`） |
| 易失寄存器 | 与 AAPCS64 / Linux syscall 习惯一致：**`x0`–`x17`** 可被内核破坏；若用户态需跨 `SVC` 保留，调用方自行保存。 |

**与通用 PCS**：用户态把 `SVC` 看作 **C ABI 边界**；内核返回后仅保证 **`x0`** 与保留寄存器 **`x19`–`x28`**、**`x29`（FP）**、**`x30`（LR）** 符合被调用方约定（具体以实现与编译器为准；MVP 以单线程 EL0 任务为主）。

---

## 3. `SYS_ABI_INFO` 与能力位

| 编号 | 常量 | 参数 | `x0` 返回 |
|------|------|------|-----------|
| **11** | **`SYS_ABI_INFO`** | 忽略 | **非负**：低 **32** 位 = `ABI_VERSION`；高 **32** 位 = `ABI_CAP_*` 按位或（**非 errno**） |

当前能力位（`hawthorn_syscall_abi`）：

- **`ABI_CAP_EL0_USER_AS`**：内核为任务维护 **EL0 用户地址空间**（固定低 VA 窗口 + 用户指针校验），与 `user_layout` / `mm` 一致。

---

## 4. 系统调用表（核心 `0..=11`）

语义以 **`kernel/src/syscall.rs`** 为准。

| 编号 | 常量 | 参数（`x0`–`x5`） | `x0` 返回 |
|------|------|-------------------|-----------|
| 0 | `SYS_WRITE` | `fd`, `buf`, `len` | 写入字节数；错误为负 errno（`fd` 目前仅 **1**；用户指针经 `user_layout` 校验） |
| 1 | `SYS_READ` | （预留） | **`ENOSYS`** — 显式桩，待实现 |
| 2 | `SYS_YIELD` | — | `0` |
| 3 | `SYS_GETPID` | — | 当前任务 id |
| 4 | `SYS_EXIT` | `code` | 不返回 |
| 5 | `SYS_SLEEP` | `ms` | `0` |
| 6 | `SYS_ENDPOINT_CREATE` | — | endpoint id；满表 `ENOMEM` |
| 7 | `SYS_ENDPOINT_DESTROY` | `id` | 成功 `0`；`EINVAL`/`ENOENT`/`EPERM` |
| 8 | `SYS_ENDPOINT_CALL` | `id`, `msg`（MVP：低 **32** 位） | 成功为 `reply` 的 **32** 位有符号扩展值；无对手就绪 **`EAGAIN`（-11）** |
| 9 | `SYS_ENDPOINT_RECV` | `id` | 成功为 `(client_id << 32) \| request`（各 **32** 位）；无消息 **`EAGAIN`（-11）** |
| 10 | `SYS_ENDPOINT_REPLY` | `id`, `client_id`, `msg`（MVP：低 **32** 位） | 成功 `0` |
| 11 | `SYS_ABI_INFO` | 忽略 | 见 §3 |

`12..=63`：**未分配**，当前分派返回 **`ENOSYS`**。

---

## 5. 错误码与返回值

- **`hawthorn_syscall_abi::Errno`**：用户态/内核共享；`as_u64()` 将错误编码为 **`x0 = -errno`**（`errno` 为正整数）。
- 「返回值解析」辅助：`is_error`、`errno_from_ret`（仅覆盖已列出的 errno）。
- 细节与历史修复说明：[缺陷修复笔记.md](./缺陷修复笔记.md) 第 1 节。

---

## 6. 用户态接入

- 仅通过 **`hawthorn_syscall_abi` crate**（或由此生成的封装）发起调用；**禁止**链接内核私有符号。
- 与 [架构.md §8](./架构.md) 依赖约束一致。

---

## 相关文档

- [缺陷修复笔记.md](./缺陷修复笔记.md)
- [内核.md](./内核.md)
- [架构.md](./架构.md)
- [移植.md](./移植.md)
- [陷入与异常.md](./陷入与异常.md)（`SVC` / `x8` 硬件路径）
