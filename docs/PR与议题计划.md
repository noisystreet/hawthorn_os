# PR 与 Issue 编排（当前里程碑）

> **[English](./en/PR_ISSUE_PLAN.md)** — English mirror of this document.

本文档把 **GitHub Issue** 与建议的 **PR 顺序** 固定下来，便于开分支、写 PR 描述时引用 `Closes #…` / `Refs #…`。能力级 backlog 仍以 [待办.md](./待办.md) 为准。

---

## 当前跟踪：QEMU `virt` 上 `hawthorn_kernel` 最小可运行路径

| 角色 | 链接 |
|------|------|
| **Meta（总览）** | <https://github.com/noisystreet/hawthorn_os/issues/5> |

### Issue 列表（按建议实现顺序）

| 顺序 | Issue | 标题（摘要） | 状态 |
|------|--------|----------------|------|
| 1 | [#1](https://github.com/noisystreet/hawthorn_os/issues/1) | M1：`hawthorn_kernel` 最小引导（QEMU virt）+ PL011 panic | ✅ 完成 |
| 2 | [#2](https://github.com/noisystreet/hawthorn_os/issues/2) | M1b：`qemu_minimal` 经 `hawthorn_kernel` 公开 API 启动 | ✅ 完成 |
| 3 | [#3](https://github.com/noisystreet/hawthorn_os/issues/3) | M2：`VBAR_EL1` 异常向量 + GICv3 + IRQ 分发 | ✅ 完成 |
| 4 | [#4](https://github.com/noisystreet/hawthorn_os/issues/4) | M3：协作式调度 MVP（TCB / 就绪队列 / yield）| ✅ 完成 |

**建议 PR 顺序：** `#1 → #2 → #3 → #4`。其中 **#3（M2 向量表）** 在 M1 的入口与符号稳定后，可与 **#2（M1b）** 并行开发，合并时注意冲突（向量表 vs qemu 联动以先合并者为准，后者变基）。

---

## PR 开法约定

1. **一个 PR 尽量对应一个 issue**；大改可拆 PR，但每个 PR 仍应 `Closes #n` 或 `Refs #n`。
2. PR 描述使用仓库模板 [.github/pull_request_template.md](../.github/pull_request_template.md)，在 **相关 Issue** 填写例如：`Closes #1`。
3. **提交信息**：`docs/提交约定.md` — 第 1 行英文 Conventional Commits，第 2 行中文对应。
4. **标签**：内核相关 issue 已使用 `kernel` + `enhancement`；新 issue 标题建议继续带 **`[kernel]`**、`[IPC]` 等前缀（与 [待办.md](./待办.md) 说明一致）。

---

## 本地验证（与 CI / AGENTS 对齐）

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo check -p hawthorn_kernel
cargo check -p hawthorn_kernel --target aarch64-unknown-none
cargo build -p hawthorn_qemu_minimal --features bare-metal --target aarch64-unknown-none
```

M1 合并后若引入新 crate feature 或新 bin，请在本文件与 **相关 issue 验收标准** 中同步更新命令。

---

## 后续（尚未建 issue）

下一批可在 TODO 中勾选并 **另开 issue** 的条目示例：`syscall_abi` crate、SVC 统一分发、最小 IPC（短消息）。Meta issue **#5** 关闭后，可新建 `[meta]` issue 跟踪下一阶段。

---

## 第二阶段：抢占调度 + 用户态边界 + IPC

M1–M3 已全部完成。以下为建议的第二阶段里程碑与对应 Issue：

| 顺序 | 建议 Issue | 标题（摘要） | 依赖 |
|------|------------|----------------|------|
| 5 | **#6**（建议） | M4：抢占式调度 + 阻塞原语（时间片、sleep、等待队列） | Timer + TCB 已就绪 |
| 6 | **#7**（建议） | M5：SVC 入口 + syscall 分发 + `syscall_abi` crate | M2 异常向量已有 EL0 Sync 槽 |
| 7 | **#8**（建议） | M6：用户地址空间（EL0 页表、用户栈、`eret` 返回用户态） | syscall ABI |
| 8 | **#9**（建议） | M7：同步短消息 IPC（call/reply、rendezvous、≤128B 拷贝） | 用户地址空间 + TCB 阻塞 |

### M4 详细设计要点

**目标**：从协作式调度升级为 **固定优先级抢占式调度（FP）**，同优先级轮转；添加阻塞原语。

- **时间片（time slice）**：Timer 中断递减当前任务的 `time_slice` 计数器；耗尽时置 `need_reschedule` 标志，在中断返回前调用 `schedule()`。
- **抢占调度**：`schedule()` 从就绪队列选择最高优先级任务；若当前任务优先级不高于下一任务则强制切换。
- **阻塞原语**：
  - `task::sleep(ms)` — 插入定时器等待队列，到期唤醒回就绪队列
  - `task::block()` / `task::unblock(id)` — 手动阻塞/唤醒，供 IPC 等后续模块使用
- **等待队列**：按到期时间排序的单链表（或简化为固定数组扫描），Timer tick 中检查并唤醒到期任务。
- **Tickless idle**（可选）：无就绪任务时停掉周期 tick，设下一次唤醒的单次定时器（`CNTP_TVAL_EL0`）。

### M5 详细设计要点

**目标**：建立内核 ↔ 用户态的稳定 syscall ABI。

- **SVC 入口**：`ESR.EC = 0x15` 分支解析 `x8` = syscall 号，`x0–x7` = 参数，分发到处理函数表
- **`syscall_abi` crate**（独立 crate，内核与用户态共享）：
  - `SYS_*` 编号常量、`MAX_ARGS`、错误码枚举
  - 版本探测 syscall（返回 `ABI_VERSION` + 能力位掩码）
- **寄存器约定**：`x8`=syscall 号、`x0–x5`=参数、`x0`=返回值 + 错误码
- **非法 syscall**：返回 `ENOSYS`

### M6 详细设计要点

**目标**：支持 EL0 用户态任务运行，内核管理独立地址空间。

- **地址空间对象**：页表根（TTBR0_EL1）、ASID、引用计数
- **map / unmap API**：与 capability 绑定；`PROT_READ` / `PROT_WRITE` / `PROT_EXEC`，默认 W^X
- **用户栈映射**：预映射 + guard page
- **上下文切换**：切换 TTBR0_EL1 + ASID + TLB 刷新
- **用户态进入/返回**：设置 `SP_EL0`、`ELR_EL1`、`SPSR_EL1`（EL0 AArch64），`eret`

### M7 详细设计要点

**目标**：实现微内核的核心通信机制——同步短消息 IPC。

- **call / reply**：客户端 `call(endpoint, msg)` 阻塞直至服务端 `reply`；rendezvous 语义
- **消息负载**：≤128B 内联拷贝（寄存器传递优化可后期添加）
- **端点对象**：持有等待接收者队列；与 capability 绑定
- **阻塞与调度协同**：`recv` 无消息时阻塞 → `block()`；`send` 目标阻塞时 → `unblock()`
- **能力传递**：随消息转移或复制 capability 槽位（MVP 可先仅支持移动语义）

### 建议的 PR 顺序

`#6 (M4) → #7 (M5) → #8 (M6) → #9 (M7)`

其中 M5 和 M6 可部分并行（SVC 分发和地址空间设计可独立推进），但 M6 依赖 M5 的 syscall ABI 定义，M7 依赖 M6 的用户态上下文切换。

### 远期方向（第三阶段）

- 物理内存帧分配器 → 伙伴系统
- Capability 系统（不可伪造句柄、代际、权限派生、撤销级联）
- Notification / IrqControl（中断完成与线程唤醒桥梁）
- SMP 多核支持（AP 启动、per-CPU 数据、IPI、负载均衡）
- RK3588 / 香橙派 5 真机 BSP
- 用户态驱动服务、根服务 init

---

## 状态更新（2026-04-29）

- M4（抢占调度）✅：时间片、`sleep/block/unblock`、IRQ 驱动重调度已接通。
- M5（SVC + syscall）✅：`syscall_abi` + 分发表 + `SYS_write/getpid/exit/sleep/yield` 可用。
- M6（EL0 用户态）✅：已完成最小可运行闭环，QEMU 串口可见 `hello from EL0!`。
- CI 已加入两条串口回归：
  - `scripts/verify_kernel_qemu_virt_serial.sh`
  - `scripts/verify_kernel_qemu_virt_el0_serial.sh`
- M7-W2（IPC MVP）🔄：`endpoint` + `SYS_ENDPOINT_{CREATE,DESTROY,CALL,RECV,REPLY}`；当前为 **≤32 位** 载荷 MVP，`call/recv` 在无对手就绪时返回 **`EAGAIN`（-11）** 由调用方轮询（`verify_kernel_qemu_virt_el0_serial.sh` 覆盖 EL1 `task E/D` 串口回环）。真正 **≤128B** 与 **双 EL0** 任务回环仍为后续。

> 注：当前 M6 属于“可运行 MVP”，尚有隔离强化与生命周期完善空间（见下文 M7 拆分）。

---

## M7 两周执行清单（建议）

### Week 1（稳态与正确性）

| 子项 | 目标 | 验收标准 | 建议 Issue |
|------|------|----------|------------|
| M7-W1-1 用户态上下文完整保存/恢复 | 完整化 EL0 trap 上下文策略（含 GPR 语义） | 多次 syscall + 抢占后用户任务持续正确运行 | `#9-1` |
| M7-W1-2 任务退出与资源回收 | `sys_exit` 后回收用户页、页表、TCB 资源 | 反复创建/退出用户任务无明显资源泄漏 | `#9-2` |
| M7-W1-3 用户指针校验基线 | 为 `sys_write` 增加最小 `copy_from_user` 路径 | 非法用户地址返回错误码，不触发内核异常 | `#9-3` |
| M7-W1-4 回归增强 | CI 增加 EL0 syscall/exit 回归用例 | CI 稳定通过，失败时日志可定位 | `#9-4` |

### Week 2（IPC MVP 落地）

| 子项 | 目标 | 验收标准 | 建议 Issue |
|------|------|----------|------------|
| M7-W2-1 端点对象与 capability 绑定 | 引入 endpoint + 基本权限检查 | 创建/销毁端点、权限拒绝路径可测 | `#9-5` |
| M7-W2-2 同步短消息 call/reply | 实现 rendezvous（≤128B） | 双用户任务完成 request/reply 回环 | `#9-6` |
| M7-W2-3 阻塞队列与调度协同 | `recv` 阻塞、`reply` 唤醒 | 队列顺序与唤醒语义稳定 | `#9-7` |
| M7-W2-4 文档与 ABI 收口 | 更新 `SYSCALL_ABI`/`KERNEL`/`TODO` | 文档和实现对齐，无陈旧接口说明 | `#9-8` |

### M7 完成定义（Definition of Done）

1. 至少 2 个 EL0 任务可通过 syscall + IPC 完成端到端交互。
2. 用户任务可创建/退出并回收资源，连续回归无泄漏迹象。
3. CI 同时覆盖：
   - 内核 banner 回归
   - EL0 输出回归
   - EL0 syscall + IPC smoke 回归

---

## 六个月发展规划（建议草案）

### 目标边界

- 平台主线：先稳定 QEMU `virt`（AArch64），再推进 RK3588 真机迁移。
- 架构主线：微内核；驱动/服务尽量用户态化。
- 交付标准：每月都有“可演示 + 可回归 + 可量化”的里程碑。

### M+1（第 1 个月）：EL0 边界收口与稳态化

- **重点**：把“能跑”提升到“稳定跑”。
- **交付**：
  - EL0 上下文保存/恢复语义收口（含多任务切换场景）
  - 用户任务 `create/exit` 资源回收（页表/页框/TCB）
  - `copy_from_user`/`copy_to_user` 最小安全路径
  - 回归：EL0 syscall/exit 压测脚本
- **验收**：
  - 100+ 次用户任务创建退出无崩溃
  - CI 稳定通过（基础串口 + EL0 串口）

### M+2（第 2 个月）：IPC MVP 与服务框架

- **重点**：同步短消息通信落地。
- **交付**：
  - endpoint 对象 + capability 基本权限校验
  - `call/reply`（≤128B）rendezvous MVP
  - 阻塞/唤醒与调度器联动稳定
  - 用户态服务骨架（init/name/driver 模板）
- **验收**：
  - 两个 EL0 进程完成 request/reply 回环
  - 1000 次消息往返无死锁

### M+3（第 3 个月）：外设驱动最小可用

- **重点**：从内核直驱转向用户态驱动服务。
- **交付**：
  - 至少一个用户态驱动服务（串口服务化或 timer 事件服务）
  - IRQ 到用户态服务的最小桥接路径
  - 基本设备能力模型（最小 rights）
- **验收**：
  - 驱动服务可稳定收发事件/数据
  - 服务异常退出后可重启并恢复

### M+4（第 4 个月）：块设备通路

- **重点**：为文件系统做前置。
- **交付**：
  - QEMU `virtio-blk`（或等价块设备）MVP
  - 块读写 API + 简单缓存层
  - I/O 错误与超时处理路径
- **验收**：
  - 稳定读写块设备镜像
  - 回归覆盖读写一致性

### M+5（第 5 个月）：文件系统 MVP

- **重点**：文件抽象可用。
- **交付**：
  - VFS 最小抽象（inode/file/ops）
  - 首个 FS（建议 RAMFS 起步，后接简单磁盘 FS）
  - `open/read/write/close` 最小 syscall 语义
- **验收**：
  - EL0 程序可读写文件
  - 基本并发读写路径不崩溃

### M+6（第 6 个月）：工程化与上板准备

- **重点**：从功能走向产品工程。
- **交付**：
  - 性能/稳定性基线（启动时延、IPC 延迟、I/O 吞吐）
  - 故障注入与恢复策略（服务重启、超时、重试）
  - RK3588 bring-up 阻塞项与最小验证路径
  - 文档冻结：ABI、IPC、驱动模型、FS 约束
- **验收**：
  - 一键回归稳定
  - 上板前阻塞项清单清晰可执行

### 横向保障（贯穿六个月）

- 每月固定三项：
  1. 回归增强（新增能力必须带脚本与 CI）
  2. 可观测性（日志、计数器、故障码）
  3. 文档同步（中英与实现同频更新）
