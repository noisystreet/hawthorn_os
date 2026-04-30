# Boot and boot info block (skeleton)

> **[中文](../引导.md)** — Chinese source of this document.

Binary contract between **Bootloader → Hawthorn (山楂) kernel**; addresses and Rockchip flow: [PORTING.md](./PORTING.md) and BSP.

---

## 1. Boot info block

The kernel entry (short, interrupts-off path) reads a **versioned** memory block (magic + `layout_version`). Suggested fields (final types in headers / shared with `syscall_abi` when frozen):

| Field direction | Description |
|-----------------|-------------|
| Magic / version | Validate loader vs kernel match |
| Physical RAM ranges | Usable RAM (or filled after U-Boot/FDT parse) |
| FDT pointer | If DT used: physical address and size; else 0 |
| Reserved / framebuffer | Avoid overlap with kernel / user |
| Boot slot / OTA metadata | Optional; aligns with M3 secure boot |

**Status:** fields and ABI not frozen; update here and under `bsp/orangepi5-rk3588/` when fixed.

---

## 2. Boot phases (aligned with KERNEL doc)

1. **Earliest:** stack, BSS, CPU features, minimal **MMU** (and **MPU** if present).  
2. **Kernel threads ready:** root capability space, first schedulable context.  
3. **Root user task:** create **init**, which starts drivers and other services per capabilities.

See [KERNEL.md §3.1](./KERNEL.md).

---

## 3. QEMU `virt` minimal entry (M1, `hawthorn_kernel`)

Unlike the long-term **boot info block** ABI (§1), the table below is the **currently implemented** QEMU `virt` AArch64 smoke contract:

| Item | Contract |
|------|------------|
| Linker script | [`kernel/link-qemu_virt.ld`](../../kernel/link-qemu_virt.ld) (shared with `hawthorn_qemu_minimal`) |
| Entry symbol | **`_start`** (`ENTRY(_start)`) |
| Initial stack | **`SP = __stack_top`** (16-byte aligned word below RAM end) |
| Rust entry | **`kernel_main`** (`extern "C"`, `hawthorn_kernel::boot_qemu_virt`) |
| Early steps | Zero BSS (`__bss_start`…`__bss_end`) → PL011 init |
| Debug UART | **PL011**, physical base **`0x9000_0000`** |
| Bare-metal bin | **`hawthorn_kernel_qemu_virt`**, needs **`--features bare-metal`** + **`--target aarch64-unknown-none`** |

---

## Related documents

- [Architecture](./ARCHITECTURE.md)
- [Porting](./PORTING.md)
- [Microkernel design](./KERNEL.md)
