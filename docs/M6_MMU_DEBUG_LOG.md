# M6 QEMU `virt` MMU 启用调试与修复记录

> **English:** [en/M6_MMU_DEBUG_LOG.md](./en/M6_MMU_DEBUG_LOG.md)

本文记录 Hawthorn 在 QEMU `virt` 上启用 AArch64 MMU 时的故障现象、排查过程与**最终修复**（`scripts/verify_kernel_qemu_virt_serial.sh` 已通过）。

## 1. 现象

- `bash scripts/verify_kernel_qemu_virt_serial.sh` 超时，串口看不到  
  `Hawthorn: hawthorn_kernel on QEMU virt OK`。
- 日志通常能打印到 **`[mm/step4] ... MMU NOT enabled yet`**，在 **`enable_mmu_step5`**（写入 `SCTLR_EL1` 打开 **M**）之后无输出或挂死。
- 若用 QEMU 跟踪，可见 **Prefetch Abort / Translation fault** 等（与具体配置阶段有关）。

## 2. 根因归纳（按重要性）

### 2.1 `T0SZ` 与页表层级不一致

- AArch64 在 **4 KiB 粒度**下，**起始 lookup level 由 `TCR_EL1.T0SZ` 决定**。
- 曾使用 **`VA_BITS_T0 = 39`**（`T0SZ = 25`），硬件从 **L1** 起 walk，而内核构建的是 **以 L0 为根的四级表**，`TTBR0` 指向的表被当成 L1 解释，翻译错误。
- **修复：** 使用 **`VA_BITS_T0 = 48`**（`T0SZ = 16`），从 **L0** 起 walk，与 `map_block_2m` / `map_page` 一致。

### 2.2 页表 AP / UXN / PXN / 表属性与参考实现不一致

对照本机参考工程 **`aarch64_kernel`**（`kernel/mm/bits.rs`、`identity.rs`）后：

| 项 | 问题倾向 | 对齐方式 |
|----|----------|----------|
| **AP** | 使用 **`0b01 << 6`（EL1+EL0 RW）** 与参考的 **`DESC_AP_RW_EL1`（`0b00`，仅 EL1 RW）** 不一致，开 MMU 后行为与预期不符 | 内核 RAM / 低设备区改用 **`PTE_AP_RW_EL1`**；用户页将来用 **`PTE_AP_RW_ALL`** |
| **可执行性** | 未按常见 idmap 设置 **UXN/PXN** | Normal RAM：**`PTE_UXN`**；Device：**`PTE_UXN | PTE_PXN`** |
| **表描述符** | 仅 `valid \| table` | 增加 **`TABLE_UXNTABLE | TABLE_APTABLE0`**（与参考 L0 表属性一致） |

> **勘误（旧稿）：** 曾误写「AP=00 只允许 EL1 读」。在 AArch64 页表语义中，**`AP[2:1]=00` 表示 EL1 可读写、EL0 无访问**；**`01` 为 EL1 与 EL0 均可读写**。最终以 Arm ARM 与参考代码为准。

### 2.3 `TCR_EL1` 缺少 `IPS` 字段与 `EPD1`

- **IPS（Intermediate Physical Address Size）**：`TCR_EL1.IPS`（bits [34:32]）决定 MMU walker 允许的物理地址宽度。若 **IPS=0**（默认），PA 仅 **32 位**（4 GiB），而 RAM 起始 **0x40000000** 已超出该范围，walker 拒绝翻译 → **Translation fault**。
- **修复：** 从 `ID_AA64MMFR0_EL1.PARange` 动态读取 PA 大小（QEMU virt 为 44 位，编码 4），写入 `IPS` 字段。
- **EPD1（TTBR1 禁用）**：未设置 `EPD1=1` 时，高位地址翻译可能走 `TTBR1_EL1`（值为 0），产生额外 Translation fault。添加 `EPD1=1`（bit 23）。

### 2.4 `SCTLR_EL1.WXN` 与全 RW 映射

- 若 **`SCTLR_EL1.WXN = 1`**，可写区域在架构上视为 **不可执行**；早期 idmap 对 RAM 使用 **RW** 时，开 MMU 后取指可能失败。
- **修复：** 在写入 `SCTLR` 打开 **M** 时 **清除 `WXN`**（与其它保留位一起基于 `mrs` 再合并）。

