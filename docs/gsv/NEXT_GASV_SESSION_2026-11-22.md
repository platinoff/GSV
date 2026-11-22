# NEXT — GASV session handoff (2026-11-22, iceman-free → next session)

## Steady state (verified)

- **HEADs:** GSV `f8e8ae7` (clean, pushed), rusty-engine `3bb3397` (clean, pushed).
- **Cluster:** Min active = hub RDNA2 WGPU #37 + bifrost #39. Valhall #40 withdrawn by owner; Med/Max spec/gated.
- **Best 4-bit MoE у мірорі:** `L:/gsv_mirror/models/gsv-versus-ai/gsv-versus-ai-full-q4k.gguf` — 733/733 tensors, 21,166,255,488 B, Q4_K/Q6_K direct-copy, sha256 `2f37d609…`. Записаний у `gsv-versus-ai.publish.json` як `best_4bit_companion` + `files[role=versioned-4bit-companion]`. **Канон F32 `25fadfbb…` не змінено.**
- **Fitness:** G15 done — Min canaries g-713/g-714 (blk.3/blk.11 attn_output, peers [39,37]) PASS `6.94e-18`; scorecard `docs/gsv/research/GASV_G15_FITNESS_SCORECARD_2026-11-22.md`. NanoJev eval-oracle operational (inspect 322 tensors; predict/replay CPU-bound).
- **G17 done (audit-only):** no Astra patch candidate — 1B qwen35 layer-23 is generic artifact (EN control 0.3762 > UK fact 0.3251), Granite audit blocked by qwen35-only geometry. Report `GASV_G17_ASTRA_CANDIDATE_AUDIT_2026-11-22.md`.

## Tickets (board, via GSV MCP)

| Ticket | Status | Next action |
|---|---|---|
| G15 | **done** | — |
| G16 | open | G10/G12 reconcile + Min placement manifest (#37+#39) |
| G17 | **done** | — |
| G18 | open | Vision/mmproj weight-change feasibility (owner-gated) |
| G19 | open | Staged Q4_K patch — **DO NOT START** (G17 found no safe candidate) |
| **G20** | **in_progress** | artifact-backed WGPU weight lane — next focus |
| G13 | blocked | owner runtime/RAM + infer/quality |

## Exact next steps (G20)

1. Read `docs/gsv/research/GASV_G20_ARTIFACT_WEIGHT_LANE_2026-11-22.md`. Code slice already in `src/boxes/weight_store.rs`: `read_chunk` falls back to `read_chunk_artifact` with sha256 verification; unit test green; clippy green.
2. Implement the **generator** (Rust, `cargo xtask` or `rusty` subcommand): parse artifact GGUF header → for a huge tensor split bytes into ≤4 MiB slices → sha256 each slice as key → append rows to `data/wgpu_weights_artifact_sources.json` → register tensor via `weight_store::register_tensor`.
3. Live-prove: register `blk.5.ffn_gate_exps.weight`, `GET /api/wgpu/weights/chunk?key=…` serves range bytes, enqueue `weights_matvec` canary on Min (#37+#39), require `max_err_model < 1e-4`.
4. Gates: `cargo test --lib`, `cargo clippy --all-targets -- -D warnings`, `gsv_disk` (12 GB min), one GSV commit, `tickets_done` with evidence.

## Rules (hard)

- Rust/cargo xtask + MSYS2 bash + GSV MCP **only**. **No Perl, no Python**, no PowerShell for project work.
- No weights/secrets in git; no `git add -A`; one commit per repo; push + summary.
- NanoJev = eval/oracle/fitness only (never GGUF weight gen/transplant). Router/gate weights freeze. No silent coercion.
- No "runnable full LLM" / quality/parity claim until G13 green.
- Do not kill `target/live/gsv-server.exe` before tests/builds.

## Tooling note

- HTTP POST with JSON body: use a **bash script file** with `curl -H 'Content-Type: application/json' --data-binary "$BODY"` (works; inline shell quoting gets mangled by the harness). See temp `enqueue.sh` pattern. Do not use Perl for this.
