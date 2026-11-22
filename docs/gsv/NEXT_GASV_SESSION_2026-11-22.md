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
| **G20** | **done** | artifact-backed WGPU weight lane implemented + proved end-to-end (doc §5) |
| **G21** | **done** | Q4_K canary PASS `g-721` `6.94e-18` on peers [39,37] |
| G13 | blocked | owner runtime/RAM + infer/quality |

## G21 outcome (closed)

- Canon :9999 deployed (build + `/api/update/apply`), hub-side (`1b187db`) + page-side (`2fa9eb2`) Q4_K dequant live.
- `artifact/blk.0.attn_gate.weight` [2048,4096] Q4_K registered (2 chunks), chunk GET sha256 == key.
- `g-721` weights_matvec shards=2 on Min [39,37]: both `done`, `max_err_model=6.938893903907228e-18` (<1e-4). G21 done via MCP.

## Next possible work (not yet ticketed)

- Scale the verified lane to the 122 huge MoE experts (~151 MB each) — generator is ready (`rusty artifact-chunks`), registration + canary per tensor. Still no runnable full LLM (G13 blocked).
- Vision/mmproj weight-change feasibility G18 (owner-gated, open).
- G16 (G10/G12 reconcile + Min placement manifest), open.
4. Gates: `cargo test --lib`, `cargo clippy --all-targets -- -D warnings`, `gsv_disk` (12 GB min), one commit per repo, `tickets_done` with evidence. No full-LLM/quality claim; G13 blocked.

## Rules (hard)

- Rust/cargo xtask + MSYS2 bash + GSV MCP **only**. **No Perl, no Python**, no PowerShell for project work.
- No weights/secrets in git; no `git add -A`; one commit per repo; push + summary.
- NanoJev = eval/oracle/fitness only (never GGUF weight gen/transplant). Router/gate weights freeze. No silent coercion.
- No "runnable full LLM" / quality/parity claim until G13 green.
- Do not kill `target/live/gsv-server.exe` before tests/builds.

## Tooling note

- HTTP POST with JSON body: use a **bash script file** with `curl -H 'Content-Type: application/json' --data-binary "$BODY"` (works; inline shell quoting gets mangled by the harness). See temp `enqueue.sh` pattern. Do not use Perl for this.
