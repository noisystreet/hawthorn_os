# PR 与 Issue 编排（当前里程碑）

> **[English](./en/PR_ISSUE_PLAN.md)** — English mirror of this document.

本文档把 **GitHub Issue** 与建议的 **PR 顺序** 固定下来，便于开分支、写 PR 描述时引用 `Closes #…` / `Refs #…`。能力级 backlog 仍以 [TODO.md](./TODO.md) 为准。

---

## 当前跟踪：QEMU `virt` 上 `hawthorn_kernel` 最小可运行路径

| 角色 | 链接 |
|------|------|
| **Meta（总览）** | <https://github.com/noisystreet/hawthorn_os/issues/5> |

### Issue 列表（按建议实现顺序）

| 顺序 | Issue | 标题（摘要） |
|------|--------|----------------|
| 1 | [#1](https://github.com/noisystreet/hawthorn_os/issues/1) | M1：`hawthorn_kernel` 最小引导（QEMU virt）+ PL011 panic |
| 2 | [#2](https://github.com/noisystreet/hawthorn_os/issues/2) | M1b：`qemu_minimal` 经 `hawthorn_kernel` 公开 API 启动 |
| 3 | [#3](https://github.com/noisystreet/hawthorn_os/issues/3) | M2：`VBAR_EL1` 异常向量与 sync/IRQ 桩 |
| 4 | [#4](https://github.com/noisystreet/hawthorn_os/issues/4) | M3：协作式调度 MVP（TCB / 就绪队列 / yield） |

**建议 PR 顺序：** `#1 → #2 → #3 → #4`。其中 **#3（M2 向量表）** 在 M1 的入口与符号稳定后，可与 **#2（M1b）** 并行开发，合并时注意冲突（向量表 vs qemu 联动以先合并者为准，后者变基）。

---

## PR 开法约定

1. **一个 PR 尽量对应一个 issue**；大改可拆 PR，但每个 PR 仍应 `Closes #n` 或 `Refs #n`。
2. PR 描述使用仓库模板 [.github/pull_request_template.md](../.github/pull_request_template.md)，在 **相关 Issue** 填写例如：`Closes #1`。
3. **提交信息**：`docs/COMMIT_CONVENTIONS.md` — 第 1 行英文 Conventional Commits，第 2 行中文对应。
4. **标签**：内核相关 issue 已使用 `kernel` + `enhancement`；新 issue 标题建议继续带 **`[kernel]`**、`[IPC]` 等前缀（与 [TODO.md](./TODO.md) 说明一致）。

---

## 本地验证（与 CI / AGENTS 对齐）

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo check -p hawthorn_kernel
cargo check -p hawthorn_kernel --target aarch64-unknown-none
cargo build -p hawthorn_qemu_minimal --features bare-metal --target aarch64-unknown-none
```

M1 合并后若引入新 crate feature 或新 bin，请在本文件与 **相关 issue 验收标准** 中同步更新命令。

---

## 后续（尚未建 issue）

下一批可在 TODO 中勾选并 **另开 issue** 的条目示例：`syscall_abi` crate、SVC 统一分发、最小 IPC（短消息）。Meta issue **#5** 关闭后，可新建 `[meta]` issue 跟踪下一阶段。
