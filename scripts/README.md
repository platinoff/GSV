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

Implementation: `src/boxes/xtask.rs`. MCP: `gsv_xtask` / `gsv_disk`. Canon: [`docs/gsv/GSV_RUST_DEV.md`](../docs/gsv/GSV_RUST_DEV.md).
