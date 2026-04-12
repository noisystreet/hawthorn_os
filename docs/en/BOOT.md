# Boot and boot info block (skeleton)

> **[中文](../BOOT.md)** — Chinese source of this document.

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

## Related documents

- [Architecture](./ARCHITECTURE.md)
- [Porting](./PORTING.md)
- [Microkernel design](./KERNEL.md)
