# Exception / Trap Design

> **[中文](../TRAP.md)** — Chinese source of this document.

This document describes the AArch64 exception vector table, trap entry, context save, and exception dispatch design for the **Hawthorn (山楂) microkernel**. The current milestone (M2) covers **EL1 exception handling only**; EL0 user-space traps and EL2/EL3 switching will be extended in later milestones.

---

## 1. Overview

Under the AArch64 exception model, all synchronous exceptions and asynchronous interrupts enter through a **Vector Table**. The kernel must write the vector table base address to `VBAR_EL1` during early initialization; otherwise any exception (page fault, misalignment, IRQ) jumps to `0x0` or `0x200` etc., causing a CPU infinite loop.

Hawthorn kernel exception handling responsibilities:

- **Hardware entry**: assembly saves context → jumps to Rust dispatch function.
- **Classify & dispatch**: route to the appropriate handler based on exception type (sync / IRQ / FIQ / SError).
- **Context restore**: after handling, restore registers from the saved stack frame and `eret`.

---

## 2. AArch64 Vector Table Layout

AArch64 `VBAR_EL1` points to 16 vector slots, each 128 bytes (0x80) aligned. The table is organized into 4 groups by **exception level at the time of exception** and **stack pointer selection (SP)**:

| Offset   | Exception Type | Scenario                                   |
|----------|---------------|---------------------------------------------|
| `0x000`  | Sync          | Current EL, using SP_EL0                    |
| `0x080`  | IRQ           | Current EL, using SP_EL0                    |
| `0x100`  | FIQ           | Current EL, using SP_EL0                    |
| `0x180`  | SError        | Current EL, using SP_EL0                    |
| `0x200`  | Sync          | Current EL, using SP_ELx (e.g. SP_EL1)      |
| `0x280`  | IRQ           | Current EL, using SP_ELx                    |
| `0x300`  | FIQ           | Current EL, using SP_ELx                    |
| `0x380`  | SError        | Current EL, using SP_ELx                    |
| `0x400`  | Sync          | Lower EL (EL0→EL1), AArch64                 |
| `0x480`  | IRQ           | Lower EL (EL0→EL1), AArch64                 |
| `0x500`  | FIQ           | Lower EL (EL0→EL1), AArch64                 |
| `0x580`  | SError        | Lower EL (EL0→EL1), AArch64                 |
| `0x600`  | Sync          | Lower EL (EL0→EL1), AArch32                 |
| `0x680`  | IRQ           | Lower EL (EL0→EL1), AArch32                 |
| `0x700`  | FIQ           | Lower EL (EL0→EL1), AArch32                 |
| `0x780`  | SError        | Lower EL (EL0→EL1), AArch32                 |

### 2.1 Active Vector Slots in Hawthorn

Current stage (M2, EL1 kernel only), **active slots** are:

