# Glossary

> **[中文](../GLOSSARY.md)** — Chinese source of this document.

| Term | Short definition |
|------|-------------------|
| **山楂 (Shānzhā)** | **Chinese name** of this OS. English code name **hawthorn**; main kernel crate **`hawthorn_kernel`**. You may write “山楂（hawthorn）”. |
| **hawthorn** | English project code name; repo directory and remote name are recommended to match (custom names OK). |
| **Microkernel** | Minimal mechanism: scheduling, IPC, address spaces & mapping, capabilities, interrupt delivery; drivers and stacks in user services. |
| **User service** | Process/task at **EL0** (or equivalent), talking to the kernel via **syscalls**; code under `servers/` (planned). |
| **Capability** | Unforgeable kernel reference to an object or resource; derivable and revocable. See [KERNEL.md](./KERNEL.md). |
| **IPC** | Inter-process communication; Hawthorn uses a **synchronous message** minimal set. See KERNEL doc. |
| **Endpoint / Port** | IPC endpoint object, referenced by capabilities. |
| **TCB** | Trusted Computing Base — the minimal audited kernel code. |
| **RK3588** | Rockchip SoC, AArch64, big.LITTLE; Tier-1 bring-up hardware. |
| **EL0 / EL1** | ARMv8 exception levels: EL0 user, EL1 typically kernel. |
| **GIC** | Generic Interrupt Controller; exact variant on RK3588 per **TRM**. |
| **FDT / DTB** | Flattened Device Tree / blob; adoption and subset scope: [ARCHITECTURE.md §10](./ARCHITECTURE.md). |
| **TF-A** | Trusted Firmware-A; **EL handoff** with bootloader and kernel. |
