# 系统调用 ABI（骨架）

> **[English](./en/SYSCALL_ABI.md)** — English mirror of this document.

用户态服务与 **山楂（hawthorn）微内核** 通过 **稳定 syscall 界面** 交互；实现细节在 `kernel/syscall/` 与未来 `syscall_abi` crate 中落地。本文跟踪 **冻结前** 的约定方向。

---

## 1. 调用约定（AArch64，待冻结）

| 项目 | 方向 |
|------|------|
| 陷入方式 | **`SVC`** 进入 EL1 内核（具体立即数或 x8 传 syscall 号：**TBD**） |
| 参数寄存器 | 遵循 AAPCS64 与 Linux aarch64 syscall 惯例的 **子集** 或 **自定义最小集**（二选一拍板） |
| 返回值 | `x0` = 结果或错误码；错误码枚举与 `errno` 是否对齐 POSIX 子集：**TBD** |
| 易失 / 保存寄存器 | 按 AAPCS64 被调用方保存约定文档化 |

---

## 2. 系统调用号空间（占位）

建议分区（示例，未分配编号）：

| 区间用途 | 说明 |
|----------|------|
| 线程与调度 | 退出、yield、优先级等 |
| IPC | send、recv、reply、endpoint 管理等 |
| 内存与能力 | 映射、撤销、能力派生等 |
| 中断与通知 | IRQ 绑定、notification 信号等 |
| 调试 | 可选，发布版编译裁剪 |

正式编号表在首次 ABI 冻结时写入本节并生成 `syscall_abi` 常量。

---

## 3. 与用户态存根

- 用户态仅通过 **`syscall_abi` crate** 或生成代码发起调用，**不链接**内核私有符号。  
- 与 [ARCHITECTURE.md §8](./ARCHITECTURE.md) 依赖约束一致。

---

## 相关文档

- [微内核设计](./KERNEL.md)
- [架构说明](./ARCHITECTURE.md)
- [移植指南](./PORTING.md)
