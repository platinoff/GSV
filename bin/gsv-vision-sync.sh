#!/usr/bin/env bash
# Thin wrapper: cargo run --bin gsv-vision-sync
# Usage: bash bin/gsv-vision-sync.sh [--check]
set -euo pipefail
export PATH="${HOME}/.cargo/bin:/ucrt64/bin:/usr/bin:${PATH:-}"
export RUSTUP_TOOLCHAIN="${RUSTUP_TOOLCHAIN:-stable-x86_64-pc-windows-gnu}"
unset CARGO_TARGET_DIR
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
exec cargo run -q --bin gsv-vision-sync -- "$@"
