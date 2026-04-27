# M6 剩余工作实施计划：用户态任务与 EL0 执行

> 本文档是 [M6_PLAN.md](./M6_PLAN.md) 的补充，聚焦 M6 已完成 MMU 启用后的剩余任务。

## 当前状态（M6 已完成部分）

| 组件 | 状态 | 说明 |
|------|------|------|
| 帧分配器 | ✅ | `frame_alloc.rs` — bump allocator，位图管理 32768 帧 |
| 4 级页表 | ✅ | `mm.rs` — 2MiB block mappings，恒等映射内核 RAM + 设备区 |
| MMU 启用 | ✅ | `enable_mmu()` — MAIR/TCR/TTBR0/SCTLR 正确配置，M/C/I 全启用 |
| Syscall 分发 | ✅ | `syscall.rs` + `trap.rs` — EL0/EL1 SVC 均已处理 |

**缺失核心能力**：创建 EL0 用户态任务、上下文切换到用户态、从用户态返回。

---

## 目标

在 QEMU `virt` 上运行第一个 EL0 用户态程序：
1. 用户程序通过 `svc #0` 调用 `SYS_write` 输出 `"hello from EL0!"`
2. 调用 `SYS_yield` 让出 CPU
3. 调用 `SYS_exit` 退出

---

## 模块变更计划

### 1. task.rs — TCB 扩展与用户任务创建

#### 1.1 TCB 新增字段

```rust
struct Task {
    // ... 现有字段 ...
    is_user: bool,           // 是否为 EL0 用户态任务
    user_page_table: usize,  // TTBR0_EL1 值（用户页表物理地址）
    saved_elr: u64,          // 用户态返回地址（ELR_EL1）
    saved_spsr: u64,         // 用户态 PSTATE（SPSR_EL1）
    saved_sp_el0: u64,       // 用户态栈指针
}
```

#### 1.2 新增接口

```rust
/// 创建 EL0 用户态任务
/// 
/// # Arguments
/// - `entry`: 用户程序入口虚拟地址
/// - `stack_top`: 用户栈顶虚拟地址
/// 
/// # Returns
/// - `Some(TaskId)`: 创建成功
/// - `None`: 任务表满或内存不足
pub fn create_user(entry: usize, stack_top: usize) -> Option<TaskId>;
```

#### 1.3 创建流程

1. 从 `TASK_TABLE` 分配槽位
2. 分配内核栈（4K），设置 `TaskContext`（x19=entry, lr=`user_trap_return`）
3. `is_user = true`
4. `saved_elr = entry`
5. `saved_spsr = 0x0000_0000`（EL0t, AArch64, IRQ/FIQ/SError 未屏蔽）
6. `saved_sp_el0 = stack_top`
7. 调用 `mm::create_user_page_table()` 创建用户页表
8. 映射用户代码页 + 用户栈页（恒等映射，后续可改为独立映射）
9. 状态设为 `Ready`

---

### 2. mm.rs — 用户页表支持

#### 2.1 新增接口

```rust
/// 创建空的用户页表（仅复制内核恒等映射）
pub fn create_user_page_table() -> Option<usize>;

/// 在用户页表中映射一页
/// 
/// # Safety
/// 调用者需确保页表和物理帧有效
pub unsafe fn map_user_page(table: usize, vaddr: usize, paddr: usize, attr: u64);
```

#### 2.2 实现要点

- `create_user_page_table()`：分配新页表根，复制内核 PGD[0] 的 L1 表指针（共享内核映射）
- 用户代码/栈映射在独立的 L2/L3 表中，不与内核冲突
- 属性：`PTE_AP_RW_ALL`（EL0+EL1 RW）、`PTE_UXN`（用户不可执行内核代码）、`PTE_PXN`（内核不可执行用户代码，可选）

---

### 3. trap.rs — 用户态返回路径

#### 3.1 问题分析

当前所有异常返回路径共用相同的 `eret` 逻辑，但：
- **内核任务**：`eret` 回到 EL1（当前实现）
- **用户任务**：`eret` 需要回到 EL0，且需恢复 `SP_EL0`、`ELR_EL1`、`SPSR_EL1`、切换 `TTBR0_EL1`

#### 3.2 解决方案

在 `handle_exception` 返回后，根据当前任务类型选择返回路径：

**方案 A：Rust 中分发（推荐）**

```rust
// handle_exception 末尾
if crate::task::current_is_user() {
    // 跳转到汇编 user_return
} else {
    // 正常内核返回（当前路径）
}
```

**方案 B：纯汇编判断**

在 `el0_sync_a64` / `el0_irq_a64` 返回路径中，检查 `CURRENT_TASK.is_user` 标志，分支到不同恢复逻辑。

#### 3.3 用户态返回汇编（`user_return`）

