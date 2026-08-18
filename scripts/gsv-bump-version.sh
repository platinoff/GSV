#!/usr/bin/env bash
# Set [package] version so semver minor equals the VDT band (python-free).
# Usage: bash scripts/gsv-bump-version.sh --band N [Cargo.toml]
#    or: GSV_BAND=N bash scripts/gsv-bump-version.sh [Cargo.toml]
# Same band already on minor N → patch +1 (0.149.0 → 0.149.1).
# New band → MAJOR.N.0 (keeps major).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BAND="${GSV_BAND:-}"
TOML=""
while [ $# -gt 0 ]; do
  case "$1" in
    --band)
      shift
      BAND="${1:-}"
      if [ -z "$BAND" ]; then
        echo "gsv-bump-version: --band needs a number" >&2
        exit 1
      fi
      shift
      ;;
    --help|-h)
      sed -n '1,8p' "$0"
      exit 0
      ;;
    *)
      TOML="$1"
      shift
      ;;
  esac
done
TOML="${TOML:-$ROOT/Cargo.toml}"
if ! [[ "$BAND" =~ ^[0-9]+$ ]]; then
  echo "gsv-bump-version: missing --band (or GSV_BAND)" >&2
  exit 1
fi
if [ ! -f "$TOML" ]; then
  echo "gsv-bump-version: missing $TOML" >&2
  exit 1
fi
tmp="${TOML}.bump.$$"
in_pkg=0
bumped=0
new_ver=""
while IFS= read -r line || [ -n "$line" ]; do
  case "$line" in
    '[package]') in_pkg=1 ;;
    '['*) in_pkg=0 ;;
  esac
  if [ "$in_pkg" = 1 ] && [ "$bumped" = 0 ] && [[ "$line" =~ ^version\ =\ \"([0-9]+)\.([0-9]+)\.([0-9]+)\" ]]; then
    major="${BASH_REMATCH[1]}"
    minor="${BASH_REMATCH[2]}"
    patch="${BASH_REMATCH[3]}"
    if [ "$minor" = "$BAND" ]; then
      patch=$((patch + 1))
    else
      minor="$BAND"
      patch=0
    fi
    new_ver="${major}.${minor}.${patch}"
    line="version = \"${new_ver}\""
    bumped=1
    in_pkg=0
  fi
  printf '%s\n' "$line"
done < "$TOML" > "$tmp"
if [ "$bumped" != 1 ]; then
  rm -f "$tmp"
  echo "gsv-bump-version: no [package] version = \"X.Y.Z\" in $TOML" >&2
  exit 1
fi
mv "$tmp" "$TOML"
echo "gsv-bump-version: $new_ver"
