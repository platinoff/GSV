#!/usr/bin/env bash
# Append one drain fingerprint JSONL row and print commit trailers.
# Usage: bash scripts/gsv-fingerprint.sh
# Env: GSV_FINGERPRINT_FILE GSV_ACTOR GSV_IDE GSV_MODEL GSV_AGENT GSV_BAND GSV_SUMMARY
#      GSV_PRODUCT (default gsv) GSV_PRODUCT_ROOT (tree to read version from)
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
JSONL="${GSV_FINGERPRINT_FILE:-$ROOT/docs/gsv/fingerprints.jsonl}"
ACTOR="${GSV_ACTOR:-agent}"
IDE="${GSV_IDE:-cursor}"
MODEL="${GSV_MODEL:-unknown}"
AGENT="${GSV_AGENT:-orchestrator}"
BAND="${GSV_BAND:-}"
SUMMARY="${GSV_SUMMARY:-drain close}"
PRODUCT="${GSV_PRODUCT:-gsv}"
PRODUCT_ROOT="${GSV_PRODUCT_ROOT:-$ROOT}"
json_esc() {
  printf '%s' "$1" | sed 's/\\/\\\\/g; s/"/\\"/g'
}
# MSYS: Windows `C:\tmp\...` would eat `\t` as tab if left unquoted-expanded in paths.
to_unix_path() {
  printf '%s' "$1" | sed 's#\\#/#g'
}
JSONL="$(to_unix_path "$JSONL")"
PRODUCT_ROOT="$(to_unix_path "$PRODUCT_ROOT")"
read_cargo_ver() {
  local toml="$1"
  [ -f "$toml" ] || return 1
  local in_pkg=0
  local line
  while IFS= read -r line || [ -n "$line" ]; do
    case "$line" in
      '[package]') in_pkg=1 ;;
      '['*) in_pkg=0 ;;
    esac
    if [ "$in_pkg" = 1 ] && [[ "$line" =~ ^version\ =\ \"([0-9]+\.[0-9]+\.[0-9]+)\" ]]; then
      printf '%s' "${BASH_REMATCH[1]}"
      return 0
    fi
  done < "$toml"
  return 1
}
read_npm_ver() {
  local json="$1"
  [ -f "$json" ] || return 1
  local ver
  ver="$(sed -n 's/.*"version"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$json" | head -1)"
  [ -n "$ver" ] || return 1
  printf '%s' "$ver"
}
ver=""
ver="$(read_cargo_ver "$PRODUCT_ROOT/Cargo.toml" || true)"
if [ -z "$ver" ]; then
  ver="$(read_npm_ver "$PRODUCT_ROOT/package.json" || true)"
fi
if [ -z "$ver" ]; then
  echo "gsv-fingerprint: no version in $PRODUCT_ROOT (Cargo.toml / package.json)" >&2
  exit 1
fi
ts="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
git_head="$(git -C "$PRODUCT_ROOT" rev-parse --short HEAD 2>/dev/null || true)"
mkdir -p "$(dirname "$JSONL")"
{
  printf '{"ts":"%s","actor":"%s","ide":"%s","model":"%s","agent":"%s","version":"%s","product":"%s"' \
    "$(json_esc "$ts")" "$(json_esc "$ACTOR")" "$(json_esc "$IDE")" \
    "$(json_esc "$MODEL")" "$(json_esc "$AGENT")" "$(json_esc "$ver")" \
    "$(json_esc "$PRODUCT")"
  if [ -n "$git_head" ]; then
    printf ',"git_head":"%s"' "$(json_esc "$git_head")"
  fi
  if [ -n "$BAND" ]; then
    printf ',"band":"%s"' "$(json_esc "$BAND")"
  fi
  printf ',"summary":"%s"}\n' "$(json_esc "$SUMMARY")"
} >> "$JSONL"
echo "Gsv-Actor: $ACTOR"
echo "Gsv-Ide: $IDE"
echo "Gsv-Model: $MODEL"
echo "Gsv-Product: $PRODUCT"
echo "gsv-fingerprint: appended $JSONL (product=$PRODUCT v$ver)"