```asm
user_return:
    // 恢复用户态专用寄存器
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
    
    // 恢复通用寄存器（x0-x30 已在 TrapFrame 中）
    // ... 从栈恢复 x0-x30 ...
    
    eret  // 回到 EL0
```

#### 3.4 关键修改点

- `el0_sync_a64` 和 `el0_irq_a64` 的返回路径需区分内核/用户
- 添加 `user_return` 全局汇编符号
- `handle_exception` 返回后不再直接 `eret`，而是根据任务类型跳转

---

### 4. boot_qemu_virt.rs — 集成测试

#### 4.1 内嵌用户程序

使用 `global_asm!` 定义用户程序，链接脚本导出符号获取字节范围：

```rust
global_asm!(
    ".section .user_program, \"ax\"",
    ".global __user_program_start",
    ".global __user_program_end",
    "__user_program_start:",
    // mov x0, #1              // fd = stdout
    "mov x0, #1",
    // adr x1, msg             // msg 地址（相对 PC）
    "adr x1, 1f",
    // mov x2, #16             // len = 16
    "mov x2, #16",
    // mov x8, #0              // SYS_write
    "mov x8, #0",
    // svc #0
    "svc #0",
    // mov x8, #2              // SYS_yield
    "mov x8, #2",
    // svc #0
    "svc #0",
    // mov x8, #1              // SYS_exit
    "mov x8, #1",
    // mov x0, #0              // exit code = 0
    "mov x0, #0",
    // svc #0
    "svc #0",
    "1:",
    ".asciz \"hello from EL0!\\n\"",
    "__user_program_end:",
);

extern "C" {
    static __user_program_start: u8;
    static __user_program_end: u8;
}
```

#### 4.2 初始化流程

```rust
pub extern "C" fn kernel_main() -> ! {
    // ... 已有初始化：frame_alloc, mm, trap, gic, irq, timer, task, syscall ...
    
    // 创建内核 demo 任务（已有）
    // ...
    
    // 创建 EL0 用户任务
    let user_entry = unsafe { &__user_program_start as *const _ as usize };
    let user_stack_top = alloc_user_stack();  // 分配用户栈（4K）
    
    if let Some(user_id) = task::create_user(user_entry, user_stack_top) {
        crate::println!("[kernel] created user task {:?}", user_id);
    }
    
    // 启动调度
    task::yield_now();
    
    // idle 循环
    loop {
        task::yield_now();
    }
}
```

---

## 实施顺序

```
步骤 1: mm.rs — 添加 create_user_page_table() 和 map_user_page()
步骤 2: task.rs — 扩展 TCB，实现 create_user()
步骤 3: trap.rs — 添加 user_return 汇编，修改 el0_* 返回路径
步骤 4: link-qemu_virt.ld — 添加 .user_program 段
步骤 5: boot_qemu_virt.rs — 内嵌用户程序，创建用户任务
步骤 6: QEMU 验证 — 期望看到 "hello from EL0!" 输出
步骤 7: 代码清理 — 移除调试输出，消除 warnings
```

---

## 验证标准

```bash
cargo build -p hawthorn_kernel --features bare-metal --target aarch64-unknown-none
qemu-system-aarch64 -machine virt,gic-version=3 -cpu cortex-a57 -display none \
    -serial stdio -kernel target/aarch64-unknown-none/debug/hawthorn_kernel_qemu_virt
```

**期望输出**：
```
[kernel] created user task TaskId(4)
hello from EL0!
[task A] round 0
[task B] round 0
...
```

---

## 风险与注意事项

1. **页表切换时机**：`TTBR0_EL1` 切换后必须 `tlbi vmalle1is` + `isb`，否则 TLB 缓存导致错误翻译
2. **SPSR 设置**：`saved_spsr` 必须设置 `M[3:0] = 0b0000`（EL0t），否则 `eret` 回到错误异常级
3. **用户栈 guard page**：MVP 可省略，但建议保留 4K 未映射区域作为 guard，检测栈溢出
4. **内核/用户页表隔离**：当前方案共享 L1 表（内核映射），用户代码/栈在独立 L2。后续应完全分离 TTBR0/TTBR1
5. **调试难度**：EL0 异常（如 Data Abort）会进入 `el0_serror_a64`，需确保该路径能打印诊断信息

---

## 相关文档

- [M6_PLAN.md](./M6_PLAN.md) — M6 总体计划（含已完成部分）
- [M6_MMU_DEBUG_LOG.md](./M6_MMU_DEBUG_LOG.md) — MMU 调试与修复记录
- [PR_ISSUE_PLAN.md](./PR_ISSUE_PLAN.md) — M7 IPC 规划
- [KERNEL.md](./KERNEL.md) — 内核设计文档
