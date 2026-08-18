# comitmsg

Local drain commit messages and logs. **Do not stage** this folder except this README.

| Kind | Pattern | Staged? |
|------|---------|---------|
| Commit message | `*.md` | no — `cargo xtask git commit --file comitmsg/<name>.md` |
| Log | `*.log` | no |
| Shell leftovers | `*.sh` / `*.txt` | no (retired; use xtask) |

```bash
cargo xtask git status
cargo xtask git log
cargo xtask git fetch
cargo xtask git commit --file comitmsg/.band156-commit-msg.md
cargo xtask git push    # or: cargo xtask push
```

Canon: `src/boxes/gitkit.rs` · `docs/gsv/GSV_RUST_DEV.md`.
