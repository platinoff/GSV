#!/usr/bin/env bash
# Mirror .agents/skills/ → .cursor/skills/ and .opencode/skills/ (Windows: copy, not symlink).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC="$ROOT/.agents/skills"
for d in "$SRC"/*; do
  [ -d "$d" ] || continue
  name="$(basename "$d")"
  rm -rf "$ROOT/.cursor/skills/$name" "$ROOT/.opencode/skills/$name"
  mkdir -p "$ROOT/.cursor/skills" "$ROOT/.opencode/skills"
  cp -a "$d" "$ROOT/.cursor/skills/$name"
  cp -a "$d" "$ROOT/.opencode/skills/$name"
  echo "mirrored $name"
done
