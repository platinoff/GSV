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
| G16 | **done** | G10/G12 reconciled + Min placement manifest (#37+#39) |
| G22 | **done** | huge-expert canary g-730 PASS `3.47e-18` on #37 (8/8 shards) |
| G17 | **done** | — |
| G18 | open | Vision/mmproj weight-change feasibility (owner-gated) |
| G19 | open | Staged Q4_K patch — **DO NOT START** (G17 found no safe candidate) |
| **G20** | **done** | artifact-backed WGPU weight lane implemented + proved end-to-end (doc §5) |
| **G21** | **done** | Q4_K canary PASS `g-721` `6.94e-18` on peers [39,37] |
| G23 | **done** | Q6_K canary `g-736` PASS `1.73e-18/3.47e-18` on [39,37] — 122/122 experts addressable |
| G24 | **done** | durable device_key→peer_id map (`wgpu_peer_ids.json`), stable IDs across reconnect |
| G13 | blocked | owner runtime/RAM + infer/quality |

## G21 outcome (closed)

- Canon :9999 deployed (build + `/api/update/apply`), hub-side (`1b187db`) + page-side (`2fa9eb2`) Q4_K dequant live.
- `artifact/blk.0.attn_gate.weight` [2048,4096] Q4_K registered (2 chunks), chunk GET sha256 == key.
- `g-721` weights_matvec shards=2 on Min [39,37]: both `done`, `max_err_model=6.938893903907228e-18` (<1e-4). G21 done via MCP.

## Closed outcome

- G21 PASS (`g-721` 6.94e-18 on [39,37]); **G22 PASS (`g-730` 3.47e-18, 8/8 shards on #37)**. 100 Q4_K huge MoE experts registered (3638 artifact-source keys). Hub stable ~35 MB with 8-shard load. Remaining: G18 (vision, owner-gated), G13 (blocked), Q6_K lane for 20 experts.

## Next ticketed work

- **G22** `t-1795359320465948200` (in_progress): scale lane to 122 huge MoE experts. DONE + deployed to canon :9999: hub `dequant_q4_k_range` + `weights_matvec_reference_segment`, page `downloadWeightsFlat(tensor,shardId,shards)` (shard-row dequant), `x_spec=onehot:<idx>:<len>` compact enqueue, 3D cols flatten (GSV `234c01f`, `465b808`). 65 wgpu tests + clippy green. `g-726` weights_matvec on `blk.5.ffn_gate_exps.weight` shards=8 enqueued; **shard 0/8 computed by peer #37 in 926 ms @ 72 MFLOPS (256×131072)** — streaming dequant works. Group completion pending. If all 8 shards `done` with `max_err_model < 1e-4`, close G22 via MCP. Note: `apply` alone does NOT respawn when version is unchanged — manually kill the listener PID and let the watchdog respawn from the rebuilt binary. Still no runnable full LLM (G13 blocked).
- **G18** (open, owner-gated): vision/mmproj weight-change feasibility.
- **G19** — DO NOT START (G17 found no safe weight-patch candidate).

## Rules (hard)

- Rust/cargo xtask + MSYS2 bash + GSV MCP **only**. **No Perl, no Python**, no PowerShell for project work.
- No weights/secrets in git; no `git add -A`; one commit per repo; push + summary.
- NanoJev = eval/oracle/fitness only (never GGUF weight gen/transplant). Router/gate weights freeze. No silent coercion.
- No "runnable full LLM" / quality/parity claim until G13 green.
- Do not kill `target/live/gsv-server.exe` before tests/builds.

## Tooling note

- HTTP POST with JSON body: use a **bash script file** with `curl -H 'Content-Type: application/json' --data-binary "$BODY"` (works; inline shell quoting gets mangled by the harness). See temp `enqueue.sh` pattern. Do not use Perl for this.
