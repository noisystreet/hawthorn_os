#!/usr/bin/env bash
# Capture hawthorn_kernel_qemu_virt PL011 output without a TTY (CI / pipes).
# Uses: socat TCP listener -> file, QEMU chardev socket (client) -> serial, stdbuf on QEMU.
# Requires: qemu-system-aarch64, cargo, Rust target aarch64-unknown-none, socat.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
TARGET=aarch64-unknown-none
PROFILE="${PROFILE:-debug}"
REL=()
if [[ "${PROFILE}" == "release" ]]; then
  REL=(--release)
fi

STDBUF=()
if [[ -x /usr/bin/stdbuf ]]; then
  STDBUF=(/usr/bin/stdbuf -oL -eL)
elif command -v stdbuf >/dev/null 2>&1; then
  STDBUF=(stdbuf -oL -eL)
fi

say() {
  printf '%s\n' "$*" >&2
}

# Do not use env name `PORT` — many hosts/sandboxes clear or override it.
SERIAL_PORT="${HAWTHORN_QEMU_SERIAL_PORT:-0}"
if [[ "${SERIAL_PORT}" -eq 0 ]]; then
  SERIAL_PORT=$((10000 + (RANDOM % 50000)))
fi

OUT="$(mktemp)"
SOCAT_LOG="$(mktemp)"
cleanup() {
  rm -f "${OUT}" "${SOCAT_LOG}" 2>/dev/null || true
}
trap cleanup EXIT

say "[hawthorn] build hawthorn_kernel (bare-metal, ${TARGET}, ${PROFILE})"
cargo build -p hawthorn_kernel --features bare-metal --target "${TARGET}" "${REL[@]}"

BIN="${ROOT}/target/${TARGET}/${PROFILE}/hawthorn_kernel_qemu_virt"
if [[ ! -f "${BIN}" ]]; then
  say "[hawthorn] missing binary: ${BIN}"
  exit 1
fi

say "[hawthorn] serial capture on 127.0.0.1:${SERIAL_PORT} -> ${OUT} (socat) + stdbuf qemu"

# socat listens; QEMU connects as client (server=off) so the guest starts immediately.
socat -lf "${SOCAT_LOG}" "TCP-LISTEN:${SERIAL_PORT},reuseaddr,fork" "OPEN:${OUT},append,ignoreeof,creat" &
SOCAT_PID=$!

sleep 0.3
if ! kill -0 "${SOCAT_PID}" 2>/dev/null; then
  say "[hawthorn] socat failed to start; log:"
  cat "${SOCAT_LOG}" >&2 || true
  exit 1
fi

set +e
if ((${#STDBUF[@]} > 0)); then
  "${STDBUF[@]}" timeout 8 qemu-system-aarch64 \
    -machine virt \
    -cpu cortex-a76 \
    -display none \
    -kernel "${BIN}" \
    -chardev "socket,id=ser,host=127.0.0.1,port=${SERIAL_PORT},nodelay=on,server=off" \
    -serial chardev:ser
else
  timeout 8 qemu-system-aarch64 \
    -machine virt \
    -cpu cortex-a76 \
    -display none \
    -kernel "${BIN}" \
    -chardev "socket,id=ser,host=127.0.0.1,port=${SERIAL_PORT},nodelay=on,server=off" \
    -serial chardev:ser
fi
QEMU_RC=$?
set -e

kill "${SOCAT_PID}" 2>/dev/null || true
wait "${SOCAT_PID}" 2>/dev/null || true

if [[ "${QEMU_RC}" -ne 0 ]] && [[ "${QEMU_RC}" -ne 124 ]]; then
  say "[hawthorn] qemu exit ${QEMU_RC}"
  exit "${QEMU_RC}"
fi

if ! grep -q 'Hawthorn: hawthorn_kernel on QEMU virt OK' "${OUT}"; then
  say "[hawthorn] FAIL: expected PL011 line not in ${OUT}"
  say "[hawthorn] socat log (${SOCAT_LOG}):"
  cat "${SOCAT_LOG}" >&2 || true
  say "[hawthorn] captured bytes (hex):"
  xxd "${OUT}" >&2 || true
  say "[hawthorn] captured text:"
  cat "${OUT}" >&2 || true
  exit 1
fi

say "[hawthorn] OK: PL011 banner found in capture."
exit 0
