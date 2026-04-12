# Supported platforms and tiers

> **[中文](../PLATFORMS.md)** — Chinese source of this document.

**Hawthorn (山楂)** uses hardware **tiers** by maturity. Current plan:

## Tier 1 (first bring-up)

| Board | SoC | Arch / environment | Notes |
|-------|-----|---------------------|--------|
| [Orange Pi 5](https://www.orangepi.org/html/hardWare/computerAndMicrocontrollers/details/Orange-Pi-5.html) | RK3588 | AArch64, EL1 kernel + EL0 user (target) | BSP path: `bsp/orangepi5-rk3588/` |

## Tier 2 / 3 (planned)

Long-term product spectrum; **not** promised on the same schedule as Tier 1. Names and paths TBD.

| Direction | Notes |
|-----------|--------|
| Other AArch64 SBCs / SoCs | Port per board once HAL / microkernel boundaries are clear |
| Cortex-M / RISC-V MCUs | Large divergence from RK3588 **MMU + multi-cluster GIC** path; separate porting line |

Vision for “MCU through MPU spectrum”: [ARCHITECTURE.md §1.1](./ARCHITECTURE.md).
