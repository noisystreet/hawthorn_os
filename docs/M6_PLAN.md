# M6 实施计划：用户地址空间与 EL0 用户态程序

> 本文档为 M6 里程碑的详细实施计划，目标是在 QEMU `virt` 上运行第一个 EL0 用户态程序。

## 1. 目标

- 启用 AArch64 MMU（当前内核以 MMU-off 模式运行）
- 实现 bump 帧分配器，为页表和用户内存提供物理帧
- 实现 AArch64 4 级页表管理（创建 / 映射 / 切换）
- 支持创建 EL0 用户态任务（`create_user`）
- 上下文切换支持 EL0（`eret` 到用户态 / SVC 返回内核态）
- 内嵌用户程序，QEMU 验证 EL0 执行 + `svc #0` 系统调用

## 2. 当前状态

| 模块 | 状态 |
|------|------|
| MMU | **关闭**（`_start` 中显式禁用 `SCTLR_EL1.M = 0`） |
| 内存访问 | 纯物理地址，无页表 |
| 内核加载地址 | `0x4000_0000`（QEMU `virt` RAM 起始，128 MiB） |
| 设备 MMIO | PL011 @ `0x900_0000`，GICD @ `0x800_0000`，GICR @ `0x80A_0000` |
| 调度 | 抢占式 FP 调度器，支持 sleep/block/yield |
| Syscall | SVC 分发已就绪（EL1 中 `svc #0` 已验证） |
| Trap | EL0 Sync 向量槽已存在，ESR.EC=0x15 分支已处理 |

## 3. 总体方案：最简路径

**核心决策**：MVP 阶段采用 **共享恒等映射页表**，不拆分 TTBR0/TTBR1。

理由：
- 当前内核链接在 `0x4000_0000`（低地址），属于 TTBR0_EL1 地址空间
- 拆分 TTBR0/TTBR1 需要重定位内核到高地址（`0xFFFF_0000_0000_0000+`），改动巨大
- 共享页表方案下：内核和用户代码都在同一恒等映射页表中，用户代码映射在低地址
- 用户态 **没有内存隔离**（可访问内核），但 EL0 执行 + SVC 通道完整可用
- 后续可迭代添加 TTBR0/TTBR1 分离实现真正隔离

### 执行流程

```
_start (EL2 → EL1, MMU off)
  → kernel_main
    → frame_alloc::init()     # 标记内核占用帧
    → mm::init()              # 创建恒等映射页表
    → mm::enable_mmu()        # 启用 MMU
    → trap::init()            # VBAR_EL1（已有）
    → gic::init()             # GICv3（已有）
    → irq::init()             # IRQ 分发（已有）
    → timer::init()           # Generic Timer（已有）
    → task::init()            # 调度器（已有）
    → syscall::init()         # Syscall 分发（已有）
    → 创建内核态 demo 任务
    → 创建 EL0 用户态任务
    → daifclr #2 (使能 IRQ)
    → idle yield 循环

用户任务执行：
  EL0: 用户代码运行
    → svc #0                # 系统调用
  EL1: EL0 Sync 向量入口
    → 保存 TrapFrame
    → handle_exception
      → syscall::dispatch   # 处理 syscall
    → 恢复 TrapFrame
    → eret                  # 返回 EL0
```

## 4. 模块设计

### 4.1 帧分配器 `frame_alloc.rs`

**接口**：
```rust
pub fn init(kernel_end_paddr: usize);  // 初始化，标记 [0, kernel_end) 为已用
pub fn alloc_frame() -> Option<usize>; // 分配一个 4K 物理帧，返回物理地址
pub fn free_frame(paddr: usize);       // 释放物理帧（MVP 可不实现）
```

**实现**：
- 物理内存范围：`0x4000_0000` – `0x4800_0000`（128 MiB = 32768 帧）
- 位图：`static mut BITMAP: [u64; 512]`（每 bit 一帧，512 × 64 = 32768）
- 分配策略：bump（线性扫描），找到第一个 0 bit，置 1，返回对应物理地址
- 初始化时将内核镜像占用的帧标记为已用

**内核结束地址**：由链接脚本导出 `__kernel_end` 符号，向上对齐到 4K 边界。

### 4.2 页表管理 `mm.rs`

**AArch64 4 级页表结构**（4K 页，48 位虚拟地址）：

