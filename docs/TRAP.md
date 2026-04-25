# 异常与陷入设计（Exception / Trap）

> **[English](./en/TRAP.md)** — English mirror of this document.

本文档描述 **山楂（hawthorn）微内核** 的 AArch64 异常向量表、陷入入口、上下文保存与异常分发设计。当前阶段（M2）仅覆盖 **EL1 异常处理**；后续 EL0 用户态陷入与 EL2/EL3 切换将在对应里程碑扩展。

---

## 1. 概述

AArch64 异常模型下，所有同步异常与异步中断通过 **向量表（Vector Table）** 统一入口。内核必须在早期初始化中将向量表基址写入 `VBAR_EL1`，否则任何异常（包括缺页、未对齐访问、IRQ）都会跳到 `0x0` 或 `0x200` 等无效地址，导致 CPU 死循环。

山楂内核的异常处理职责：

- **硬件入口**：汇编保存上下文 → 跳转到 Rust 分发函数。
- **分类与分发**：根据异常类型（sync / IRQ / FIQ / SError）路由到对应处理逻辑。
- **上下文恢复**：处理完毕后从保存的栈帧恢复寄存器，`eret` 返回。

---

## 2. AArch64 向量表布局

AArch64 `VBAR_EL1` 指向 16 个 128 字节（0x80）对齐的向量槽。每个异常级别（EL）的向量表按 **异常发生时的异常级别（EL）** 与 **栈指针选择（SP）** 分为 4 组，每组 4 个槽位：

| 偏移     | 异常类型   | 发生场景                              |
|----------|-----------|---------------------------------------|
| `0x000`  | Sync       | 当前 EL，使用 SP_EL0                  |
| `0x080`  | IRQ       | 当前 EL，使用 SP_EL0                  |
| `0x100`  | FIQ       | 当前 EL，使用 SP_EL0                  |
| `0x180`  | SError    | 当前 EL，使用 SP_EL0                  |
| `0x200`  | Sync       | 当前 EL，使用 SP_ELx（如 SP_EL1）     |
| `0x280`  | IRQ       | 当前 EL，使用 SP_ELx                  |
| `0x300`  | FIQ       | 当前 EL，使用 SP_ELx                  |
| `0x380`  | SError    | 当前 EL，使用 SP_ELx                  |
| `0x400`  | Sync       | 低 EL（EL0→EL1），使用 AArch64        |
| `0x480`  | IRQ       | 低 EL（EL0→EL1），使用 AArch64        |
| `0x500`  | FIQ       | 低 EL（EL0→EL1），使用 AArch64        |
| `0x580`  | SError    | 低 EL（EL0→EL1），使用 AArch64        |
| `0x600`  | Sync       | 低 EL（EL0→EL1），使用 AArch32        |
| `0x680`  | IRQ       | 低 EL（EL0→EL1），使用 AArch32        |
| `0x700`  | FIQ       | 低 EL（EL0→EL1），使用 AArch32        |
| `0x780`  | SError    | 低 EL（EL0→EL1），使用 AArch32        |

### 2.1 山楂使用的向量槽

当前阶段（M2，仅 EL1 内核态），**活跃的槽位**为：

| 偏移     | 用途                                   | 初始实现                    |
|----------|----------------------------------------|-----------------------------|
| `0x200`  | EL1 同步异常（SPx）                    | 打印 ESR/ELR/FAR，死循环    |
| `0x280`  | EL1 IRQ（SPx）                         | 桩：确认 GIC，返回          |
| `0x300`  | EL1 FIQ（SPx）                         | 未使用，死循环              |
| `0x380`  | EL1 SError（SPx）                      | 打印 ESR，死循环            |
| `0x400`  | EL0→EL1 同步异常（AArch64）            | 系统调用入口（SVC #0）      |
| `0x480`  | EL0→EL1 IRQ（AArch64）                 | 外设中断递交（GIC → 线程）  |
| `0x580`  | EL0→EL1 SError（AArch64）              | 打印 ESR，杀线程或死循环    |

