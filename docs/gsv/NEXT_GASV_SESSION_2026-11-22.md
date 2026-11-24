# NEXT — GASV session handoff (2026-11-24, hy4-flash-free → next session)

## Steady state (verified)

- **HEADs:** GSV `cc7158a` (clean, pushed), rusty-engine `61bb0ba` (clean, pushed).
- **Cluster:** Min active = hub RDNA2 WGPU #37 + bifrost #39. Valhall #40 withdrawn by owner; Med/Max spec/gated.
- **Best 4-bit MoE у мірорі:** `L:/gsv_mirror/models/gsv-versus-ai/gsv-versus-ai-full-q4k.gguf` — 733/733 tensors, 21,166,255,488 B, Q4_K/Q6_K direct-copy, sha256 `2f37d609…`. Записаний у `gsv-versus-ai.publish.json` як `best_4bit_companion` + `files[role=versioned-4bit-companion]`. **Канон F32 `25fadfbb…` не змінено.** Q4_K vision companions у
`L:/gsv_mirror/models/gsv-versus-ai/`: `gsv-versus-ai-vision-q4k.gguf` (2048,
геометрично сумісний із versus-ядром), `inkling2-vision-q4k.gguf` (2304),
`minicpm1b-vision-q4k.gguf` (1024). Усі — versioned companions, не канон.
- **Fitness:** G15 done — Min canaries g-713/g-714 (blk.3/blk.11 attn_output, peers [39,37]) PASS `6.94e-18`; scorecard `docs/gsv/research/GASV_G15_FITNESS_SCORECARD_2026-11-22.md`. NanoJev eval-oracle operational (inspect 322 tensors; predict/replay CPU-bound).
- **G17 done (audit-only):** no Astra patch candidate — 1B qwen35 layer-23 is generic artifact (EN control 0.3762 > UK fact 0.3251), Granite audit blocked by qwen35-only geometry. Report `GASV_G17_ASTRA_CANDIDATE_AUDIT_2026-11-22.md`.

## Tickets (board, via GSV MCP)

