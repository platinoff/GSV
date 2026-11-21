# GASV-A1 — Ecosystem verification matrix + steel-plan sync

**Date:** 2026-11-21  
**Ticket:** `t-1795270402999084900` (gsv, in_progress)  
**Scope:** research/docs only. No code/weights/secrets.  
**Plan:** [`docs/superpowers/plans/2026-11-21-gasv-versus-ai-wgpu-cluster-skeleton.md`](../../superpowers/plans/2026-11-21-gasv-versus-ai-wgpu-cluster-skeleton.md)

## 1. Registered product state (live snapshot)

Source: `cargo xtask products --state` (GSV, 2026-11-21).

| id | head | branch | dirty | age(s) | ver | reg | notes |
|---|---|---|---:|---:|---|:---:|---|
| gsv | 69829ca | main...origin/main | 1 | 68 | 0.248.0 | yes | dirty = `docs/gsv/tickets.jsonl` (A1/A2 claim) |
| poolai | 52eb60dbd | main...origin/main | 0 | 63343 | 0.2.2 | yes | FM/тато; WSP sketch docs-only |
| linfs | 95d4e23 | main...origin/main | 0 | 4050 | - | yes | |
| telenetis | bc81a69 | main...origin/main | 0 | 5048 | 0.2.0 | yes | |
| llama-rs | c5ef64c | master...origin/master | 0 | 4454 | 0.1.0 | yes | |
| rebook | 869bef8 | master...origin/master | 0 | 4050 | 0.1.0 | yes | book content out of GASV scope |
| orr_desktop | 337b2b1 | master...origin/master | 0 | 4041 | 0.1.0 | yes | |
| omniroute | a2e1e9f | release/v3.8.50...origin/release/v3.8.50 | 0 | 1653 | - | yes | CNV-1 W28 pushed |
| rusty-engine | bf20c64 | main...origin/main | 5 | 5053 | 0.1.0 | yes | dirty pre-existing: Cargo.lock/toml/lib.rs + nanojev untracked; **do not touch in GASV A2 doc-only** |
| gsv-wgpu | 9c65f59 | main | 0 | 5048 | 0.1.0 | yes | **no remote** (owner-gate) |
| sport-forcast | c7213ea | main...origin/main | 0 | 4041 | 0.1.0 | yes | |
| coreboard_labs_shop | 85d6e8f | main...origin/main | 0 | 4041 | 0.1.0 | yes | |
| adoreble_wifi | c1c8ff4 | main...origin/main | 0 | 4035 | - | yes | ubuntu CI clippy pre-existing fail (known) |
| freebeat.ai | 87daf9e | main | 0 | 1975464 | - | no | unregistered |
| mesh-node | 08cf81b | main | 0 | 4516 | - | yes | **no remote** (owner-gate) |
| platinoff-profile | 5d81430 | main...origin/main | 0 | 2025942 | - | no | unregistered |

## 2. Mirror / L: snapshot (owner-confirmed)

- **Path:** `L:\gsv_mirror\wgpu_weights\` (peer mirror, disk L). 
- **Snapshot 2026-11-21:**
  - `blk7_ffn_{down,gate,up}_shexp.f32` + `blk8..12/*_shexp.f32` = 18 tensors × 4MiB = 72MiB.
  - `wgpu_weights/chunks/` = 31 chunks, ~112MiB total.
  - `wgpu_weights/index.json` present (31 entries, bytes/hits/stored_ts, hits>0).
  - `wgpu_weights_tensors.json` **not visible** in this dir snapshot; verify during B2 manifest work.
- **Code contract:** `src/boxes/weight_store.rs` — content-addressed sha256 chunks, 4MiB chunk, 4GiB cap default, manifest hardcodes `minicpm-v-4.7-1b/qwen35-hybrid/q8_0` (gap for GASV multi-model provenance).
- **Risk:** do not mix GASV experimental artifacts into live prod mirror without owner go + manifest isolation.

## 3. MCP / harness / toolchain

| Check | Status | Evidence |
|---|---|---|
| GSV MCP openbot | canon present | `docs/gsv/GSV_MCP_OPENBOT.md`; stdio `target/live/gsv-mcp.exe`, Cursor HTTP `9999/mcp` |
| Kit wiring | previously audited SYM-4 | AGENTS: cursor/opencode/grok wired; mesh-node/GSV-WGPU no remote |
| Toolchain alignment | SYM-5 done | `docs/gsv/RUST_TOOLCHAIN_ALIGNMENT_2026-11-21.md`; exceptions: adoreble_wifi/mesh-node host-agnostic, omniroute no root toolchain |
| Rust-first gate | ok | MSYS2 bash, no PowerShell/npm for GSV hub |

## 4. Steel-plan sync status

| Plan/doc | Status | Action |
|---|---|---|
| `GSV_ECOSYSTEM_SYMBIOSIS.md` | active canon | keep; GASV-A4 will clarify NanoJev/WSP/K8s boundary |
| `MESH_ROADMAP_TASKS_2026-11-19.md` | active | min set = hub+bifrost #38 confirmed for GASV |
| `K8S_WGPU_SYNTHESIS_2026-11-18.md` | active | no k8s for current fleet |
| `NANOJEV_PORT_FEASIBILITY_2026-11-18.md` | active | NanoJev safetensors eval/research, not GGUF transplant |
| `WSP_*` poolAI/GSV docs | active, docs-only/sketch | ECO-9.2 sketch; owner intent NanoJev/RL replacement direction recorded in GASV plan |
| GASV plan | added 69829ca | `docs/superpowers/plans/2026-11-21-gasv-versus-ai-wgpu-cluster-skeleton.md` |

## 5. Gaps / owner-gated

- `mesh-node`, `GSV-WGPU` — no git remotes; local commits cannot be pushed.
- `rusty-engine` dirty tree has pre-existing nanojev/bin changes — A2 must be doc-only, no staging unrelated files.
- `wgpu_weights_tensors.json` absent in L: snapshot — confirm expected by weight_store runtime.
- poolAI FM missing explicit WSP/NanoJev/WGPU-mesh rows (GASV-A4 gap).
- Full GSV `cargo test`/clippy not run in this A1 docs slice (heavy on this box); fmt check run on commit.

## 6. A1 verdict

**PASS (research/docs).** Ecosystem state captured; mirror path confirmed; blockers/gaps explicit. Next: A2 feasibility doc, A5 bifrost-minimal smoke criteria.