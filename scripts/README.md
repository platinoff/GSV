# GSV kit scripts

MSYS2 bash only (`C:\msys64\usr\bin\bash.exe`). No PowerShell. No `.ps1`.

| Script | Role |
|--------|------|
| `list-vdt-products.sh` | Discover workspace + sibling git repos for `абракадабра` / `abrakadabra` Step 0 |
| `check_target_disk.sh` | S0 disk guard (`GSV_MIN_FREE_DISK_GB` / `GSV_MAX_TARGET_DIR_GB`) |
| `git-push-only.sh` | `git push origin main` after a sprint commit |
| `sync-vdt-skill-mirrors.sh` | copy `.agents/skills/` → `.cursor/skills/` + `.opencode/skills/` |

Product wrappers live in `bin/` (timing / clippy JSON / vision-sync). Rust bins stay in `src/bin/`.