### 2.5 启动与异常路径

- **`VBAR_EL1`**：须在打开 MMU **之前**安装，否则异常/取指路径可能落到未初始化向量。
- **`SPSel`**：若 **`SPSel = 0`**，EL1 使用 `SP_EL0`，异常走 VBAR **0x0–0x180** 槽位；工程里 **`generic_stub` 为 `b .`** 时会**无声死循环**。在 `_start` 中 **`msr spsel, #1`**。
- **EL2 → EL1**：**禁止**用 **`msr sctlr_el1, xzr`** 清 `SCTLR_EL1`（破坏 **RES1** 等）；应到 EL1 后再 **读-改-写** 仅关闭 **M/C/I**。
- **IRQ**：MMU 已开、**GIC 未初始化**前若进入 **`irq::dispatch` → `ICC_IAR1_EL1`** 可能异常。参考 **`head.S`**：入口 **`msr daifset, #0xf`**；`enable_mmu_step5` 前再次保证掩码一致思路。

### 2.6 `SCTLR` 与缓存

- 参考 **`aarch64_kernel`** 的 `enable_mmu()` 仅 **`orr` 打开 M**。
- 本仓库 **`_start`** 会清除 **C/I**，因此在 **M** 打开并 **`ic iallu` / `dsb` / `isb`** 后，**第二次写入 `SCTLR` 再打开 C、I**。

### 2.7 D-Cache 一致性（`dc civac`）

- 在 MMU 启用前，页表写入可能残留在数据缓存中，对 MMU walker 不可见。
- **修复：** 在 `mm::init()` 末尾，对页表所在区域执行 **`dc civac`**（Clean & Invalidate by VA to PoC），然后 `dsb ishst` + `isb` 确保所有 walker 可见。

## 3. 根因影响排序

| # | 根因 | 影响程度 | 说明 |
|---|------|----------|------|
| 1 | `msr sctlr_el1, xzr` 清零 RES1 位 | **致命** | 破坏 SCTLR 保留位，导致不可预测行为 |
| 2 | TCR 缺少 IPS 字段 | **致命** | 默认 IPS=0 意味着 32 位 PA，0x40000000 超出范围 |
| 3 | T0SZ 与页表层级不一致 | **致命** | 硬件从错误级别开始 walk 页表 |
| 4 | TCR 缺少 EPD1 | **严重** | TTBR1 未禁用，高位地址翻译走 TTBR1(=0) |
| 5 | Block 缺少 SH_IS | **严重** | 共享性配置与 TCR 不匹配 |
| 6 | SCTLR.WXN 未清除 | **严重** | RW 映射在 WXN=1 时不可执行 |
| 7 | 两阶段 SCTLR 启用 | **重要** | 避免缓存与 TLB 同时生效的竞争 |
| 8 | D-Cache civac | **防御性** | 确保页表对 walker 可见 |
| 9 | UXN/PXN/AP 属性 | **功能性** | 安全必需，不影响 Translation fault 本身 |
| 10 | SPSel / VBAR 顺序 | **可靠性** | 防止异常路径死循环 |

## 4. 验证

```bash
bash scripts/verify_kernel_qemu_virt_serial.sh
```

期望捕获中出现：`Hawthorn: hawthorn_kernel on QEMU virt OK`。

## 5. 相关源码

| 内容 | 路径 |
|------|------|
| 页表、TCR/MAIR、`enable_mmu_step4` / `step5` | `kernel/src/mm.rs` |
| `_start`、DAIF、`SPSel`、EL2 降 EL1 | `kernel/src/bin/qemu_virt.rs` |
| 调用顺序（含 `trap::init` 在 MMU 前） | `kernel/src/boot_qemu_virt.rs` |
| 串口验证脚本 | `scripts/verify_kernel_qemu_virt_serial.sh` |

## 6. 时间线

- **2025-04-26**：初版调试日志（分步与部分分析；AP 表述有误）。
- **2026-04-26**：按 `T0SZ`、异常路径、`SCTLR`/`WXN` 与 **aarch64_kernel** 对齐页表属性后修复；串口验证通过；本文合并为定稿说明。
