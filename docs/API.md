# 对外 API 索引

> **[English](./en/API.md)** — English mirror of this document.

**山楂（hawthorn）** 尚未发布稳定 crate API。以下内容随代码落地逐步填充。

| 区域 | 说明 | 状态 |
|------|------|------|
| 系统调用 ABI | 寄存器约定、调用号、错误码 | 见 [SYSCALL_ABI.md](./SYSCALL_ABI.md) |
| `syscall_abi` crate | 用户态存根与常量 | 目录规划中，见 [ARCHITECTURE.md §8](./ARCHITECTURE.md) |
| 内核内部 crate | `kernel` 内模块 API | 非稳定，仅内核构建使用 |
