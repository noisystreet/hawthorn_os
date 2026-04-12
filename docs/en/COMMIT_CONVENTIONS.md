# Hawthorn / 山楂 — Commit conventions

> **[中文](../COMMIT_CONVENTIONS.md)** — Chinese source of this document.

Git **commit messages**, **branches**, and **pull requests** for the **Hawthorn (山楂)** repo.

### Automated checks (pre-commit · commit-msg)

After **`pre-commit install`**:

- **`pre-commit` stage:** `cargo fmt --check`, `cargo clippy --workspace -D warnings` (same as CI).  
- **`commit-msg` stage:** **`scripts/commit_msg_bilingual.py`** enforces the **bilingual subject** rules in **§1.0** below.

Optional: `git config commit.template .gitmessage`.

---

## 1. Conventional Commits + bilingual subject

### 1.0 Bilingual subject (required)

- **First non-empty, non-`#` line:** **English only**, must match **Conventional Commits** `<type>(<scope>): <subject>` (`scope` optional). **No Chinese (CJK) on this line** — English and Chinese **must not share one line**.  
- **Second non-empty line:** **Chinese**, **strictly the same meaning** as line 1, on its **own line**; do **not** repeat an English `type(scope):` header.  
- **From line 3:** blank line, then body / footers (`Closes`, `BREAKING CHANGE`, …).

**Valid example:**

```
feat(kernel): add boot info block parser

添加启动信息块解析。

More detail in the body…
```

**Invalid:**

- `feat(kernel): 添加解析` — CJK on the same line as English header.  
- Only one English line — missing Chinese pair.

**Exceptions (hook skips):** Git default **`Merge …`**, and **`fixup!` / `squash!`** lines.

---

### 1.1 `type`

| type | Meaning |
|------|---------|
| `feat` | New feature |
| `fix` | Bug fix |
| `docs` | Documentation only |
| `style` | Formatting (not CSS) |
| `refactor` | Refactor (not feat/fix) |
| `perf` | Performance |
| `test` | Tests |
| `build` | Build / deps |
| `ci` | CI config |
| `chore` | Misc |
| `revert` | Revert a prior commit |

### 1.2 `scope`

Optional; lowercase; match repo dirs: `kernel`, `hal`, `bsp`, `servers`, `syscall_abi`, `middleware`, `examples`, `docs`, `ci`.

**English line 1** examples:

- `feat(kernel): add priority ceiling mutex`
- `fix(hal): correct DMA alignment check`
- `docs: add CODE_STYLE.md`

### 1.3 English subject (line 1)

Imperative, **lowercase** first word, **no** trailing period; prefer **≤50 chars**; **English only**.

### 1.4 Chinese subject (line 2)

One **Chinese** sentence matching line 1; may include necessary English acronyms/paths; **no** second `feat(...):` prefix.

### 1.5 Body & footer (from line 3)

Blank line after the two-line subject; then body and footers as usual.

---

## 2. Commit granularity

**Atomic** changes; separate large `cargo fmt` as `style:` / `chore:`.

---

## 3. Branch names (suggested)

`feature/`, `fix/`, `docs/`, `refactor/`, `chore/` — see Chinese doc for examples.

---

## 4. Pull requests

Title, body, testing, breaking-change callouts — same as common practice.

---

## 5. Versioning / changelog (optional)

Align with changelog tooling if used.

---

## Related documents

- [Architecture](./ARCHITECTURE.md)
- [Microkernel design](./KERNEL.md)
- [Code style](./CODE_STYLE.md)
- [Contributing](../../CONTRIBUTING.md)