| 级别 | 名称 | 索引位 | 表大小 |
|------|------|--------|--------|
| 0 | PGD | [47:39] | 512 × 8B = 4K |
| 1 | PUD | [38:30] | 512 × 8B = 4K |
| 2 | PMD | [29:21] | 512 × 8B = 4K |
| 3 | PTE | [20:12] | 512 × 8B = 4K |

**页表项格式**（Block/Page 描述符）：
- Bit 0: Valid
- Bit 1: Page/Table（Level 3 中 bit 1 = Page）
- Bits [47:12]: Next-level table address / Output address（物理页号）
- Bits [63:52]: AP (Access Permissions), UXN, PXN, AttrIndx 等

**接口**：
```rust
pub fn init();                                    // 创建内核恒等映射页表
pub fn enable_mmu();                              // 配置 TCR/MAIR/TTBR0，启用 MMU
pub fn map_page(vaddr: usize, paddr: usize, attr: u64); // 映射一个 4K 页
pub fn create_user_page_table() -> usize;         // 创建空的用户页表
pub fn map_user_page(table: usize, vaddr: usize, paddr: usize, attr: u64); // 用户页表映射
```

**内存属性**（MAIR_EL1）：
- Attr0 = `0xFF`：Normal, Write-Back, non-transient（内核代码/数据/栈）
- Attr1 = `0x04`：Device-nGnRE（UART、GIC 等 MMIO）

**恒等映射范围**：
- `0x4000_0000` – `0x4800_0000`：RAM（内核代码+数据+BSS+堆），Attr0
- `0x0800_0000` – `0x0810_0000`：GIC MMIO（GICD + GICR），Attr1
- `0x0900_0000` – `0x0900_1000`：PL011 UART，Attr1

**MMU 启用序列**：
```
1. 创建页表 + 恒等映射
2. MAIR_EL1 = 0x04FF (Attr0=0xFF, Attr1=0x04)
3. TCR_EL1: TG0=4K, T0SZ=16(48bit), IRGN0=WB, ORGN0=WB, SH0=Inner
4. TTBR0_EL1 = 页表基地址
5. ISB
6. SCTLR_EL1.M = 1 (启用 MMU)
7. ISB
```

**⚠️ 关键注意**：启用 MMU 前必须确保所有后续代码地址在页表中有映射，否则立即产生 Instruction Abort。由于使用恒等映射（VA=PA），这天然满足。

### 4.3 用户任务扩展 `task.rs`

**TCB 新增字段**：
```rust
struct Task {
    // ... 现有字段 ...
    is_user: bool,           // 是否为 EL0 用户态任务
    user_page_table: usize,  // TTBR0_EL1 值（用户页表物理地址）
    saved_elr: u64,          // 用户态返回地址（ELR_EL1）
    saved_spsr: u64,         // 用户态 PSTATE（SPSR_EL1, EL0 AArch64）
    saved_sp_el0: u64,       // 用户态栈指针
}
```

**新增接口**：
```rust
pub fn create_user(entry: usize, stack_top: usize) -> Option<TaskId>;
```

**创建流程**：
1. 从 TASK_TABLE 分配一个槽位
2. 分配内核栈（4K），设置 TaskContext（x19=entry, lr=user_trap_return）
3. `is_user = true`，设置 `saved_elr = entry`, `saved_spsr = 0x0000`（EL0t, AArch64, IRQ 未屏蔽）, `saved_sp_el0 = stack_top`
4. 分配用户页表，映射用户代码页 + 用户栈页（恒等映射）
5. 将任务设为 Ready

### 4.4 上下文切换改动

**当前 `context_switch` 汇编**：只保存/恢复 x19–x30 + SP

**新增 `user_trap_return` 汇编**：
```asm
user_trap_return:
    // 恢复用户态上下文
    ldr x0, [current_task + offset_saved_sp_el0]
    msr sp_el0, x0
    ldr x0, [current_task + offset_saved_elr]
    msr elr_el1, x0
    ldr x0, [current_task + offset_saved_spsr]
    msr spsr_el1, x0
    // 切换用户页表
    ldr x0, [current_task + offset_user_page_table]
    msr ttbr0_el1, x0
    isb
    tlbi vmalle1is
    isb
    // 恢复通用寄存器并返回用户态
    ... 恢复 x0-x30 ...
    eret
```