其余槽位（EL0 SP0 组、AArch32 组）在初始实现中跳到 **通用异常桩**，输出诊断信息后死循环。

---

## 3. 异常上下文保存（Trap Frame）

### 3.1 保存内容

异常发生时，硬件自动保存 `ELR_EL1`、`SPSR_EL1`（旧 PSTATE）到系统寄存器。软件需要保存的通用寄存器为 `x0`–`x30` + `SP_EL0`，共计 32 个 64 位值。

**Trap Frame 布局**（从高地址向低地址生长，栈顶对齐）：

```
┌──────────────────┐ ← 高地址（栈底方向）
│   SP_EL0         │  offset +248
│   x30 (LR)       │  offset +240
│   x29 (FP)       │  offset +232
│   x28            │  offset +224
│   ...            │
│   x1             │  offset +8
│   x0             │  offset +0   ← 低地址（栈顶方向）
├──────────────────┤
│   ELR_EL1        │  (栈顶下方，汇编额外保存)
│   SPSR_EL1       │
└──────────────────┘
```

### 3.2 栈选择

- **EL1 异常**（`0x200`/`0x280` 组）：使用 **SP_EL1**（内核栈）。每 CPU 核需有独立的内核异常栈；单核阶段使用链接脚本中 `__stack_top` 定义的主栈。
- **EL0→EL1 异常**（`0x400`/`0x480` 组）：进入 EL1 后使用 **SP_EL1**。后续引入线程后，每个 TCB 拥有自己的内核栈。

### 3.3 栈深度预算

| 异常路径                      | 预估栈消耗     |
|-------------------------------|---------------|
| 裸上下文保存（汇编 entry）     | 256B（trap frame） |
| Rust handler 调用链            | ≤512B（无堆分配） |
| 嵌套异常（IRQ 中发生 sync）    | +256B（第二次 trap frame） |
| **单核最小内核异常栈**         | **4 KiB**（含安全余量） |

单核阶段使用链接脚本定义的 128MB RAM 末尾栈；多核阶段需为每个 CPU 核分配独立异常栈（≥4 KiB/核）。

---

## 4. 汇编入口约定

每个向量槽的汇编入口遵循统一模式：

```asm
// 以 EL1 Sync (SPx) @ 0x200 为例
.align 7
vector_el1_sync_spx:
    // 1. 保存通用寄存器到内核栈
    sub sp, sp, #256           // 分配 trap frame 空间
    stp x0, x1,   [sp, #0]
    stp x2, x3,   [sp, #16]
    stp x4, x5,   [sp, #32]
    stp x6, x7,   [sp, #48]
    stp x8, x9,   [sp, #64]
    stp x10, x11, [sp, #80]
    stp x12, x13, [sp, #96]
    stp x14, x15, [sp, #112]
    stp x16, x17, [sp, #128]
    stp x18, x19, [sp, #144]
    stp x20, x21, [sp, #160]
    stp x22, x23, [sp, #176]
    stp x24, x25, [sp, #192]
    stp x26, x27, [sp, #208]
    stp x28, x29, [sp, #224]
    str x30,      [sp, #240]

    // 2. 保存 SP_EL0 与异常返回信息
    mrs x0, sp_el0
    str x0, [sp, #248]
    mrs x0, elr_el1
    mrs x1, spsr_el1

    // 3. 调用 Rust 分发函数
    //    原型: fn handle_exception(kind: ExceptionKind, trap_frame: &mut TrapFrame, elr: u64, spsr: u64)
    mov x2, sp                  // trap_frame 指针
    bl handle_exception

    // 4. 恢复 SP_EL0
    ldr x0, [sp, #248]
    msr sp_el0, x0

    // 5. 恢复通用寄存器
    ldp x0, x1,   [sp, #0]
    ldp x2, x3,   [sp, #16]
    // ... (对称恢复)
    ldp x28, x29, [sp, #224]
    ldr x30,      [sp, #240]

    // 6. 恢复 ELR/SPSR 并返回
    add sp, sp, #256
    eret
```

