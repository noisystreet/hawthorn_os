#!/usr/bin/env bash
# Verify hawthorn_kernel_qemu_virt serial output includes:
# 1) kernel banner
# 2) EL0 demo ("hello from EL0!")
# 3) EL1 bad-pointer SYS_write -> EFAULT (-14)
# 4) EL0 SYS_exit path: "[syscall] task N exit(0)"
# 5) user resource release after exit
#
# Stricter than verify_kernel_qemu_virt_serial.sh; failures name the missing pattern.
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

say "[hawthorn-el0] build hawthorn_kernel (${TARGET}, ${PROFILE})"
cargo build -p hawthorn_kernel --features bare-metal --target "${TARGET}" "${REL[@]}"

BIN="${ROOT}/target/${TARGET}/${PROFILE}/hawthorn_kernel_qemu_virt"
if [[ ! -f "${BIN}" ]]; then
  say "[hawthorn-el0] missing binary: ${BIN}"
  exit 1
fi

say "[hawthorn-el0] serial capture 127.0.0.1:${SERIAL_PORT} -> ${OUT}"
socat -lf "${SOCAT_LOG}" "TCP-LISTEN:${SERIAL_PORT},reuseaddr,fork" "OPEN:${OUT},append,ignoreeof,creat" &
SOCAT_PID=$!

sleep 0.3
if ! kill -0 "${SOCAT_PID}" 2>/dev/null; then
  say "[hawthorn-el0] socat failed to start; log:"
  cat "${SOCAT_LOG}" >&2 || true
  exit 1
fi

set +e
if ((${#STDBUF[@]} > 0)); then
  "${STDBUF[@]}" timeout 10 qemu-system-aarch64 \
    -machine virt,gic-version=3 \
    -cpu cortex-a76 \
    -display none \
    -kernel "${BIN}" \
    -chardev "socket,id=sock,host=127.0.0.1,port=${SERIAL_PORT},nodelay=on,server=off" \
    -serial chardev:sock
else
  timeout 10 qemu-system-aarch64 \
    -machine virt,gic-version=3 \
    -cpu cortex-a76 \
    -display none \
    -kernel "${BIN}" \
    -chardev "socket,id=sock,host=127.0.0.1,port=${SERIAL_PORT},nodelay=on,server=off" \
    -serial chardev:sock
fi
QEMU_RC=$?
set -e

kill "${SOCAT_PID}" 2>/dev/null || true
wait "${SOCAT_PID}" 2>/dev/null || true

if [[ "${QEMU_RC}" -ne 0 ]] && [[ "${QEMU_RC}" -ne 124 ]]; then
  say "[hawthorn-el0] qemu exit ${QEMU_RC}"
  exit "${QEMU_RC}"
fi

if ! rg -q 'Hawthorn: hawthorn_kernel on QEMU virt OK' "${OUT}"; then
  say "[hawthorn-el0] FAIL: missing kernel banner"
  say "[hawthorn-el0] captured text:"
  cat "${OUT}" >&2 || true
  exit 1
fi

if ! rg -q 'hello from EL0!' "${OUT}"; then
  say "[hawthorn-el0] FAIL: missing EL0 demo output"
  say "[hawthorn-el0] captured text:"
  cat "${OUT}" >&2 || true
  exit 1
fi

if ! rg -q '\[task D\] SYS_write returned 22' "${OUT}"; then
  say "[hawthorn-el0] FAIL: EL1 kernel-buffer SYS_write should return full length (22)"
  say "[hawthorn-el0] captured text:"
  cat "${OUT}" >&2 || true
  exit 1
fi

if ! rg -q '\[task D\] SYS_write\(bad ptr\) returned -14 \(expect -14 EFAULT\)' "${OUT}"; then
  say "[hawthorn-el0] FAIL: bad-pointer SYS_write did not return EFAULT (-14)"
  say "[hawthorn-el0] captured text:"
  cat "${OUT}" >&2 || true
  exit 1
fi

if ! rg -q '\[task E\] endpoint_create returned [0-9]+' "${OUT}"; then
  say "[hawthorn-el0] FAIL: missing task E endpoint_create success output"
  say "[hawthorn-el0] captured text:"
  cat "${OUT}" >&2 || true
  exit 1
fi

if ! rg -q '\[task E\] endpoint_recv got client=[0-9]+ msg=42' "${OUT}"; then
  say "[hawthorn-el0] FAIL: missing endpoint_recv rendezvous output (expect msg=42)"
  say "[hawthorn-el0] captured text:"
  cat "${OUT}" >&2 || true
  exit 1
fi

if ! rg -q '\[task E\] endpoint_reply returned 0' "${OUT}"; then
  say "[hawthorn-el0] FAIL: endpoint_reply should succeed with 0"
  say "[hawthorn-el0] captured text:"
  cat "${OUT}" >&2 || true
  exit 1
fi

if ! rg -q '\[task D\] endpoint_call returned 43 \(expect 43\)' "${OUT}"; then
  say "[hawthorn-el0] FAIL: endpoint_call should return reply 43"
  say "[hawthorn-el0] captured text:"
  cat "${OUT}" >&2 || true
  exit 1
fi

if ! rg -q '\[task E\] endpoint_destroy returned 0' "${OUT}"; then
  say "[hawthorn-el0] FAIL: endpoint_destroy should succeed with 0"
  say "[hawthorn-el0] captured text:"
  cat "${OUT}" >&2 || true
  exit 1
fi

if ! rg -q '\[syscall\] task [0-9]+ exit\(0\)' "${OUT}"; then
  say "[hawthorn-el0] FAIL: missing EL0 sys_exit(0) log (expect [syscall] task N exit(0))"
  say "[hawthorn-el0] captured text:"
  cat "${OUT}" >&2 || true
  exit 1
fi

if ! rg -q '\[task\] released user resources:' "${OUT}"; then
  say "[hawthorn-el0] FAIL: missing user resource release output"
  say "[hawthorn-el0] captured text:"
  cat "${OUT}" >&2 || true
  exit 1
fi

say "[hawthorn-el0] OK: banner + EL0 hello + EFAULT bad-ptr + exit(0) + resource release."
exit 0
