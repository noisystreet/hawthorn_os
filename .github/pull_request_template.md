## 摘要

<!-- 用一句话说明本 PR 做什么。合并时首条提交建议为「英文 Conventional + 单独一行中文」双语标题（见 docs/COMMIT_CONVENTIONS.md §1.0）。 -->

## 变更类型（可多选）

- [ ] `feat` — 新功能
- [ ] `fix` — 缺陷修复
- [ ] `docs` — 仅文档
- [ ] `refactor` / `perf` / `test` / `build` / `ci` / `chore` — 其他（见提交信息）

## 检查清单

- [ ] 已运行 **`cargo fmt --all -- --check`**
- [ ] 已运行 **`cargo clippy --workspace --all-targets -- -D warnings`**
- [ ] 若修改 **`docs/*.md`**，已同步更新 **`docs/en/`** 下**同名**英文稿（[双语规则](.cursor/rules/hawthorn-docs-bilingual.mdc)）
- [ ] 若本 PR 含新提交：已满足 **commit-msg** 要求（**第 1 行英文** Conventional + **第 2 行中文**对应、不同行；见 `docs/COMMIT_CONVENTIONS.md` §1.0）
- [ ] 若变更 **架构 / 内核行为 / syscall ABI / 启动契约**，已更新 **`docs/`** 中对应章节（如 `ARCHITECTURE.md`、`KERNEL.md`、`BOOT.md`、`SYSCALL_ABI.md`）

## 测试说明

<!-- 如何验证：命令、板卡、QEMU 等 -->

## 相关 Issue

<!-- 示例：Closes #123 -->
