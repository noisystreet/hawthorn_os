#!/usr/bin/env bash
# Build and run hawthorn_qemu_minimal on QEMU virt (PL011 @ 0x9000000).
# Requires: qemu-system-aarch64, Rust target aarch64-unknown-none.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
PROFILE="${PROFILE:-debug}"
REL=()
if [[ "${PROFILE}" == "release" ]]; then
  REL=(--release)
fi
TARGET=aarch64-unknown-none

say() {
  printf '%s\n' "$*" >&2
}

say "[hawthorn] 正在编译 hawthorn_qemu_minimal（target=${TARGET}, profile=${PROFILE}）…"
cargo build -p hawthorn_qemu_minimal --features bare-metal --target "${TARGET}" "${REL[@]}"
BIN="${ROOT}/target/${TARGET}/${PROFILE}/hawthorn_qemu_minimal"
say "[hawthorn] 编译完成: ${BIN}"
say "[hawthorn] 正在启动 QEMU（virt + PL011 → 本终端；-nographic 将串口接到 stdio）。"
say "[hawthorn] 若一行日志也没有，仍可能已在跑；按 Ctrl+C 结束。"
if [[ ! -t 1 ]]; then
  say "[hawthorn] 提示：stdout 不是终端，部分环境下可能看不到访客串口；请在交互式终端里重试。"
fi
say ""

# -nographic：串口接到控制台（勿再写 -serial stdio，否则易与 monitor 争用 stdio）。
# stdbuf：尽量行缓冲，便于立刻看到 PL011 输出（若环境有 stdbuf）。
if command -v stdbuf >/dev/null 2>&1; then
  exec stdbuf -oL -eL qemu-system-aarch64 \
    -machine virt,gic-version=3 \
    -cpu cortex-a76 \
    -nographic \
    -kernel "${BIN}"
else
  exec qemu-system-aarch64 \
    -machine virt,gic-version=3 \
    -cpu cortex-a76 \
    -nographic \
    -kernel "${BIN}"
fi
