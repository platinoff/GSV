#!/usr/bin/env bash
# Increment [package] version patch in Cargo.toml (python-free).
# Usage: bash scripts/gsv-bump-version.sh [Cargo.toml]
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TOML="${1:-$ROOT/Cargo.toml}"
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
    patch=$((patch + 1))
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