### 4.1 调用约定

| 寄存器 | 入口时内容         | 传给 Rust 的用途               |
|--------|--------------------|---------------------------------|
| `x0`   | 异常种类编号       | `ExceptionKind` 枚举值         |
| `x1`   | —                  | 保留/未使用                     |
| `x2`   | `sp`（trap frame） | `&mut TrapFrame`               |
| `x3`   | `ELR_EL1`          | 异常返回地址                    |
| `x4`   | `SPSR_EL1`         | 保存的 PSTATE                   |

Rust 分发函数签名为：

```rust
#[no_mangle]
unsafe extern "C" fn handle_exception(
    kind: ExceptionKind,
    _reserved: u64,
    trap_frame: *mut TrapFrame,
    elr: u64,
    spsr: u64,
)
```

> 注意：`handle_exception` 返回后，汇编入口从 trap frame 恢复寄存器并 `eret`。若需修改返回地址（如信号注入或系统调用重定向），Rust handler 直接修改 trap frame 中的 `SP_EL0` 字段或修改 `ELR_EL1`。

---

## 5. 异常种类与分发

### 5.1 ExceptionKind 枚举

```rust
#[repr(u64)]
enum ExceptionKind {
    El1SyncSpx  = 0,  // 0x200: EL1 同步异常（SPx）
    El1IrqSpx   = 1,  // 0x280: EL1 IRQ
    El1FiqSpx   = 2,  // 0x300: EL1 FIQ
    El1SErrorSpx = 3, // 0x380: EL1 SError
    El0SyncA64  = 4,  // 0x400: EL0→EL1 同步异常（AArch64）
    El0IrqA64   = 5,  // 0x480: EL0→EL1 IRQ（AArch64）
    El0FiqA64   = 6,  // 0x500: EL0→EL1 FIQ（AArch64）
    El0SErrorA64 = 7, // 0x580: EL0→EL1 SError（AArch64）
}
```

### 5.2 同步异常分类（ESR 解析）

同步异常发生时，`ESR_EL1`（Exception Syndrome Register）包含异常原因：

| ESR.EC 值 | 含义                          | 山楂处理                         |
|-----------|-------------------------------|----------------------------------|
| `0x00`    | Unknown reason                | 打印诊断，死循环                 |
| `0x15`    | SVC 指令执行（EL0）           | **系统调用分发**                 |
| `0x24`    | Data Abort（EL0，同/不同步）  | 用户态缺页 → 映射或杀线程        |
| `0x25`    | Data Abort（EL1）             | 内核 bug，打印诊断并死循环       |
| `0x20`    | Instruction Abort（EL0）      | 代码缺页 → 映射或杀线程          |
| `0x21`    | Instruction Abort（EL1）      | 内核 bug，打印诊断并死循环       |

**M2 阶段**：仅处理 EL1 同步异常（打印 ESR/ELR/FAR + 死循环）与 SVC（系统调用桩）。EL0 Data/Instruction Abort 在引入用户态后实现。

### 5.3 IRQ 处理流程

```
IRQ 入口 (汇编)
    → 保存 trap frame
    → handle_exception(El1IrqSpx / El0IrqA64, ...)
        → gic_acknowledge()           // 读取 IAR，获取中断号
        → dispatch_irq(irq_num)       // 按中断号路由到 handler
            → timer_irq_handler()     // Generic Timer PPI
            → uart_irq_handler()      // PL011 (后期)
            → ...                     // 其他已注册 handler
        → gic_end_of_interrupt(irq_num) // 写入 EOIR
    → 恢复 trap frame
    → eret
```

**M2 阶段**：IRQ handler 仅包含 GIC acknowledge + EOI 桩，不挂接具体设备中断。实际设备中断在 GICv3 初始化 + Timer 驱动就绪后挂接。

