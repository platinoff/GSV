#!/usr/bin/env bash
# Discover VDT products from the live environment (workspace + sibling git repos).
# Used by «абракадабра» Step 0 — do not hardcode gsv|poolai.
set -euo pipefail

KIT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PARENT="$(cd "$KIT_ROOT/.." && pwd)"
WORKSPACE="$KIT_ROOT/gsv.code-workspace"
REGISTRY="$KIT_ROOT/docs/gsv/PRODUCTS.md"

win_path() {
  if command -v cygpath >/dev/null 2>&1; then
    cygpath -m "$1"
  else
    echo "$1"
  fi
}

slug_of() {
  basename "$1" | tr '[:upper:]' '[:lower:]' | tr -d ' '
}

registered_id() {
  local slug="$1"
  case "$slug" in
    gsv) echo "gsv" ;;
    poolai) echo "poolai" ;;
    *) echo "$slug" ;;
  esac
}

is_registered() {
  local id="$1"
  grep -qiE "^\| \*\*${id}\*\*" "$REGISTRY" 2>/dev/null
}

declare -A SEEN=()
ROWS=()

add_row() {
  local path="$1"
  local source="$2"
  [ -d "$path" ] || return 0
  local real
  real="$(cd "$path" && pwd)"
  [ -n "${SEEN[$real]:-}" ] && return 0
  SEEN[$real]=1

  local name slug id kind gitf cargo node reg
  name="$(basename "$real")"
  slug="$(slug_of "$real")"
  id="$(registered_id "$slug")"
  kind="folder"
  gitf="no"
  cargo="no"
  node="no"
  [ -d "$real/.git" ] || [ -f "$real/.git" ] && gitf="yes"
  [ -f "$real/Cargo.toml" ] && cargo="yes"
  [ -f "$real/package.json" ] && node="yes"
  if [ "$cargo" = "yes" ]; then
    kind="rust"
  elif [ "$node" = "yes" ]; then
    kind="node"
  elif [ "$gitf" = "yes" ]; then
    kind="git"
  fi
  reg="no"
  if is_registered "$id"; then
    reg="yes"
  fi
  ROWS+=("$id	$name	$(win_path "$real")	$kind	$reg	$source	$gitf	$cargo")
}

# 1) Cursor / OpenCode workspace folders
if [ -f "$WORKSPACE" ]; then
  while IFS= read -r rel; do
    [ -z "$rel" ] && continue
    if [ "$rel" = "." ]; then
      add_row "$KIT_ROOT" "workspace"
    else
      add_row "$KIT_ROOT/$rel" "workspace"
    fi
  done < <(grep -oE '"path"[[:space:]]*:[[:space:]]*"[^"]+"' "$WORKSPACE" | sed 's/.*"path"[[:space:]]*:[[:space:]]*"//;s/"$//')
fi

# 2) Sibling git repos under the parent of this kit (S:/rust)
if [ -d "$PARENT" ]; then
  for d in "$PARENT"/*; do
    [ -d "$d" ] || continue
    if [ -d "$d/.git" ] || [ -f "$d/.git" ]; then
      add_row "$d" "sibling"
    fi
  done
fi

# Always include the kit itself
add_row "$KIT_ROOT" "kit"

printf 'id\tname\tpath\tkind\tregistered\tsource\tgit\tcargo\n'
for row in "${ROWS[@]}"; do
  printf '%s\n' "$row"
done
