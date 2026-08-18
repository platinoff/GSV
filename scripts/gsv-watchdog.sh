#!/usr/bin/env bash
# Detach gsv-watchdog so :9999 comes back if gsv-live.sh / Cursor dies.
# Prefers an already-built target/debug/gsv-watchdog.exe (no cargo lock).
#
# Usage: bash scripts/gsv-watchdog.sh
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
EXE="$ROOT/target/debug/gsv-watchdog.exe"
if [ ! -f "$EXE" ]; then
  echo "gsv-watchdog: build first: cargo build --bin gsv-watchdog" >&2
  exit 1
fi
CMD="/c/Windows/System32/cmd.exe"
WIN_EXE="$(cygpath -w "$EXE" 2>/dev/null || echo "$EXE")"
WIN_ROOT="$(cygpath -w "$ROOT" 2>/dev/null || echo "$ROOT")"
if [ -x "$CMD" ]; then
  MSYS2_ARGCONV_EXCL='*' "$CMD" //c "start /min gsv-watchdog \"${WIN_EXE}\" --repo-root ${WIN_ROOT}"
  echo "gsv-watchdog: detached ($WIN_EXE)"
  exit 0
fi
echo "gsv-watchdog: cmd.exe missing; run target/debug/gsv-watchdog.exe in a spare terminal" >&2
exit 1
