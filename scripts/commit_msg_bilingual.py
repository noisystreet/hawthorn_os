#!/usr/bin/env python3
"""
commit-msg hook: bilingual subject for Hawthorn (山楂).

- First non-empty, non-comment line: Conventional Commits in English only (no CJK).
- Second non-empty line: Chinese summary, must contain CJK, same meaning as line 1;
  must not duplicate an English conventional header.
- English and Chinese must not appear on the same line (line 1 must have no CJK).

Skips: Merge …, fixup! …, squash! …
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

TYPES = "feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert"
LINE1 = re.compile(rf"^({TYPES})(\([^\)]+\))?!?:\s+\S.+$")
EN_HEADER = re.compile(rf"^({TYPES})(\([^\)]+\))?!?:\s*\S")


def has_cjk(s: str) -> bool:
    return any(
        "\u4e00" <= c <= "\u9fff" or "\u3400" <= c <= "\u4dbf" for c in s
    )


def meaningful_lines(path: Path) -> list[str]:
    out: list[str] = []
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        s = line.strip()
        if not s or s.startswith("#"):
            continue
        out.append(s)
    return out


def main() -> int:
    if len(sys.argv) < 2:
        print("commit-msg: missing COMMIT_EDITMSG path", file=sys.stderr)
        return 1
    path = Path(sys.argv[1])
    if not path.is_file():
        print(f"commit-msg: not a file: {path}", file=sys.stderr)
        return 1

    lines = meaningful_lines(path)
    if not lines:
        print("commit-msg: empty message", file=sys.stderr)
        return 1

    if lines[0].startswith("Merge "):
        return 0
    if re.match(r"^(fixup|squash)!", lines[0]):
        return 0

    if has_cjk(lines[0]):
        print(
            "commit-msg: 第 1 行须为英文 Conventional Commits 标题，且不得与中文写在同一行。\n"
            "commit-msg: Line 1 must be English-only (Conventional Commits); "
            "do not put Chinese on the same line.",
            file=sys.stderr,
        )
        return 1

    if not LINE1.match(lines[0]):
        print(
            "commit-msg: 第 1 行格式应为 <type>(<scope>): <subject> 或 <type>: <subject>（英文）。\n"
            "commit-msg: Line 1 must match Conventional Commits, e.g. feat(kernel): add foo",
            file=sys.stderr,
        )
        return 1

    if len(lines) < 2:
        print(
            "commit-msg: 第 2 行须为与第 1 行语义对应的中文说明（单独一行，勿与英文同行）。\n"
            "commit-msg: Line 2 must be a Chinese subject line (same meaning as line 1); "
            "EN and ZH must not share one line.",
            file=sys.stderr,
        )
        return 1

    if not has_cjk(lines[1]):
        print(
            "commit-msg: 第 2 行须含中文（CJK），作为与英文标题对应的中文说明。\n"
            "commit-msg: Line 2 must contain Chinese (CJK) as the paired summary.",
            file=sys.stderr,
        )
        return 1

    if EN_HEADER.match(lines[1]):
        print(
            "commit-msg: 第 2 行应为中文说明，不要再次使用英文 type(scope): 式标题。\n"
            "commit-msg: Line 2 should be Chinese text only, not a second English conventional header.",
            file=sys.stderr,
        )
        return 1

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
