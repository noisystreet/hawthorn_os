# 山楂 / hawthorn 提交规范

> **[English](./en/COMMIT_CONVENTIONS.md)** — English mirror of this document.

本文档约定 **山楂（hawthorn）** 仓库中 **Git 提交信息**、**分支** 与 **合并请求（PR）** 的写法，便于自动生成变更日志、代码评审与问题追溯。

### 自动校验（pre-commit · commit-msg）

安装 [pre-commit](https://pre-commit.com/) 并执行 **`pre-commit install`** 后：

- **`pre-commit` 阶段**：`cargo fmt --check`、`cargo clippy -D warnings`（与 CI 一致）。  
- **`commit-msg` 阶段**：运行 **`scripts/commit_msg_bilingual.py`**，校验**双语标题**（见下文 **§1.0**）。

可选：`git config commit.template .gitmessage`，在编辑器中显示格式提示（`#` 行不会进入最终提交）。

---

## 1. 提交信息格式（Conventional Commits + 双语标题）

### 1.0 双语标题（强制）

- **第 1 条非空、非 `#` 注释行**：**英文**，须符合下方 **Conventional Commits** 的 `<type>(<scope>): <subject>`（`scope` 可省略）。**本行不得出现中文（CJK）**，即英文与中文**不得写在同一行**。  
- **第 2 条非空行**：**中文**，须与第 1 行**语义严格对应**（同一提交意图的简短表述），**单独占一行**；不得再使用英文的 `type(scope):` 式标题。  
- **第 3 行起**：空一行后可写正文、页脚（动机、**Closes**、**BREAKING CHANGE** 等）。

**示例（合法）：**

```
feat(kernel): add boot info block parser

添加启动信息块解析。

Further details in body…
```

**反例：**

- `feat(kernel): 添加解析` — 中英文混在同一行，**禁止**。  
- 只有一行英文、缺少中文对应行 — **禁止**（钩子会拒绝）。

**例外（钩子自动跳过）：** Git 默认 **`Merge …`** 行、以及 **`fixup!` / `squash!`** 交互变基行，不强制双语。

---

### 1.1 类型 `type`（常用）

| type | 含义 |
|------|------|
| `feat` | 新功能 |
| `fix` | 缺陷修复 |
| `docs` | 仅文档 |
| `style` | 不影响语义的格式（空格、分号等；非 CSS） |
| `refactor` | 重构（非 feat 也非 fix） |
| `perf` | 性能优化 |
| `test` | 测试相关 |
| `build` | 构建系统或依赖 |
| `ci` | CI 配置 |
| `chore` | 其他杂项（工具脚本等） |
| `revert` | 回滚某次提交（见 [Conventional Commits](https://www.conventionalcommits.org/zh-hans/v1.0.0/)） |

### 1.2 范围 `scope`（建议与目录对应）

可选；小写，简短，与仓库目录一致。示例：`kernel`、`hal`、`bsp`、`servers`（用户态驱动与服务）、`syscall_abi`、`middleware`、`examples`、`docs`、`ci`。若变更仅限某驱动 crate，可用 `servers` 或在正文中写明路径。

**英文标题行**示例：

- `feat(kernel): add priority ceiling mutex`
- `fix(hal): correct DMA alignment check`
- `docs: add CODE_STYLE.md`

### 1.3 英文标题行 `subject`（第 1 行）

- 使用祈使语气，**首字母小写**，行末 **不加句号**。  
- 尽量 **50 字符内**；细节放正文。  
- **仅英文**（与 §1.0 一致）。

### 1.4 中文标题行（第 2 行）

- **一句中文**，与第 1 行**含义一致**；可含必要英文缩写、路径名。  
- **单独一行**；**不要**再写 `feat(...):` 等英文前缀。

### 1.5 正文与页脚（第 3 行起）

- 与标题块之间 **空一行**；说明动机、实现要点、权衡。  
- **关闭议题**：`Closes #12` / `Fixes #34`。  
- **破坏性变更**：英文标题行可加 `!`，或页脚：

  ```
  BREAKING CHANGE: 描述迁移方式
  ```

---

## 2. 提交粒度

- **原子性**：一次提交解决一个逻辑单元；避免「一个大提交里混文档、格式化与功能」。  
- **格式化**：大范围 `cargo fmt` 可单独 `style:` 或 `chore:` 提交，减轻评审 diff 噪音。

---

## 3. 分支命名（建议）

| 前缀 | 用途 |
|------|------|
| `feature/` | 新功能 |
| `fix/` | 缺陷修复 |
| `docs/` | 文档 |
| `refactor/` | 重构 |
| `chore/` | 工具、依赖等 |

示例：`feature/kernel-sched-stats`、`fix/hal-uart-timeout`。

---

## 4. 合并请求（PR）

- **标题**：可与首条提交一致，或概括为同一 Conventional Commits 形式。  
- **描述**：背景、方案要点、**如何测试**（`cargo test`、板卡型号与步骤）；关联 issue / 设计文档链接。  
- **破坏性变更**：在描述顶部显著标明，并写清升级说明。

---

## 5. 版本与变更日志（可选）

- 若使用 [Release Please](https://github.com/googleapis/release-please)、`semantic-release` 或手工维护 `CHANGELOG.md`，**破坏性变更**与 **feat/fix** 的区分应与上述约定一致。

---

## 相关文档

- [架构说明](./ARCHITECTURE.md)
- [微内核设计](./KERNEL.md)
- [代码风格](./CODE_STYLE.md)
- [贡献指南](../CONTRIBUTING.md)
