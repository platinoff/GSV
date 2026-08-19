# GSV rust-first tests, benches, scripts

**Status:** Accepted (owner 2026-08-18 · band **153**) · **Workspace:** `S:\rust\GSV`  
**MCP:** `gsv_xtask` · `gsv_disk` · resource `gsv://docs/rust-dev`

Product **tests**, **benchmarks**, and **kit scripts** are Rust (`.rs`). Do not add `.sh`, `.ps1`, or JSON as the implementation of those things.

JSON remains **data or host protocol** (vision snapshots, MCP client configs, `Cargo.lock`). It is not a test harness, a bench driver, or a drain script.

## Invoke

| Need | Command |
|------|---------|
| Abracadabra Step 0 | `cargo xtask products` |
| S0 disk | `cargo xtask disk` (`--enforce`; `--clean` deletes debug cache and **keeps** `target/live`) |
| Always-on UI | `cargo build --bin gsv-server --bin gsv-mcp --bin gsv-live --bin gsv-watchdog` then `cargo xtask live` (copies server + mcp) |
| Outer watchdog | `cargo xtask watchdog` / `cargo xtask watchdog-install` |
| Speeds / Clippy panels | `cargo xtask record-speed` / `cargo xtask record-rust` |
| Vision close | `cargo xtask sync` then `cargo xtask sync --check` |
| Band close | `cargo xtask bump --band N` then `cargo xtask fingerprint` (optional `--model`) |
| Skill mirrors | `cargo xtask mirrors` |
| Push after commit | `cargo xtask git push` (alias `cargo xtask push`) |
| Commit (message file) | `cargo xtask git commit --file comitmsg/<name>.md` |
| Grok Bot tunnel | `cargo xtask tunnel` (cloudflared; owner opt-in; `/mcp` becomes public) |
| Tests | `cargo test` (`tests/*.rs`) |
| Benches | `cargo bench --bench gsv_dev` (`benches/*.rs`) |

Alias: `.cargo/config.toml` → `xtask = "run --quiet --bin gsv-xtask --"`. Logic lives in `src/boxes/xtask.rs` so HTTP (`GET /api/xtask`, `GET /api/disk`) and MCP call the **same** functions.

The agent **shell** is still MSYS2 bash (`C:\msys64\usr\bin\bash.exe -lc '…'`) to run `cargo` / `git`. That is not a product script.

## What we compared (10 Rust projects)

| Project | Automation | Tests / benches | Fit for GSV |
|---------|------------|-----------------|-------------|
| [matklad/cargo-xtask](https://github.com/matklad/cargo-xtask/) | Spec: `cargo xtask` alias, no Make/bash required | n/a (pattern) | **Adopted.** GSV keeps xtask **in the `gsv` crate** so MCP/Galaxy can import it (a separate `xtask/` member cannot). |
| [rust-analyzer](https://github.com/rust-lang/rust-analyzer/tree/master/xtask) | `xtask/` crate: install, dist, codegen, metrics | `tests/` + rustc-style | Same alias. Their xtask is release/codegen; ours is drain/S0/live/MCP. |
| [rust-lang/cargo](https://github.com/rust-lang/cargo) | cargo-xtask alias | integration tests in Rust | Confirms even Cargo itself uses xtask, not project `.sh`. |
| [helix-editor/helix](https://github.com/helix-editor/helix/tree/master/xtask) | xtask: query validation, docs | Rust tests | Docs/codegen in Rust — same direction as `gsv-vision-sync`. |
| [clap-rs/clap](https://github.com/clap-rs/clap) | xtask + complete generation | lib tests | CLI completeness in Rust. GSV stays clap-free (manual args, smaller crate). |
| [tokio-rs/tokio](https://github.com/tokio-rs/tokio) | CI YAML + `tests/` | `benches/` Criterion | Tests/benches are `.rs`. YAML is CI, not product scripts. |
| [BurntSushi/ripgrep](https://github.com/BurntSushi/ripgrep) | little shell; tests in Rust | integration `.rs` | Search tool: product tests are Rust. |
| [gfx-rs/wgpu](https://github.com/gfx-rs/wgpu) | xtask-style workspace tasks | `tests/` + benches | GPU stack still automates in Rust. |
| [zellij-org/zellij](https://github.com/zellij-org/zellij) | mix of xtask + some CI shell | Rust tests | Shell leftover is CI; GSV goes further and deletes product `.sh`. |
| [rust-lang/rust](https://github.com/rust-lang/rust) bootstrap | `x.py` / bootstrap in Rust | compiletest | Heavyweight; we do **not** copy `x.py`. cargo-xtask is the right scale. |

**Verdict:** our drain already used `src/bin/` for vision-sync, loc-audit, watchdog, MCP. The remaining `scripts/*.sh` / `bin/*.sh` were thin wrappers and Windows-only detach. Moving them into `boxes/xtask.rs` matches cargo-xtask **and** GSV’s MCP/Galaxy rule (one box, many clients).

## Still not `.rs`

| Kind | Why |
|------|-----|
| `.mcp.json` / `opencode.json` | Host MCP stdio (live `gsv-mcp`). |
| `.cursor/mcp.json` | Cursor HTTP MCP (`url` → live `:9999/mcp`). Folder **GSV** only — never User (`%USERPROFILE%/.cursor/mcp.json`). |
| `docs/vision/*.json` | Snapshots written **by** `gsv-vision-sync` / speed-index bins. |
| `docs/gsv/fingerprints.jsonl` | Append-only data. Writer is Rust. |
| Marketplace `find-polluter.sh` under skills | Upstream skill copy, not a GSV product script. |
| `comitmsg/*.md` / `*.log` | Local commit messages and logs; never staged except `comitmsg/README.md`. Use `cargo xtask git`. |

## MCP

- `gsv_xtask` `{task}` — `catalog` (default) · `products` · `disk` · `sync` (`--check` drift only). Mutating names (`push`, `bump`, `live`, remirror, …) return a tool error: run `cargo xtask` or `gsv_vision_sync`.
- `gsv_disk` `{enforce?}` — S0 guard (`free_mb` / `free_gb`; `--clean` is CLI only, never MCP).
- Resource `gsv://docs/rust-dev` — this file.
- Drain prompt names `gsv_xtask`, `gsv_disk`, and this URI.

## Ratio

`gsv-loc-audit` already counts `benches/*.rs` as `rust_benches`. Leftover git-tracked `bin/*.sh` / `scripts/*.sh` stay `ops_shell` and add a note to drop them.
