#!/usr/bin/env bash
# Scan Clippy JSON messages → docs/vision/rust_diagnostics.json
# (Galaxy Rust Diagnostics panel + gsv-vision-sync mirror).
#
# Usage (MSYS2):
#   bash bin/record-rust-diagnostics.sh
#   bash bin/record-rust-diagnostics.sh --skip-run
#   bash bin/record-rust-diagnostics.sh --ci
#   HOST_LABEL=win10-local bash bin/record-rust-diagnostics.sh
set -euo pipefail

export PATH="${HOME}/.cargo/bin:/ucrt64/bin:/usr/bin:${PATH:-}"
case "$(uname -s)" in
  MINGW*|MSYS*|CYGWIN*)
    export RUSTUP_TOOLCHAIN="${RUSTUP_TOOLCHAIN:-stable-x86_64-pc-windows-gnu}"
    unset CARGO_TARGET_DIR
    ;;
  *) ;;
esac

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

SOURCE="local"
SKIP=0
for arg in "$@"; do
  case "$arg" in
    --skip-run) SKIP=1 ;;
    --ci) SOURCE="ci" ;;
    -h|--help)
      echo "Usage: bash bin/record-rust-diagnostics.sh [--skip-run] [--ci]"
      exit 0
      ;;
  esac
done

if [[ "$SKIP" -eq 1 ]]; then
  cargo run -q --bin gsv-rust-diagnostics -- --print
  exit 0
fi

HOST_LABEL="${HOST_LABEL:-${COMPUTERNAME:-${HOSTNAME:-local}}}"
CMD="${RUST_DIAGNOSTICS_CMD:-cargo clippy --message-format=json --all-targets}"

echo "==> rust diagnostics scan (source=$SOURCE host=$HOST_LABEL)"
echo "==> $CMD"

set +e
cargo run -q --bin gsv-rust-diagnostics -- \
  --scan \
  --command "$CMD" \
  --host "$HOST_LABEL" \
  --source "$SOURCE"
STATUS=$?
set -e

echo "==> recorded (exit=$STATUS) — vision panel: Rust"
exit "$STATUS"
