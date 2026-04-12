# 启动与启动信息块（骨架）

> **[English](./en/BOOT.md)** — English mirror of this document.

本文档约定 **Bootloader → 山楂（hawthorn）内核** 之间的二进制契约；具体地址与 Rockchip 流程见 [PORTING.md](./PORTING.md) 与 BSP。

---

## 1. 启动信息块（Boot Info Block）

内核入口在 **关中断、极短路径** 内读取一块 **版本化** 内存结构（魔数 + `layout_version`），建议字段方向如下（实现时以头文件 / `syscall_abi` 旁路共享类型为准）：

| 字段方向 | 说明 |
|----------|------|
| 魔数 / 版本 | 校验引导与内核是否匹配 |
| 物理内存范围 | 可用 RAM 区间（或由 U-Boot/FDT 解析后填入） |
| FDT 指针 | 若使用设备树：物理地址与大小；若不使用则为 0 |
| 保留区 / 帧缓冲等 | 避免内核与用户态堆叠使用 |
| 启动槽位 / OTA 元数据 | 可选，与 M3 安全启动协同 |

**状态**：字段表与 ABI 尚未冻结；冻结后在本文件与 `bsp/orangepi5-rk3588/` 同步更新。

---

## 2. 引导阶段（与 KERNEL 文档对齐）

1. **极早期**：栈、BSS、CPU 特性、**MMU（及平台若有的 MPU）** 最小配置。  
2. **内核线程就绪**：建立根能力空间、首个可调度上下文。  
3. **根用户任务**：创建 **init**，由其按能力拉起驱动与其它服务。

详见 [KERNEL.md §3.1](./KERNEL.md)。

---

## 相关文档

- [架构说明](./ARCHITECTURE.md)
- [移植指南](./PORTING.md)
- [微内核设计](./KERNEL.md)
