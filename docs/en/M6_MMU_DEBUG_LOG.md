# M6 QEMU `virt` MMU bring-up — debug log and fix

> **[中文](../M6_MMU_DEBUG_LOG.md)** — Chinese source (`docs/M6_MMU_DEBUG_LOG.md`)

This document records symptoms, investigation, and the **resolved** AArch64 MMU bring-up for Hawthorn on QEMU `virt` (`scripts/verify_kernel_qemu_virt_serial.sh` passes).

## 1. Symptoms

- `bash scripts/verify_kernel_qemu_virt_serial.sh` times out; serial never shows  
  `Hawthorn: hawthorn_kernel on QEMU virt OK`.
- Logs usually reach **`[mm/step4] ... MMU NOT enabled yet`**, then stall inside **`enable_mmu_step5`** (writing `SCTLR_EL1` to set **M**).
- With QEMU tracing, **Prefetch Abort / translation faults** may appear depending on the broken configuration.

## 2. Root causes (by importance)

### 2.1 `T0SZ` vs page-table walk level

- With **4 KiB granules**, the **starting lookup level** follows **`TCR_EL1.T0SZ`**.
- **`VA_BITS_T0 = 39`** (`T0SZ = 25`) makes the MMU start at **level 1**, while this kernel builds a **four-level, L0-rooted** tree. **TTBR0** then points at a table interpreted as **L1**, so walks are wrong.
- **Fix:** **`VA_BITS_T0 = 48`** (`T0SZ = 16`) so the walk starts at **L0**, matching `map_block_2m` / `map_page`.

### 2.2 PTE AP / UXN / PXN / table attributes (vs reference kernel)

Compared with **`aarch64_kernel`** (`kernel/mm/bits.rs`, `identity.rs`):

| Item | Issue | Alignment |
|------|--------|-----------|
| **AP** | **`0b01 << 6` (EL1+EL0 RW)** differed from **`DESC_AP_RW_EL1` (`0b00`, EL1 RW only, EL0 no access)** | Kernel RAM / low IO: **`PTE_AP_RW_EL1`**; user pages later: **`PTE_AP_RW_ALL`** |
| **Execute** | Missing common idmap **UXN/PXN** | Normal RAM: **`PTE_UXN`**; device: **`PTE_UXN | PTE_PXN`** |
| **Table desc** | Only `valid \| table` | Add **`TABLE_UXNTABLE | TABLE_APTABLE0`** (same idea as reference L0) |

> **Erratum (older draft):** AP=00 was wrongly described as “EL1 read-only”. In AArch64 VMSA, **`AP[2:1]=00`** is **EL1 read/write, EL0 no access**; **`01`** is **EL1+EL0 read/write**. Trust the Arm ARM and the reference code.

### 2.3 `SCTLR_EL1.WXN` with an RW identity map

- If **`SCTLR_EL1.WXN = 1`**, writable regions are treated as **execute-never**; with an early **RW** RAM map, instruction fetch can fail after **M** is enabled.
- **Fix:** When turning **M** on, **clear WXN** (merge with a read of `SCTLR_EL1`, not a blind OR mask that ignores RES1).

### 2.4 Boot and exception path

- **`VBAR_EL1`:** must be installed **before** enabling the MMU.
- **`SPSel`:** if **0**, EL1 uses **SP_EL0** and exceptions use VBAR slots **0x0–0x180**; a **`generic_stub` that is `b .`** **spins silently**. **`msr spsel, #1`** in `_start`.
- **EL2 → EL1:** do **not** **`msr sctlr_el1, xzr`** (breaks **RES1**); at EL1, **read/modify/write** to clear **M/C/I** only.
- **IRQ:** if the MMU is on but **GIC is not inited**, **`irq::dispatch` → `ICC_IAR1_EL1`** can misbehave. Match reference **`head.S`:** **`msr DAIFSet, #0xf`** at entry; keep the same idea before MMU in `enable_mmu_step5`.

### 2.5 `SCTLR` and caches

- Reference **`aarch64_kernel`** `enable_mmu()` only **ORs M**.
- This repo’s **`_start`** clears **C/I**, so after **M** plus **`ic iallu` / `dsb` / `isb`**, a **second `SCTLR` write** turns **C** and **I** on.

## 3. Verification

```bash
bash scripts/verify_kernel_qemu_virt_serial.sh
```

Expect the capture to contain: `Hawthorn: hawthorn_kernel on QEMU virt OK`.

## 4. Related source

| Topic | Path |
|--------|------|
| Page tables, TCR/MAIR, `enable_mmu_step4` / `step5` | `kernel/src/mm.rs` |
| `_start`, DAIF, `SPSel`, EL2 drop | `kernel/src/bin/qemu_virt.rs` |
| Call order (including `trap::init` before MMU) | `kernel/src/boot_qemu_virt.rs` |
| Serial check script | `scripts/verify_kernel_qemu_virt_serial.sh` |

## 5. Timeline

- **2025-04-26:** First debug notes (step breakdown; some AP text was wrong).
- **2026-04-26:** Fixed with `T0SZ`/exception path/`SCTLR`+**WXN** and **aarch64_kernel**-style PTE/table attrs; serial verify passes; this page is the consolidated record.
