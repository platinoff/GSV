#!/usr/bin/env bash
# Append one drain fingerprint JSONL row and print commit trailers.
# Usage: bash scripts/gsv-fingerprint.sh
# Env: GSV_FINGERPRINT_FILE GSV_ACTOR GSV_IDE GSV_MODEL GSV_AGENT GSV_BAND GSV_SUMMARY
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
JSONL="${GSV_FINGERPRINT_FILE:-$ROOT/docs/gsv/fingerprints.jsonl}"
ACTOR="${GSV_ACTOR:-agent}"
IDE="${GSV_IDE:-cursor}"
MODEL="${GSV_MODEL:-unknown}"
AGENT="${GSV_AGENT:-orchestrator}"
BAND="${GSV_BAND:-}"
SUMMARY="${GSV_SUMMARY:-drain close}"
json_esc() {
  printf '%s' "$1" | sed 's/\\/\\\\/g; s/"/\\"/g'
}
ver=""
in_pkg=0
while IFS= read -r line || [ -n "$line" ]; do
  case "$line" in
    '[package]') in_pkg=1 ;;
    '['*) in_pkg=0 ;;
  esac
  if [ "$in_pkg" = 1 ] && [[ "$line" =~ ^version\ =\ \"([0-9]+\.[0-9]+\.[0-9]+)\" ]]; then
    ver="${BASH_REMATCH[1]}"
    break
  fi
done < "$ROOT/Cargo.toml"
if [ -z "$ver" ]; then
  echo "gsv-fingerprint: could not read Cargo.toml version" >&2
  exit 1
fi
ts="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
git_head="$(git -C "$ROOT" rev-parse --short HEAD 2>/dev/null || true)"
mkdir -p "$(dirname "$JSONL")"
{
  printf '{"ts":"%s","actor":"%s","ide":"%s","model":"%s","agent":"%s","version":"%s"' \
    "$(json_esc "$ts")" "$(json_esc "$ACTOR")" "$(json_esc "$IDE")" \
    "$(json_esc "$MODEL")" "$(json_esc "$AGENT")" "$(json_esc "$ver")"
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
echo "gsv-fingerprint: appended $JSONL (v$ver)"
