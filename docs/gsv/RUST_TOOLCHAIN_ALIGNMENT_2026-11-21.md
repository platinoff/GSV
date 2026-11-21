# Rust toolchain alignment audit (SYM-5)

**Date:** 2026-11-21  
**Status:** Audit complete (docs recorded)  
**Policy (GSV canon):** `stable-x86_64-pc-windows-gnu`, `components = [rustfmt, clippy]`, targets include `x86_64-pc-windows-gnu` (ecosystem owner 2026-11-04; floats with stable).  
**Exceptions allowed:** host-agnostic `channel = "stable"` when cross-platform CI/runtime requires it (document reason).

## Registered Rust repos

| Repo | rust-toolchain.toml | Channel | Components | Targets | Status | Notes |
|---|---|---|---|---|---|---|
| GSV | yes | `stable-x86_64-pc-windows-gnu` | rustfmt,clippy | gnu | aligned | hub canon |
| poolAI | yes | `stable-x86_64-pc-windows-gnu` | rustfmt,clippy | gnu | aligned | |
| rusty-engine | yes | `stable-x86_64-pc-windows-gnu` | rustfmt,clippy | gnu | aligned | |
| llama-rs | yes | `stable-x86_64-pc-windows-gnu` | rustfmt,clippy | gnu | aligned | |
| mesh-node | yes | `stable` | rustfmt,clippy | — | **exception** | Cross-platform (Windows host, Pi/Linux target); host-agnostic to avoid CI/toolchain mismatch. No remote (owner-gate). |
| GSV-WGPU | yes | `stable-x86_64-pc-windows-gnu` | rustfmt,clippy | gnu | aligned | |
| LinFS | yes | `stable-x86_64-pc-windows-gnu` | rustfmt,clippy | gnu | aligned | |
| rebook | yes | `stable-x86_64-pc-windows-gnu` | rustfmt,clippy | gnu | aligned | |
| ORR_DESKTOP | yes | `stable-x86_64-pc-windows-gnu` | rustfmt,clippy | gnu | aligned | private |
| CoreBoard_Labs_shop | yes | `stable-x86_64-pc-windows-gnu` | rustfmt,clippy | gnu | aligned | private |
| sport-forcast | yes | `stable-x86_64-pc-windows-gnu` | rustfmt,clippy | gnu | aligned | private |
| adoreble_wifi | yes | `stable` | rustfmt,clippy | — | **exception** | Cross-platform CI (ubuntu-latest + windows-latest/msvc); gnu pin caused Format/CI regression (fixed 3149e0f). |
| telenetis | yes | `stable-x86_64-pc-windows-gnu` | rustfmt,clippy | gnu | aligned | |

## omniroute (Node-primary, Rust crates additive)

| Path | rust-toolchain.toml | Status | Notes |
|---|---|---|---|
| `omniroute/` (root) | no | **exception (documented)** | Node/TS runtime source of truth; Rust crates are CNV opt-in waves. |
| `omniroute/crates/omniroute-compression` | no | inherits root/host | CNV crate; CI/gates per wave. |
| `omniroute/crates/omniroute-gcf` | no | inherits root/host | CNV crate. |
| `omniroute/crates/omniroute-napi` | no | inherits root/host | CNV crate (napi-rs). |
| `omniroute/crates/omniroute-sse` | no | inherits root/host | CNV crate. |
| `omniroute/crates/omniroute-wasm` | no | inherits root/host | CNV crate (wasm32 target). |

**Recommendation:** keep omniroute crates inheriting host/root; add per-crate toolchain only if CI drift appears. If added, prefer host-agnostic `stable` + required targets/components for cross-platform/npm CI.

## Actions taken

- SYM-4/SYM-5: added host-agnostic `rust-toolchain.toml` to `adoreble_wifi` and `mesh-node` (cross-platform exception). adoreble_wifi pushed; mesh-node local (no remote).
- No unsafe drift fixes applied to aligned repos.

## Follow-up

- Re-run audit when adding new registered Rust repos or omniroute Rust crates.
- Keep exceptions explicit in this doc and symbiosis plan.