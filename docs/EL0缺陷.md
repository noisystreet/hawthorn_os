# EL0 用户态支持 — Bug 清单（已修复）

> **[English](./en/EL0_BUGS.md)** — English mirror of this document.

本文保留 EL0 bring-up 期间的缺陷记录，供回溯。

**状态更新（2026-04-29）：** 文中 1–8 项已按顺序修复并通过基础验证（包含 `hello from EL0!` 输出）。

---

## 运行状态

- EL1 内核态功能正常：MMU ✅、GIC ✅、定时器 ✅、任务调度 ✅、EL1 SVC syscall ✅
- EL0 用户态路径已启用并验证：可创建 EL0 任务并输出 `hello from EL0!`

---

## Bug 列表

### 🔴 Bug 1：`TABLE_APTABLE0` 阻止 EL0 访问整个用户页表子树

- **文件**：`kernel/src/mm.rs`（`make_table_desc` 函数，第 131–134 行）
- **现象**：所有中间页表描述符（L0/L1/L2 table descriptor）均设置了 `TABLE_APTABLE0`（bit 61），
  该位语义为"拒绝 EL0 对整个子树的访问"。即使用户页的 L3 PTE 设置了 `PTE_AP_RW_ALL`，
  硬件在遍历页表时发现上级描述符的 APTable[0]=1，会在 EL0 访问时产生 Permission fault。
- **参考**：`aarch64_kernel` 的 `identity.rs` 中同样为内核页表设置了 `TABLE_APTABLE0`，
  但它没有用户态需求；Hawthorn 需要区分内核页表和用户页表的表描述符属性。
- **修复方向**：
  1. 为用户页表新增 `make_user_table_desc()` 函数，不设置 `TABLE_APTABLE0`（bit 61 清零）
  2. `map_user_page` / `create_user_page_table` 中创建子表时使用用户态表描述符
  3. 可选：同时清除 `TABLE_UXNTABLE`（bit 60），以允许 EL0 在代码页执行

### 🔴 Bug 2：`clone_kernel_mappings` 继承内核的 `TABLE_APTABLE0` 权限

- **文件**：`kernel/src/mm.rs`（`clone_kernel_mappings` 函数，第 253–263 行）
- **现象**：用户页表直接复制内核的 PGD 条目，这些条目包含 `TABLE_APTABLE0`（bit 61）。
  导致用户页表即使映射了自己的代码/栈页，EL0 仍然无法访问（APTable 权限从内核继承）。
- **修复方向**：复制 PGD 条目时，清除 `TABLE_APTABLE0` bit（`entry & !(1 << 61)`），
  或按 Bug 1 的方向创建独立的用户态 PGD 条目。

### 🔴 Bug 3：`user_return` 汇编中 `Task` 结构体偏移量全部错误（差 +8）

- **文件**：`kernel/src/trap.rs`（`user_return` 汇编块，第 405–475 行）
- **现象**：汇编使用硬编码偏移量访问 `Task` 结构体字段，但与 `#[repr(C)]` 实际布局不匹配：

  | 字段               | 汇编偏移 | 实际偏移 | 差异 |
  |--------------------|---------|---------|------|
  | `user_page_table`  | `#56`   | **48**  | ❌ +8 |
  | `saved_elr`        | `#64`   | **56**  | ❌ +8 |
  | `saved_spsr`       | `#72`   | **64**  | ❌ +8 |
  | `saved_sp_el0`     | `#80`   | **72**  | ❌ +8 |

  偏移量错误的根因是汇编注释中的布局假设：认为 `is_user: bool`（offset 40）后需要 8 字节
  padding 才能对齐 `user_page_table: usize`（8 字节对齐），但实际 `is_user` 后只需要
  7 字节填充到 offset 48。
- **后果**：`eret` 跳转到垃圾地址 → 同步异常 → 死循环；`TTBR0_EL1` 加载错误页表。
- **修复方向**：修正汇编偏移量为 `#48`/`#56`/`#64`/`#72`；或改用 Rust 导出的常量
  （`core::mem::offset_of!`）在汇编中引用，避免硬编码。

### 🟡 Bug 4：`USER_PROGRAM` 机器码中 `ldr x1, =msg` 加载错误地址

- **文件**：`kernel/src/boot_qemu_virt.rs`（`USER_PROGRAM` 常量，第 30 行）
- **现象**：`0x58000001` 解码为 `LDR X1, [PC, #0]`，即从当前 PC 地址加载 8 字节。
  但 PC 处存放的是下一条指令 `mov x2, #16` 的机器码（`0xd2800202`），而非消息字符串的地址。
- **后果**：EL0 执行 `SYS_write` 时 `x1` 指向错误地址，syscall 写入垃圾数据或触发 fault。
- **修复方向**：使用 `ADR X1, msg`（PC 相对地址计算）替代 `LDR X1, =msg`，
  或在指令后放置正确的 literal pool 条目。

### 🟡 Bug 5：`user_task_trampoline` 在 EL1 直接调用用户代码