---

## 6. TrapFrame 结构体

```rust
#[repr(C)]
struct TrapFrame {
    x: [u64; 31],   // x0–x30
    sp_el0: u64,     // SP_EL0
}
```

总大小：32 × 8 = 256 字节，与汇编中 `sub sp, sp, #256` 一致。

---

## 7. 初始化流程

在 `kernel_main` 中，异常向量表应在 **BSS 清零 + UART 初始化之后、任何可能触发异常的操作之前** 注册：

```rust
pub extern "C" fn kernel_main() -> ! {
    unsafe { zero_bss() };
    unsafe { pl011_init() };

    // 设置异常向量表
    trap::init();

    crate::println!("Hawthorn: hawthorn_kernel on QEMU virt OK");
    // 此后任何异常都会被向量表捕获，而非跳到 0x200 死循环
    loop { core::hint::spin_loop(); }
}
```

`trap::init()` 实现：

```rust
pub fn init() {
    extern "C" {
        static __exception_vector_table: u8;
    }
    unsafe {
        let vbar = core::ptr::addr_of!(__exception_vector_table) as u64;
        core::arch::asm!("msr vbar_el1, {0}", in(reg) vbar);
        core::arch::asm!("isb");
    }
}
```

向量表符号 `__exception_vector_table` 由汇编文件定义，放入 `.text.vector` 段，链接脚本 `KEEP` 保留。

---

## 8. 链接脚本变更

在 `link-qemu_virt.ld` 的 `.text` 段中增加向量表段：

```
.text : {
    KEEP(*(.text.boot))          /* _start 入口 */
    KEEP(*(.text.vector))        /* 异常向量表，128 字节对齐 */
    *(.text .text.*)
} > RAM
```

向量表汇编文件中，使用 `.section .text.vector, "ax"` + `.align 12`（4KiB 对齐，满足 0x80 槽位对齐要求）。

---

## 9. 代码模块划分

```
kernel/src/
├── trap/
│   ├── mod.rs           # pub fn init(), TrapFrame, ExceptionKind, handle_exception()
│   └── vector.asm       # 16 个向量槽的汇编入口（global_asm! 或 .S 文件）
├── boot_qemu_virt.rs    # kernel_main 中调用 trap::init()
└── lib.rs               # pub mod trap;
```

- `trap/mod.rs`：Rust 侧的分发逻辑、`TrapFrame` 定义、`ExceptionKind` 枚举。
- `trap/vector.asm`：16 个向量槽汇编入口。建议使用 Rust `global_asm!` 宏内联，保持单一 crate 无需额外 `.S` 文件。若汇编较长，可拆为 `trap/vector.s` 并在 `build.rs` 中编译。

---

## 10. 后续扩展

| 里程碑 | 扩展内容                                          |
|--------|---------------------------------------------------|
| M2     | EL1 向量表 + EL1 sync 打印诊断 + IRQ 桩           |
| M3     | SVC 入口（`ESR.EC = 0x15`）→ syscall 分发         |
| M3     | EL0 Data/Instruction Abort → 缺页处理或线程终止    |
| M3+    | 每线程内核栈（TCB.kernel_sp），上下文切换修改 SP   |
| SMP    | 每核 VBAR_EL1（相同向量表，不同异常栈）            |
| EL2    | 若需 EL2→EL1 降级，增加 `VBAR_EL2` 向量表         |

---

## 相关文档

- [微内核设计](./KERNEL.md) — §3.6 中断与异常、§3.8 系统调用接口
- [启动骨架](./BOOT.md) — 引导阶段与早期初始化
- [移植指南](./PORTING.md) — QEMU virt 运行环境
- [代码风格](./CODE_STYLE.md) — unsafe 与 SAFETY 注释约定
- ARM Architecture Reference Manual (ARM DDI 0487) — D1.10 Exception Vectors
