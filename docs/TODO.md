# 山楂 / hawthorn — 新功能与能力扩展（TODO）

> **[English](./en/TODO.md)** — English mirror of this document.

本页列出计划在仓库内实现的 **新功能与运行时能力**；设计背景与约束见 [ARCHITECTURE.md](./ARCHITECTURE.md)、[KERNEL.md](./KERNEL.md)、[PORTING.md](./PORTING.md)。子项尽量 **可独立开 issue**；用 `- [ ]` / `- [x]` 在 PR 中跟踪。

---

## 当前已实现模块概览（M1–M3）

| 模块 | 文件 | 功能 |
|------|------|------|
| 引导 | `boot_qemu_virt.rs` / `bin/qemu_virt.rs` | EL2→EL1 降级 + MMU 禁用 + BSS 清零 + PL011 初始化 |
| 控制台 | `console.rs` | `print!` / `println!` 宏，基于 PL011 UART |
| 异常/向量 | `trap.rs` | `VBAR_EL1` 16 槽向量表，`TrapFrame`，`handle_exception` 分发 |
| GICv3 | `gic.rs` | Distributor / Redistributor / CPU Interface 初始化，`ack()` / `eoi()` |
| IRQ 分发 | `irq.rs` | 1020 槽 handler 表，`register()` / `dispatch()` |
| 定时器 | `timer.rs` | ARM Generic Timer (PPI 30)，周期 tick，频率自 `CNTFRQ_EL0` |
| 任务调度 | `task.rs` | 协作式调度 MVP：TCB / `create()` / `yield_now()` / `context_switch` 汇编 / `task_exit` |

**启动序列**：`_start` (EL2→EL1) → `kernel_main`：BSS → UART → `trap::init()` → `gic::init()` → `irq::init()` → `timer::init()` → `task::init()` → 使能 IRQ → idle `yield_now()` 循环

**QEMU 验证**：任务 A/B 交替执行 + Timer tick 每 10ms + 任务退出后 idle 继续运行

---

## 内核基础与对象模型

### 模块边界与依赖图

- [ ] 在 `kernel/` 内划分 crate 内模块（建议名：`ids`、`caps`、`task`、`wait` 等），并在 `KERNEL.md` 画 **单向依赖**（禁止 `kernel` → `servers`）。
- [ ] 约定 **公开 API 面**（`lib.rs` re-export）与 **内部模块** 的可见性，避免过早稳定错误类型。
- [ ] 为「将来 `hal` / `bsp` 只被 arch 或 board 封装调用」预留 `cfg(target_arch)` / `feature` 钩子（可先无实现）。

### Capability 与地址空间句柄

- [ ] **Capability**：不可伪造句柄（索引 + 代际 / 版本号）、右位图（send、grant、read、write 等）的最小集合。
- [ ] **端点 / 端口 ID**：内核全局唯一或与地址空间绑定的分配与回收策略。
- [ ] **根 CSpace 或命名空间表**：boot 时首个用户任务的能力集初始化规则。
- [ ] **能力撤销**：`revoke` / 任务退出时级联失效的规则与数据结构。

### 测试与不变量

- [ ] 对 **ID 池、位图、代际检查** 编写 `#[cfg(test)]` 用例：空、满、重用、越界、双释放。
- [ ] 文档化 **不变量**（例如：cap 索引永不超过池大小；代际单调递增）与违反时的 `panic` 策略。

### 启动、Panic 与异常向量

- [x] **引导链**：`_start`（汇编）→ 设置栈 / BSS → 跳转到 Rust `kernel_main`（或等价）；与 `link-qemu_virt.ld` / 未来板级脚本对齐。
- [x] **`#[panic_handler]`**：格式化或最小化 panic 信息输出路径（UART 或内存环）。
- [x] **向量表**：`VBAR_ELx` 设置；`sync` / `irq` / `fiq` / `SError` 入口汇编桩；默认死循环或转发到 Rust `handle_exception(reason)`。
- [x] **EL 选择**：文档 + 代码一致（如长期 EL1 或先 EL2 再降级）；与 [BOOT.md](./BOOT.md) 交叉引用。

---

## 调度与执行

### 协作式调度（优先实现路径）

- [x] **线程 / 任务控制块（TCB）**：状态（就绪、运行、阻塞、退出）、优先级字段、内核栈指针。
- [x] **就绪队列**：同优先级 FIFO 或多队列；`schedule()` / `yield()` 入口。
- [ ] **自愿阻塞原语**：与 IPC 或 `wait_timeout` 对接的最小阻塞队列。

### 抢占与时间

- [ ] **可抢占标志**：临界区屏蔽（关抢占或关中断粒度文档）。
- [ ] **时间片**：固定量子或可配置；与定时器中断挂钩。
- [ ] **Tickless**：空闲时无周期 tick；唤醒依赖定时器单次触发（可先文档后实现）。