- **文件**：`kernel/src/task.rs`（`user_task_trampoline` 汇编，第 335–342 行）
- **现象**：`user_task_trampoline` 通过 `blr x19` 直接跳转到用户入口点，
  但此时代码运行在 EL1，不应直接执行 EL0 代码。正确做法是设置 `SPSR_EL1`/`ELR_EL1`/
  `SP_EL0` 后 `eret` 到 EL0。
- **注意**：当前代码路径中 `schedule()` 检测到 `is_user` 后调用 `user_return()` 而非
  `context_switch()`，因此 `user_task_trampoline` 的 `blr x19` 实际上是**死代码**。
  但此死代码仍是一个逻辑错误，若未来调度路径变更可能引发问题。
- **修复方向**：将 `user_task_trampoline` 改为设置 EL0 状态后 `eret`，
  或删除此 trampoline 并统一由 `user_return` 处理初始进入。

### 🟡 Bug 6：`user_return` 路径跳过 `context_switch`，不保存当前任务 sp

- **文件**：`kernel/src/task.rs`（`schedule` 函数，第 408–411 行）
- **现象**：当切换到用户任务时，`schedule()` 直接调用 `user_return(&mut TASK_TABLE[next])`
  而非 `context_switch()`。这导致：
  1. 当前任务的内核栈指针 `sp` 不会被保存到 `TASK_TABLE[current].sp`
  2. 下次切回当前任务时，内核栈丢失
  3. 如果从 EL0 任务切换到另一个 EL0 任务，两个任务的 `sp` 都会丢失
- **修复方向**：`schedule()` 应始终先通过 `context_switch()` 保存当前任务的 `sp`，
  然后在 `context_switch` 的恢复路径中检测 `is_user` 并调用 `user_return`。

### 🟡 Bug 7：EL0 异常返回路径未恢复已保存的 EL0 上下文

- **文件**：`kernel/src/trap.rs`（`el0_irq_a64` / `el0_sync_a64` 的返回路径）
- **现象**：EL0 异常处理完成后，汇编代码简单执行 `eret`，但此时：
  - `handle_exception` 中调用了 `set_current_saved_context(elr, spsr, sp_el0)` 保存旧值
  - 但没有将 Task 中保存的值写回 `ELR_EL1`/`SPSR_EL1`/`SP_EL0`
  - 如果 `schedule()` 切换了任务，返回时 `eret` 回到的不是原来的用户任务
- **参考**：`aarch64_kernel` 的 `vectors.S` 在 `exception_exit` 和 `exit` 路径中
  都显式恢复 `spsr_el1` 和 `elr_el1`（`msr spsr_el1, x2; msr elr_el1, x3`）。
- **修复方向**：在 EL0 异常返回路径中，从当前任务的 `saved_elr`/`saved_spsr`/`saved_sp_el0`
  恢复系统寄存器后再 `eret`；或者在 `handle_exception` 返回前写回这些寄存器。

### 🟠 Bug 8：`saved_spsr = 0x0000_0000` 覆盖 NZCV 条件标志

- **文件**：`kernel/src/task.rs`（`create_user` 函数，第 288 行）
- **现象**：SPSR 值 `0x0000_0000` 将 NZCV 条件标志位（bits [31:28]）清零，
  覆盖用户程序可能设置的条件标志。
- **影响**：对于初始入口可接受（EL0 程序从零开始执行），但应在注释中注明。
  真正的问题在于后续从异常返回时是否正确恢复原始 SPSR（见 Bug 7）。
- **修复方向**：低优先级，仅改进注释。

---

## 修复优先级建议

1. **Bug 1 + Bug 2**（页表权限）：最基础，不修复则 EL0 任何访问都会 fault
2. **Bug 3**（汇编偏移量）：不修复则 `eret` 跳转非法地址
3. **Bug 4**（USER_PROGRAM 机器码）：不修复则 syscall 传垃圾指针
4. **Bug 6**（schedule 保存 sp）：不修复则用户任务无法正确切回
5. **Bug 7**（EL0 异常返回路径）：不修复则调度后 eret 回到错误任务
6. **Bug 5**（user_task_trampoline）：当前为死代码，优先级较低
7. **Bug 8**（saved_spsr 标志覆盖）：影响最小

---

## 验证方法

每修复一个 bug 后，应执行：

```bash
cargo build -p hawthorn_kernel --features bare-metal --target aarch64-unknown-none
timeout 10 qemu-system-aarch64 -machine virt,gic-version=3 -cpu cortex-a76 \
  -display none -serial file:/tmp/hawthorn_serial.log \
  -kernel target/aarch64-unknown-none/debug/hawthorn_kernel_qemu_virt
cat /tmp/hawthorn_serial.log
```

确保：
1. 原有 EL1 功能（任务调度、SVC syscall）不被破坏
2. 新增 EL0 功能逐步生效（从 "hello from EL0!" 输出开始验证）

---

## 另见

- bring-up 之后与 **syscall 返回值、`SYS_write` 指针语义、用户任务资源回收** 相关的修复见 [缺陷修复笔记.md](./缺陷修复笔记.md)。
