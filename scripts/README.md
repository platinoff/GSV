# GSV kit scripts → `cargo xtask`

Product tests, benches, and scripts are **Rust**. There are no kit `.sh` / `.ps1` wrappers.

| Task | Command |
|------|---------|
| Discover VDT products | `cargo xtask products` |
| Always-on live copy | `cargo xtask live` |
| Detach watchdog | `cargo xtask watchdog` |
| Persist watchdog | `cargo xtask watchdog-install` |
| S0 disk | `cargo xtask disk` (`--enforce`) |
| git push only | `cargo xtask push` |
| Skill mirrors | `cargo xtask mirrors` |
| Band bump | `cargo xtask bump --band N` |
| Fingerprint | `cargo xtask fingerprint` |
| Speed index | `cargo xtask record-speed` |
| Rust diagnostics | `cargo xtask record-rust` |
| Vision sync | `cargo xtask sync` (`--check`) |
| 4-bit MoE bench (artifact-backed) | `cargo xtask moe-bench --tensor … --shards N` |
| Per-expert canary | `cargo xtask moe-expert-canary --expert N --role gate\|up\|down` |
| 1-layer MoE forward | `cargo xtask moe-forward --layer N [--pack <dir>]` |
| Multi-layer MoE forward | `cargo xtask moe-forward-all --layers N [--start-layer N] --pack <dir>` |
| Routing profile (Gini/entropy) | `cargo xtask moe-routing-profile --tokens N --seed N` |
| Dynamic hot-set plan | `cargo xtask moe-hotset --tokens N --budget-mb N` |
| Expert-major pack builder | `cargo xtask moe-pack --layer N --out <dir>` |
| Vision-forward / vision-chat | `cargo xtask vision-forward --image <png> …` / `vision-chat` |
| Mesh status / alive-watch | `cargo xtask status --json` / `alive-watch` |
| Ecosystem 1.100 gate | `cargo xtask eco-check` |

Implementation: `src/boxes/xtask.rs`. MCP: `gsv_xtask` / `gsv_disk`. Canon: [`docs/gsv/GSV_RUST_DEV.md`](../docs/gsv/GSV_RUST_DEV.md).