### SMP（多核）

- [ ] **主 CPU**：完成 GIC、定时器、全局数据结构初始化后再释放 AP。
- [ ] **AP 入口**：`spin-table` 或 PSCI 路径择一，与 [PORTING.md](./PORTING.md) 真机假设对齐。
- [ ] **IPI**：自旋锁、核间 TLB shootdown（后期）、调度器迁移（后期）。
- [ ] **每核 idle**：`wfi` / 低功耗占位与 **负载均衡** 策略（后期）。

### 睡眠与定时

- [ ] **相对超时**：插入按到期时间排序的内核定时器队列（单链表或小根堆）。
- [ ] **绝对时钟**：单调时钟 vs 墙钟策略（文档）；读 **CNTVCT_EL0** 或板级定时器。
- [ ] **与 IPC 结合**：`recv` 带超时、`sleep` 系统调用草案。

---

## IPC 与消息

### 同步短消息（MVP）

- [ ] **发送 / 阻塞接收**：固定小负载（如 ≤128B）拷贝语义；与调度器阻塞队列联动。
- [ ] **调用 / 应答（call）**：客户端阻塞直至服务端 `reply`；匹配 `request_id`。
- [ ] **超时与取消**：发送/接收超时返回码；超时与能力撤销交互说明。

### 端口与队列

- [ ] **端口对象**：持有等待接收者队列；与 capability 绑定。
- [ ] **队列深度上限**与 **反压**：满时 `send` 返回 `EAGAIN` 或阻塞策略二选一并文档化。

### 大块与流式

- [ ] **Grant / map**：将发送方物理页临时映射到接收方地址空间的能力与安全检查。
- [ ] **环形缓冲**：单生产者单消费者起步；多生产者时的锁或无锁策略（后期）。

### 能力随消息传递

- [ ] **随 `send` 转移 capability**（移动语义）与 **复制**（需 `Grant` 权）的区分。
- [ ] **内核校验**：目标地址空间是否有权接收该 cap 类型。

---

## 系统调用与用户态边界

### ABI 版本与编号

- [ ] 在 [SYSCALL_ABI.md](./SYSCALL_ABI.md) 定义 **编号空间**、**DRAFT-x.y** 与 **STABLE-x** 命名规则。
- [ ] **寄存器约定**：`x0–x7` 参数、`x8` syscall 号、`ret`/`errno` 模型；与 AArch64 PCS 差异说明。
- [ ] **版本探测 syscall**：返回 `ABI_VERSION` 与能力位掩码（可先全 0）。

### `syscall_abi` crate

- [ ] 常量：`SYS_*` 编号、`MAX_ARGS`、错误码枚举与 `From<u64>`。
- [ ] **用户侧封装**（可选子 crate）：内联 asm 或 `compiler_builtins` 兼容的封装函数（与 `no_std` 用户态对齐）。

### 陷入与返回

- [ ] **SVC 入口**：内核统一分发；非法号返回 `ENOSYS`。
- [ ] **用户栈与 TLS**：`TPIDR_EL0` 或等价；线程创建时初始化。
- [ ] **Trampoline**：内核返回用户态时恢复 **PSTATE / SP_EL0 / ELR_EL1** 的路径。

### Fault 与线程生命周期

- [ ] **Data / instruction abort**：区分用户 fault 与内核 bug；用户 fault → 发信号或杀线程（策略文档）。
- [ ] **非法 syscall 参数**：范围检查、指针用户态校验（后期 `copy_from_user` 风格）。

---

## 内存与地址空间

### 物理内存

- [ ] **内存探测**：从 FDT / 固定表 / boot info 块获取 RAM 区间（QEMU 与 RK3588 两条路径）。
- [ ] **帧分配器**：bump allocator 起步 → 伙伴系统；与锁（粗粒度自旋锁 → per-CPU 后期）。
- [ ] **物理内存热插拔**：占位否（文档一句即可）。

### 内核虚拟地址

- [ ] **恒等映射或固定偏移**：与链接脚本一致；设备区间映射为 **Device-nGnRnE**（或等价）。
- [ ] **内核堆**（可选）：`kmalloc` 风格或 slab 后期；初期可用静态池。

### 用户地址空间

- [ ] **地址空间对象**：页表根、asid（若用）、引用计数。
- [ ] **map / unmap API**：与 capability 绑定；`PROT_READ` / `PROT_WRITE` / `PROT_EXEC` 组合策略（W^X 默认策略文档）。
- [ ] **用户栈映射**：缺页或预映射；guard page（后期）。

---

## QEMU 与最小可运行镜像

### 与 `hawthorn_kernel` 联动