| Ticket | Status | Next action |
|---|---|---|
| G15 | **done** | — |
| G16 | **done** | G10/G12 reconciled + Min placement manifest (#37+#39) |
| G17 | **done** | — |
| G18 | **done** | Vision/mmproj feasibility verdict delivered (owner-gated code) |
| G19 | open | Staged Q4_K patch — **DO NOT START** (G17 found no safe candidate) |
| **G20** | **done** | artifact-backed WGPU weight lane implemented + proved end-to-end (doc §5) |
| **G21** | **done** | Q4_K canary PASS `g-721` `6.94e-18` on peers [39,37] |
| G22 | **done** | huge-expert Q4_K canary `g-730` PASS `3.47e-18` on #37 (8/8 shards) |
| G23 | **done** | Q6_K canary `g-736` PASS `1.73e-18/3.47e-18` on [39,37] — 122/122 experts addressable |
| G24 | **done** | durable device_key→peer_id map (`wgpu_peer_ids.json`), stable IDs across reconnect |
| G25–G30 | **done** | FM/roadmap 1.100 adaptation · eco-check scanner · SLI adoption · policy matrix · gates fact-check |
| G31 | **done** | `gsv_gasv_status` live-state fix (see addendum below) |
| G32 | **done** | Q4_K vision weights on Min cluster: `g-739` `1.39e-17`, `g-740` `6.94e-18` |
| G33 | **done** | result bus `GET /api/wgpu/task/result` + `vision-forward` (1/3 blocks) |
| G34 | **done** | versioned binding: 4-bit text core + 2048/2304 Q4_K vision companions |
| G35 | **done** | full 23/23 Inkling-2 Q4_K forward: 557 cluster ops, 2304-dim, 7583 s |
| G36 | **done** | 2048-dim vision forward on single peer `#37` |
| G37 | **done** | `--image <png>` real patch input (29 ops, 2304-dim, 252 s) |
| G38 | **done** | cross-artifact tensor-label collision fix (`partial length mismatch`) |
| G39 | **done** | full 27/27 geometry-matched 2048 forward with real PNG (653 ops, 6559 s) |
| G40 | **done** | 1B geometry match 1024 + rusty-engine PH-1204 `--vision-prefix` plumbing |
| G41 | **done** | `--checkpoint/--resume`; checkpoint reached 18/27 before RAM block |
| G42 | **done** | dropped-partial retry + automation `aut_20e65ced2001Kt1aKXZ0aZaSpk` |
| G13 | blocked | owner runtime/RAM + infer/quality |
| G19 | open | no safe Astra-style weight-patch candidate — DO NOT START |
| G11 | open | Superseded by G15 scorecard; keep open pending owner close |

## G21 outcome (closed)

- Canon :9999 deployed (build + `/api/update/apply`), hub-side (`1b187db`) + page-side (`2fa9eb2`) Q4_K dequant live.
- `artifact/blk.0.attn_gate.weight` [2048,4096] Q4_K registered (2 chunks), chunk GET sha256 == key.
- `g-721` weights_matvec shards=2 on Min [39,37]: both `done`, `max_err_model=6.938893903907228e-18` (<1e-4). G21 done via MCP.

## Closed outcome

- G21 PASS (`g-721` 6.94e-18 on [39,37]); **G22 PASS (`g-730` 3.47e-18, 8/8 shards on #37)**. 100 Q4_K huge MoE experts registered (3638 artifact-source keys). Hub stable ~35 MB with 8-shard load. Q6_K lane done (G23, 122/122 experts); vision work G32–G42 done. Remaining: G13 (blocked), G19 (open, no safe candidate).

## Next ticketed work (2026-11-24)

- **G35/G39/G41/G42 — vision lane done.** Full 23/23 (2304-dim) and 27/27 (2048-dim) Q4_K forwards completed on peer `#37`; `--image`, `--out`, `--checkpoint/--resume`, result bus and dropped-partial retry are in place.
- **Resume automation `aut_20e65ced2001Kt1aKXZ0aZaSpk`** (every 20 min, 8 h expiry): continues the 1B Q4_K projector from `data/vision_prefix_1b_cp.json` (`next_block=18/27`) toward the full 1024-dim vector for the locally runnable MiniCPM 1B core. Host RAM pressure is the blocker, not code.
- **G13 — blocked**: owner-approved runtime/RAM node + infer/quality gate; no runnable-full-LLM/quality claim.
- **G19 — open, DO NOT START**: G17 found no safe Astra-style weight-patch candidate.

## 2026-11-23 addendum — `gsv_gasv_status` live-state fix (G31)

The MCP status tool had hardcoded `artifact.status:"planned"` and dead peer ids
(`#35/#36/#38`), which contradicted the published artifact and the Min cluster.
Fixed in `src/mcp.rs`:

- `gasv_artifact_from_publish()` reads `L:/gsv_mirror/models/gsv-versus-ai/gsv-versus-ai.publish.json`
  at runtime (override `GSV_GASV_PUBLISH_JSON`). Missing/unreadable publish set →
  `status:"unknown"` + `note` (no artifact claim). Live probe now reports
  `status:"published"`, canonical F32 `25fadfbb…`, 4-bit companion
  `gsv-versus-ai-full-q4k.gguf` (733 tensors, sha256 `2f37d609…`) and the
  g-721/g-730/g-736 fitness canaries.
- `gasv_cluster_sets_wire()` reflects the current steady state: Min =
  hub RDNA2 `#37` + bifrost `#39` (execution); valhall `#40` owner-withdrawn;
  Med gated / Max spec-only. No `#35`/`#38` rows.
- Stale drain-prompt contract `tool_count (60)` → `61` (TOOL_COUNT symbol),
  updated in `src/mcp.rs` and `tests/gsv_mcp_contracts.rs`.

Gates: `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`,
`cargo test -q` (all suites 0 failed), stdio `tools/call gsv_gasv_status` probe.
Ticket `t-1795434418581894100` claim→done via GSV MCP. This is read-only status
plumbing — no mirror/canonical mutation, no owner-gated item started.

## Rules (hard)

- Rust/cargo xtask + MSYS2 bash + GSV MCP **only**. **No Perl, no Python**, no PowerShell for project work.
- No weights/secrets in git; no `git add -A`; one commit per repo; push + summary.
- NanoJev = eval/oracle/fitness only (never GGUF weight gen/transplant). Router/gate weights freeze. No silent coercion.
- No "runnable full LLM" / quality/parity claim until G13 green.
- Do not kill `target/live/gsv-server.exe` before tests/builds.

## Tooling note

- HTTP POST with JSON body: use a **bash script file** with `curl -H 'Content-Type: application/json' --data-binary "$BODY"` (works; inline shell quoting gets mangled by the harness). See temp `enqueue.sh` pattern. Do not use Perl for this.

## 2026-11-24 addendum — GASV G32–G42, Q4_K vision on cluster

- **HEADs:** GSV `cc7158a`, rusty-engine `61bb0ba` (both clean, pushed). MCP namespace restored, `tool_count=61`.
- **Full forwards on peer `#37` (RDNA2):**
  - Inkling-2 Q4_K 23/23 blocks: 557 cluster ops, 2304-dim finite vector, `max_abs 3.57`, `ok=true`, 7583 s (`target/vision_forward_full23_final2.json`).
  - Geometry-matched 2048 Q4_K 27/27 blocks with real PNG: 653 ops, 2048-dim finite, `max_abs 1.7985`, 6559 s; `vision-recheck` gives `ok=true` under hub gate 1e-2 (one op 1.1486e-4 above the stricter proof gate).
- **Tooling:** result bus `GET /api/wgpu/task/result?group=<id>`; `vision-forward --image/--out/--checkpoint/--resume`; `--vision-prefix <f32>` in rusty-engine (PH-1204 plumbing, state-cache bypass); per-block `gsv.vision-checkpoint.v1`; dropped-partial retry (3 attempts).
- **Vision→text geometry match:** MiniCPM 1B projector outputs 1024 = 1B text core `d_model`; `minicpm1b-vision-q4k.gguf` (308 955 964 B) + `vision_prefix_1024.f32` exported. Honest: hook mechanically works, full-depth 1024 vector still in progress (checkpoint 18/27), no image→answer/quality claim.
- **Gates:** cargo test 52 suites / 0 failed, clippy `-D warnings`, loc-audit 99.66%, rules-check 5/5, sync rev 516, next PH-S3129.
- **Owner note:** `docs/gsv/research/OWNER_NOTE_2026-11-23_4BIT_VISION_CLUSTER.md`; binding `docs/gsv/research/GASV_G34_VISION_4BIT_BINDING_2026-11-23.md`.
