#!/usr/bin/env bash
# 验证 hawthorn_qemu_minimal：Rust 检查 + 构建 + 短时启动 QEMU（确认脚本阶段有输出）。
# 不依赖串口上是否出现「Hawthorn」行（部分环境下 PL011→stdio 不可见）。
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
TARGET=aarch64-unknown-none

say() { printf '%s\n' "$*" >&2; }

say "[verify] cargo fmt --check"
cargo fmt --all -- --check

say "[verify] cargo clippy --workspace"
cargo clippy --workspace --all-targets -- -D warnings

say "[verify] cargo clippy hawthorn_qemu_minimal (${TARGET}, bare-metal)"
cargo clippy -p hawthorn_qemu_minimal --features bare-metal --target "${TARGET}" -- -D warnings

say "[verify] cargo check hawthorn_kernel (host + ${TARGET})"
cargo check -p hawthorn_kernel
cargo check -p hawthorn_kernel --target "${TARGET}"

say "[verify] cargo build hawthorn_qemu_minimal (bare-metal)"
cargo build -p hawthorn_qemu_minimal --features bare-metal --target "${TARGET}"

OUT="$(mktemp)"
say "[verify] 短时运行 scripts/run_qemu_minimal.sh（应出现多行 [hawthorn] …）"
set +e
timeout 6 bash "${ROOT}/scripts/run_qemu_minimal.sh" >"${OUT}" 2>&1
RC=$?
set -e
say "[verify] run_qemu_minimal 退出码: ${RC}（124=timeout 属正常）"

if ! grep -q '\[hawthorn\]' "${OUT}"; then
  say "[verify] 失败：未在输出中找到 [hawthorn] 前缀行。"
  cat "${OUT}" >&2 || true
  exit 1
fi

COUNT="$(grep -c '\[hawthorn\]' "${OUT}" || true)"
if [[ "${COUNT}" -lt 2 ]]; then
  say "[verify] 失败：[hawthorn] 行数过少 (${COUNT})。"
  cat "${OUT}" >&2 || true
  exit 1
fi

if grep -q 'Hawthorn: QEMU virt minimal OK' "${OUT}"; then
  say "[verify] 串口/stdio 上可见访客行 Hawthorn: QEMU virt minimal OK ✓"
else
  say "[verify] 提示：未在捕获输出中看到访客串口行（本环境常见）；Rust 与脚本阶段已通过。"
fi

say "[verify] 全部通过。"
rm -f "${OUT}"