- [x] `qemu_minimal` 通过 **`hawthorn_kernel::...` 公开 API** 打印第二行或注册 noop 任务；`Cargo.toml` feature 边界清晰。
- [ ] **可选**：拆出 `kernel_tests` 或 `examples/` 下第二个 bin，仅用于集成验证。

### 中断与时间基

- [x] **GICv3**（`virt` 默认）或 GICv2：使能 PPI **generic physical timer**；异常路由到当前实现。
- [x] **最小 IRQ handler**：计数或喂狗占位；与调度器「时间片抢占」预留钩子。

### 设备树（FDT）

- [ ] **解析**：`/memory` reg、`/chosen` `stdout-path`、bootargs（可先打印不执行）。
- [ ] **与 PORTING 对照**：记录 `virt` 与 OPi5 DTB 字段差异表（文档）。

---

## 平台与 BSP（RK3588 / 香橙派 5）

### 目录与链接

- [ ] `bsp/orangepi5-rk3588/`：`README`、**`link.ld`** / **`memory.x`**、与 `PORTING` §3 一致的 **RAM / 设备寄存器** 占位。
- [ ] **入口地址**：与 U-Boot / TF-A 移交假设一致（`ASSUME-*` 编号写进 `BOOT.md`）。

### 早期硬件 bring-up

- [ ] **UART**：PL011 或板载调试串（以 TRM / 板厂为准）；与 `qemu_minimal` 打印路径复用驱动代码（`hal` 抽象）。
- [ ] **GIC**：SPI 号与设备树一致；非 `virt` 中断号文档表。
- [ ] **Arch timer**：`CNTFRQ` 读取与 tick 换算。

### 时钟、复位、电源

- [ ] **CRU / PMU** 最小寄存器表：UART 时钟使能、总线复位（可从主线 Linux DT 反查偏移，注明许可证注意）。
- [ ] **DVFS / 温控**：占位接口；读温度传感器为后期项。

---

## 用户态服务与驱动形态

### 根服务与命名

- [ ] `servers/` 布局：`init`、可选 `name_server`；**启动顺序**（谁拉谁）写进 `KERNEL.md` 或 `BOOT.md`。
- [ ] **从内核到用户态第一条消息**：例如把 cap 交给 `init` 的硬编码路径（MVP）。

### 用户态驱动样例

- [ ] **PL011 服务**：能力 `DeviceMmio` + 中断 cap（后期）；轮询版先行。
- [ ] **virtio-console**（QEMU）：与块设备 **virtio-mmio** 寄存器布局探测（`virt` 固定偏移文档化）。

### 中间件

- [ ] `middleware/`：**机器人控制消息** schema 占位（IDL 或 Rust 类型）；与 `servers/` 的依赖方向说明。

---

## 机器人与产品化

### 实时与确定性

- [ ] 在 `ARCHITECTURE` 中定义 **RT0–RT3**（示例名）等级：抖动上限、截止期错过策略。
- [ ] **关键路径标注**：哪些 syscall 路径禁止分配堆、哪些允许。

### OTA 与 A/B

- [ ] **启动信息块**字段：`slot`、`rollback`、`image_hash` 占位与校验桩。
- [ ] **与 U-Boot / 厂商工具链** 的协作点列表（只文档亦可）。

### 遥测、日志、调试

- [ ] **内核环形日志**：大小、覆盖策略、导出 syscall（草案）。
- [ ] **速率限制**：防止用户态打爆日志。
- [ ] **JTAG / semihosting**：默认关；`feature` 或编译开关打开时的安全警告。

---

## 后续大项（依赖上述基础）

### 网络与存储（用户态为主）

- [ ] **以太网 / Wi-Fi**：驱动服务 + 协议栈进程；内核仅队列与能力。
- [ ] **块设备**：virtio-blk 或 eMMC 服务；VFS 或裸块接口二选一文档。

### 虚拟化与 I/O 路径

- [ ] **virtio** 统一：`mmio` 探测、IRQ、feature bit 协商；与 `hal` 分层。
- [ ] **DMA 一致性**：缓存维护 ops 封装（ AArch64 `dc cvac` 等）与能力模型。

### 安全与信任根

- [ ] **M3 / 安全世界**交互：scm 调用占位、正常世界内核假设。
- [ ] **安全启动链**：镜像签名验证在 Boot 还是内核阶段（决策 + 占位实现路径）。

---

## 说明（非功能条目）

- 本列表 **不替代** issue 优先级排序；合并大项前请在 issue 标题中带章节名（如 `[IPC]`、`[BSP]`）便于筛选。
- **当前里程碑的 PR / issue 顺序**（含 GitHub 链接）：[PR_ISSUE_PLAN.md](./PR_ISSUE_PLAN.md)。