**Trap 返回路径**（IRQ / SVC 处理完成后）：
- 内核任务：恢复 TrapFrame + `eret`（回到 EL1 调用点）—— 已有
- 用户任务：恢复 TrapFrame（含用户 x0-x30）+ 恢复 SP_EL0/ELR/SPSR + 切换 TTBR0 + `eret`（回到 EL0）

### 4.5 内嵌用户程序

用户程序是一个极简的 AArch64 EL0 代码序列，功能：
1. 调用 `SYS_write(1, "hello from EL0!\n", 16)` 输出字符串
2. 调用 `SYS_yield()` 让出 CPU
3. 调用 `SYS_exit(0)` 退出

以机器码字节数组形式内嵌：

```rust
static USER_PROGRAM: &[u8] = &[
    // mov x0, #1          (fd=stdout)
    0x20, 0x00, 0x80, 0xD2,
    // ldr x1, =msg_addr   (用 adr 相对寻址)
    // ... 
    // mov x2, #16         (len)
    // mov x8, #0          (SYS_write)
    // svc #0
    0x01, 0x00, 0x00, 0xD4,
    // mov x8, #4          (SYS_exit)
    // mov x0, #0          (exit code)
    // svc #0
    0x01, 0x00, 0x00, 0xD4,
    // msg: "hello from EL0!\n"
    0x68, 0x65, 0x6c, 0x6c, 0x6f, ...
];
```

**备选方案**：用 `global_asm!` 定义用户程序，再用链接脚本导出符号获取其字节范围。这比手写机器码更可靠。

## 5. 文件变更清单

| 文件 | 操作 | 内容 |
|------|------|------|
| `kernel/src/frame_alloc.rs` | **新增** | Bump 帧分配器 |
| `kernel/src/mm.rs` | **新增** | 页表管理 + MMU 启用 |
| `kernel/src/task.rs` | **修改** | TCB 扩展 + `create_user()` + 用户态调度支持 |
| `kernel/src/trap.rs` | **修改** | 用户态 trap 返回路径（eret to EL0） |
| `kernel/src/boot_qemu_virt.rs` | **修改** | 初始化帧分配器 + MMU + 创建 EL0 用户任务 |
| `kernel/src/lib.rs` | **修改** | 添加 `frame_alloc` / `mm` 模块 |
| `kernel/link-qemu_virt.ld` | **修改** | 导出 `__kernel_end` 符号 |
| `kernel/src/bin/qemu_virt.rs` | **修改** | `_start` 中移除 MMU 禁用逻辑（改为 kernel_main 中启用） |

## 6. 风险与注意事项

1. **MMU 启用是最危险的操作**：VA=PA 恒等映射必须正确，否则立即 Instruction Abort。启用前需确保：
   - 页表自身所在帧已映射
   - 当前 PC 地址已映射
   - 栈地址已映射
   - UART/GIC MMIO 已映射（否则后续打印/中断会 fault）

2. **SCTLR_EL1 配置**：启用 MMU 的同时可能需要配置缓存（I-Cache / D-Cache）。QEMU 下不启用缓存也能工作，但启用缓存可提升性能。MVP 先不启用缓存，减少复杂性。

3. **TLB 刷新**：切换 TTBR0_EL1 后必须刷新 TLB（`tlbi vmalle1is`）。共享页表方案下用户任务切换不需要刷新（同一页表），但为未来 TTBR0/TTBR1 分离预留。

4. **`_start` 改动**：当前 `_start` 显式禁用 MMU。M6 需要移除该逻辑，改为在 `kernel_main` 中帧分配器和页表就绪后启用 MMU。`_start` 仍需处理 EL2→EL1 降级。

5. **内核结束地址**：需要链接脚本导出 `__kernel_end` 符号，帧分配器据此知道内核占用了哪些帧。当前链接脚本没有此符号，需要添加。

## 7. 实施顺序

```
步骤 1: 链接脚本导出 __kernel_end
步骤 2: frame_alloc.rs — bump 帧分配器
步骤 3: mm.rs — 页表创建 + 恒等映射 + MMU 启用
步骤 4: _start 改造 — 移除 MMU 禁用，kernel_main 中启用
步骤 5: task.rs — TCB 扩展 + create_user + 用户态调度
步骤 6: trap.rs — 用户态 eret 返回路径
步骤 7: 内嵌用户程序 + boot_qemu_virt.rs 集成
步骤 8: QEMU 验证 + CI
```