| Offset   | Purpose                                | Initial Implementation               |
|----------|----------------------------------------|--------------------------------------|
| `0x200`  | EL1 synchronous exception (SPx)        | Print ESR/ELR/FAR, infinite loop     |
| `0x280`  | EL1 IRQ (SPx)                          | GICv3 acknowledge → `irq::dispatch()` → EOI |
| `0x300`  | EL1 FIQ (SPx)                          | Unused, infinite loop                |
| `0x380`  | EL1 SError (SPx)                       | Print ESR, infinite loop             |
| `0x400`  | EL0→EL1 sync exception (AArch64)       | Syscall entry (SVC #0)              |
| `0x480`  | EL0→EL1 IRQ (AArch64)                  | Device interrupt delivery (GIC → thread) |
| `0x580`  | EL0→EL1 SError (AArch64)               | Print ESR, kill thread or infinite loop |

Remaining slots (EL0 SP0 group, AArch32 group) jump to a **generic exception stub** that prints diagnostics and enters an infinite loop.

---

## 3. Exception Context Save (Trap Frame)

### 3.1 Saved Contents

On exception entry, hardware automatically saves `ELR_EL1` and `SPSR_EL1` (old PSTATE) into system registers. Software must save general-purpose registers `x0`–`x30` + `SP_EL0`, totaling 32 × 64-bit values.

**Trap Frame layout** (grows from high to low address, stack-top aligned):

```
┌──────────────────┐ ← high address (stack bottom)
│   SP_EL0         │  offset +248
│   x30 (LR)       │  offset +240
│   x29 (FP)       │  offset +232
│   x28            │  offset +224
│   ...            │
│   x1             │  offset +8
│   x0             │  offset +0   ← low address (stack top)
├──────────────────┤
│   ELR_EL1        │  (below stack top, saved separately by asm)
│   SPSR_EL1       │
└──────────────────┘
```

### 3.2 Stack Selection

- **EL1 exceptions** (`0x200`/`0x280` group): use **SP_EL1** (kernel stack). Each CPU core needs a dedicated kernel exception stack; single-core stage uses the main stack defined by `__stack_top` in the linker script.
- **EL0→EL1 exceptions** (`0x400`/`0x480` group): after entering EL1, use **SP_EL1**. Once threads are introduced, each TCB has its own kernel stack.

### 3.3 Stack Depth Budget

| Exception path                        | Estimated stack consumption |
|---------------------------------------|-----------------------------|
| Bare context save (asm entry)         | 256B (trap frame)           |
| Rust handler call chain               | ≤512B (no heap allocation)  |
| Nested exception (sync during IRQ)    | +256B (second trap frame)   |
| **Single-core minimum kernel stack**  | **4 KiB** (with safety margin) |

Single-core stage uses the stack at the end of 128 MiB RAM defined by the linker script; SMP stage needs a dedicated exception stack (≥4 KiB/core) per CPU.

---

## 4. Assembly Entry Convention

Each vector slot's assembly entry follows a uniform pattern:

```asm
// Example: EL1 Sync (SPx) @ 0x200
.align 7
vector_el1_sync_spx:
    // 1. Save general-purpose registers to kernel stack
    sub sp, sp, #256           // allocate trap frame
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

    // 2. Save SP_EL0 and exception return info
    mrs x0, sp_el0
    str x0, [sp, #248]
    mrs x0, elr_el1
    mrs x1, spsr_el1

    // 3. Call Rust dispatch function
    //    Prototype: fn handle_exception(kind: ExceptionKind, trap_frame: &mut TrapFrame, elr: u64, spsr: u64)
    mov x2, sp                  // trap_frame pointer
    bl handle_exception

    // 4. Restore SP_EL0
    ldr x0, [sp, #248]
    msr sp_el0, x0

    // 5. Restore general-purpose registers
    ldp x0, x1,   [sp, #0]
    ldp x2, x3,   [sp, #16]
    // ... (symmetric restore)
    ldp x28, x29, [sp, #224]
    ldr x30,      [sp, #240]

    // 6. Restore ELR/SPSR and return
    add sp, sp, #256
    eret
```

### 4.1 Calling Convention

| Register | Content at entry     | Purpose passed to Rust                  |
|----------|----------------------|-----------------------------------------|
| `x0`     | Exception kind index | `ExceptionKind` enum value              |
| `x1`     | —                    | Reserved / unused                       |
| `x2`     | `sp` (trap frame)    | `&mut TrapFrame`                        |
| `x3`     | `ELR_EL1`            | Exception return address                |
| `x4`     | `SPSR_EL1`           | Saved PSTATE                            |

Rust dispatch function signature:

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

> Note: after `handle_exception` returns, the asm entry restores registers from the trap frame and executes `eret`. If the return address needs modification (e.g. signal injection or syscall redirection), the Rust handler directly modifies the `SP_EL0` field in the trap frame or writes `ELR_EL1`.

---

## 5. Exception Kinds & Dispatch

### 5.1 ExceptionKind Enum

```rust
#[repr(u64)]
enum ExceptionKind {
    El1SyncSpx  = 0,  // 0x200: EL1 synchronous exception (SPx)
    El1IrqSpx   = 1,  // 0x280: EL1 IRQ
    El1FiqSpx   = 2,  // 0x300: EL1 FIQ
    El1SErrorSpx = 3, // 0x380: EL1 SError
    El0SyncA64  = 4,  // 0x400: EL0→EL1 synchronous exception (AArch64)
    El0IrqA64   = 5,  // 0x480: EL0→EL1 IRQ (AArch64)
    El0FiqA64   = 6,  // 0x500: EL0→EL1 FIQ (AArch64)
    El0SErrorA64 = 7, // 0x580: EL0→EL1 SError (AArch64)
}
```

### 5.2 Synchronous Exception Classification (ESR Parsing)

On synchronous exception, `ESR_EL1` (Exception Syndrome Register) contains the exception cause:

| ESR.EC  | Meaning                                | Hawthorn Handling                         |
|---------|----------------------------------------|-------------------------------------------|
| `0x00`  | Unknown reason                         | Print diagnostics, infinite loop          |
| `0x15`  | SVC instruction execution (EL0)        | **Syscall dispatch**                      |
| `0x24`  | Data Abort (EL0, sync/async)           | User page fault → map or kill thread      |
| `0x25`  | Data Abort (EL1)                       | Kernel bug, print diagnostics & loop      |
| `0x20`  | Instruction Abort (EL0)                | Code page fault → map or kill thread      |
| `0x21`  | Instruction Abort (EL1)                | Kernel bug, print diagnostics & loop      |

**M2 stage**: only EL1 sync exceptions (print ESR/ELR/FAR + loop) and SVC (syscall stub) are handled. EL0 Data/Instruction Abort will be implemented when user-space is introduced.

### 5.3 IRQ Handling Flow

```
IRQ entry (assembly)
    → save trap frame
    → handle_exception(El1IrqSpx / El0IrqA64, ...)
        → gic_acknowledge()           // read IAR, get IRQ number
        → dispatch_irq(irq_num)       // route to handler by IRQ number
            → timer_irq_handler()     // Generic Timer PPI
            → uart_irq_handler()      // PL011 (later)
            → ...                     // other registered handlers
        → gic_end_of_interrupt(irq_num) // write EOIR
    → restore trap frame
    → eret
```

**M2 stage**: IRQ handler routes through `irq::dispatch()` to registered handlers (`gic::ack()` → table lookup → call handler → `gic::eoi()`). Unregistered interrupt numbers perform acknowledge + EOI only and return. Device-specific interrupts are registered via `irq::register()` once their drivers are ready.

---

## 6. TrapFrame Struct

```rust
#[repr(C)]
struct TrapFrame {
    x: [u64; 31],   // x0–x30
    sp_el0: u64,     // SP_EL0
}
```

Total size: 32 × 8 = 256 bytes, matching the `sub sp, sp, #256` in assembly.

---

## 7. Initialization

In `kernel_main`, the exception vector table must be registered **after BSS zeroing + UART init, but before any operation that could trigger an exception**. GICv3 and the IRQ dispatch framework are initialized immediately after:

```rust
pub extern "C" fn kernel_main() -> ! {
    unsafe { zero_bss() };
    unsafe { pl011_init() };

    // Set up exception vector table
    trap::init();

    // Initialize GICv3 (Distributor + Redistributor + CPU Interface)
    unsafe { gic::init() };

    // Initialize IRQ dispatch table (must follow GIC init)
    irq::init();

    crate::println!("Hawthorn: hawthorn_kernel on QEMU virt OK");
    loop { core::hint::spin_loop(); }
}
```

`trap::init()` implementation:

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

The vector table symbol `__exception_vector_table` is defined in an assembly file, placed in the `.text.vector` section, and `KEEP`-ed by the linker script.

---

## 8. Linker Script Changes

Add the vector table section to `.text` in `link-qemu_virt.ld`:

```
.text : {
    KEEP(*(.text.boot))          /* _start entry */
    KEEP(*(.text.vector))        /* Exception vector table, 128-byte aligned */
    *(.text .text.*)
} > RAM
```

In the vector table assembly, use `.section .text.vector, "ax"` + `.align 12` (4 KiB alignment, satisfying the 0x80 slot alignment requirement).

---

## 9. Code Module Layout

```
kernel/src/
├── trap.rs             # pub fn init(), TrapFrame, ExceptionKind, handle_exception()
├── gic.rs              # GICv3 driver: Distributor / Redistributor / CPU Interface init
├── irq.rs              # IRQ dispatch framework: register / unregister / dispatch
├── boot_qemu_virt.rs   # kernel_main calls trap::init() → gic::init() → irq::init()
└── lib.rs              # pub mod trap; pub mod gic; pub mod irq;
```

- `trap/mod.rs`: Rust-side dispatch logic, `TrapFrame` definition, `ExceptionKind` enum. IRQ exceptions dispatch to `irq::dispatch()`.
- `gic.rs`: GICv3 driver (Distributor / Redistributor / CPU Interface initialization and interrupt enable/disable).
- `irq.rs`: IRQ dispatch framework, maintains a 1020-slot handler table, provides `register` / `unregister` / `dispatch`.
- `trap/vector.asm`: 16 vector slot assembly entries. Recommended to use Rust `global_asm!` macro inline to keep a single crate without extra `.S` files. If the assembly is long, split into `trap/vector.s` and compile via `build.rs`.

---

## 10. Future Extensions

| Milestone | Extension                                                  |
|-----------|------------------------------------------------------------|
| M2        | EL1 vector table + EL1 sync diagnostics + GICv3 init + IRQ dispatch |
| M3        | SVC entry (`ESR.EC = 0x15`) → syscall dispatch            |
| M3        | EL0 Data/Instruction Abort → page fault or thread kill     |
| M3+       | Per-thread kernel stack (TCB.kernel_sp), context switch SP |
| SMP       | Per-core VBAR_EL1 (same vector table, different stacks)   |
| EL2       | If EL2→EL1 downgrade needed, add `VBAR_EL2` vector table  |

---

## Related documents

- [Microkernel design](./KERNEL.md) — §3.6 Interrupts & exceptions, §3.8 Syscall interface
- [Boot skeleton](./BOOT.md) — Boot phases & early initialization
- [Porting](./PORTING.md) — QEMU virt runtime environment
- [Code style](./CODE_STYLE.md) — unsafe & SAFETY comment conventions
- ARM Architecture Reference Manual (ARM DDI 0487) — D1.10 Exception Vectors
