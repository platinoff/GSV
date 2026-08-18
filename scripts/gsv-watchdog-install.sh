#!/usr/bin/env bash
# Persist gsv-watchdog across Cursor / reboot (current user, no admin).
# Tries schtasks ONLOGON; falls back to HKCU Run.
#
# Usage: bash scripts/gsv-watchdog-install.sh
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
EXE="$ROOT/target/debug/gsv-watchdog.exe"
if [ ! -f "$EXE" ]; then
  echo "gsv-watchdog-install: build first: cargo build --bin gsv-watchdog" >&2
  exit 1
fi
WIN_EXE="$(cygpath -w "$EXE")"
WIN_ROOT="$(cygpath -w "$ROOT")"
TR="${WIN_EXE} --repo-root ${WIN_ROOT}"
SCHTASKS="/c/Windows/System32/schtasks.exe"
REG="/c/Windows/System32/reg.exe"
ok=0
if [ -x "$SCHTASKS" ]; then
  if "$SCHTASKS" //Create //TN "GSV-watchdog" //SC ONLOGON //RL LIMITED //F //TR "$TR"; then
    echo "gsv-watchdog-install: schtasks GSV-watchdog (ONLOGON)"
    ok=1
  else
    echo "gsv-watchdog-install: schtasks denied; trying HKCU Run" >&2
  fi
fi
if [ "$ok" != 1 ] && [ -x "$REG" ]; then
  "$REG" add "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run" //v GSV-watchdog //t REG_SZ //d "$TR" //f
  echo "gsv-watchdog-install: HKCU Run GSV-watchdog"
  ok=1
fi
if [ "$ok" != 1 ]; then
  echo "gsv-watchdog-install: could not persist (need schtasks or reg.exe)" >&2
  exit 1
fi
echo "gsv-watchdog-install: TR=$TR"
