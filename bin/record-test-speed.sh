#!/usr/bin/env bash
# Record wall-clock for `cargo test` into docs/vision/speed_index.json
# (Galaxy Speed Index panel + gsv-vision-sync mirror).
#
# Usage (MSYS2):
#   bash bin/record-test-speed.sh
#   bash bin/record-test-speed.sh --skip-run   # only print current index
#   HOST_LABEL=win10-local bash bin/record-test-speed.sh
set -euo pipefail

export PATH="${HOME}/.cargo/bin:/ucrt64/bin:/usr/bin:${PATH:-}"
export RUSTUP_TOOLCHAIN="${RUSTUP_TOOLCHAIN:-stable-x86_64-pc-windows-gnu}"
unset CARGO_TARGET_DIR

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

if [[ "${1:-}" == "--skip-run" ]]; then
  cargo run -q --bin gsv-speed-index -- --print
  exit 0
fi

HOST_LABEL="${HOST_LABEL:-${COMPUTERNAME:-${HOSTNAME:-local}}}"
CMD='cargo test'

echo "==> timing: $CMD (host=$HOST_LABEL)"
START=$(date +%s)
set +e
$CMD
STATUS=$?
set -e
END=$(date +%s)
WALL=$((END - START))

OK_FLAG=(--ok)
if [[ "$STATUS" -ne 0 ]]; then
  OK_FLAG=(--fail)
fi

cargo run -q --bin gsv-speed-index -- \
  --record-test \
  --wall-secs "$WALL" \
  "${OK_FLAG[@]}" \
  --command "$CMD" \
  --host "$HOST_LABEL"

echo "==> recorded wall_secs=${WALL} exit=${STATUS}"
exit "$STATUS"
