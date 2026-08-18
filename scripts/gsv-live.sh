#!/usr/bin/env bash
# GSV always-on supervisor: run a *copy* of the debug exe so `cargo test` /
# `cargo build` may overwrite `target/debug/gsv-server.exe` without locking
# the process on :9999. After `POST /api/update/apply` the process exits;
# this loop recopies debug → live and rebinds.
#
# Usage (MSYS2 bash):
#   cargo build --bin gsv-server
#   bash scripts/gsv-live.sh
#
# Env:
#   GSV_HOST  default 127.0.0.1
#   GSV_PORT  default 9999
#
# Do **not** kill this live copy before `cargo test`. Only stop `target/debug/`
# if that file is the listener (pre-band-144).

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEBUG="$ROOT/target/debug/gsv-server.exe"
LIVE_DIR="$ROOT/target/live"
LIVE="$LIVE_DIR/gsv-server.exe"
HOST="${GSV_HOST:-127.0.0.1}"
PORT="${GSV_PORT:-9999}"
mkdir -p "$LIVE_DIR"
copy_live() {
  cp -f "$DEBUG" "$LIVE"
}
if [ ! -f "$DEBUG" ]; then
  echo "build debug first: cargo build --bin gsv-server" >&2
  exit 1
fi
copy_live
while true; do
  copy_live
  "$LIVE" --host "$HOST" --port "$PORT" || true
  echo "gsv-live: process exited, restarting in 1s" >&2
  sleep 1
done
