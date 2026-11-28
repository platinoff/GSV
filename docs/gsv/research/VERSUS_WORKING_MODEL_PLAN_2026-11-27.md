# gsv-versus-ai — Research → adapted plan to a working model (2026-11-27)

**Status:** research + gates + plan. **No model answer and no quality claim**
(G13 blocked). BENCH-2 not run (two phones unavailable). `runnable=false` on
the mirror artifact stays as-is.

## 0. Verdict

The 4-bit numerics lane is **working today** on one bifrost phone: the exact
21 GB `gsv-versus-ai-full-q4k.gguf` was run directly (no `--pack`) with
`max_err_model 1.82e-12`, all 8 shards on peer `#39` (proof bootstrapped with
a `matmul64` probe). But this is **not a model forward**: the handoff is
zero-padded 512→2048, with no residual/RMSNorm/attention/GDN and no vocab
head. To produce a token we must first bind real GGUF experts in rusty-engine
(`PH-406`), then build a qwen35moe runner (`PH-1206`), wire serve (`PH-1306`),
and only then let G13 green the runtime/quality gate. The first two are
**owner-gated** (dedicated node/disk), and the owner already declined extra
hardware. The honest plan therefore has a non-owner-gated prep lane and an
owner-gated execution lane.

## 1. Gates actually run for this plan (evidence)

| Gate | Result |
|---|---|
| `cargo bench --bench moe` (rusty-engine, PH-407 synthetic A/B, 20 runs, 16 experts, top-8) | `resident 25,691,320 ns/iter` vs `cache_topk 77,798,960 ns/iter` — byte-budgeted cache **3.03× slower** while all experts fit RAM |
| `moe-routing-profile` (GSV, 64 tokens, seed 1) | `gini 0.969`, `entropy_bits 0.379`, `hot_count 8`, hot experts `[35,90,100,152,180,223,16,46]` |
| `moe-hotset --phase prefill` | `resident 8`, `used_bytes 16,318,464` (budget 256 MiB) |
| `moe-hotset --phase decode` | `resident 4`, `used_bytes 8,159,232` |
| GGUF-no-pack live run on `#39` (previous ticket) | `ok=true`, `max_err_model 1.8168685143610944e-12`, top-8 `[26,102,135,97,39,248,131,53]`, `out_len 512`, `all_finite=true` |
| GSV `cargo test --lib` (default target) | **609 passed / 0 failed / 5 ignored**, EXIT=0. Full `cargo test` could not rebuild `target/debug/gsv-xtask.exe` (locked by the resident `cycle-watch` daemon); an isolated `CARGO_TARGET_DIR` instead made two non-hermetic `watchdog` tests fail (they read ambient `CARGO_TARGET_DIR` instead of the passed root) — artifact of the test env, not a code regression |
| `cargo xtask rules-check` (earlier this session) | 12/12 `[ok]`, ratio 99.68%, live watchdog alive |

## 2. Capabilities already in tree (with evidence)

**GSV numerics lane** — `src/boxes/moe_forward.rs`:
- real GGUF byte ranges + Q4_K/Q6_K block shapes `:21-148`; `register_expert_range`
  sha256 fail-closed `:296-332`; router/top-8 `:250-287`; `routing_stats`
  Gini/entropy `:457-513`; `HotExpertSet` + Belady + watermark `:515-689`;
  `run_expert` gate→up→SwiGLU→**real down vector** under `Semaphore(2)` `:833-994`;
  pack-or-GGUF `resolve_expert_slices` `:788-831`; 1-layer and N-layer reports
  `:996-1234`.
- Data plane: `weight_store.rs` LOAD-0/2/3, `moe_pack.rs` LOAD-1/6,
  `server/mod.rs` LOAD-5, `wgpu.rs` SCALE-1a/PEER-1.
- Honest boundary: `"numerics_only":true`, `"no_quality_claim":true` `:1115-1116`;
  `handoff_hidden` zero-pads 512→2048 **without** attention/GDN/residual/norm
  `:1138-1147`.

**rusty-engine substrate** — `src/moe.rs`: `moe_layer_forward_cached(... load)`
`:390-431` is the exact hook for real experts; `ExpertCache` LRU/pin/prefetch
`src/cache.rs:53-279` + PH-404a next-use `:35-51,215-272`; `q35.rs` full
attention geometry `:32-266`; `runner35.rs` **dense only** (18 GDN + 6 full
blocks with dense `ffn_gate/up/down`) `:83-108,222-285`; `model.rs` validates
`expert_count`/`expert_used_count` `:239-240,366-391` but has no expert runner.
`eval.rs` perplexity primitives exist (model-free).

**Bench/CLI harnesses:** `moe-bench`, `moe-expert-canary`, `moe-pack`,
`moe-forward`, `moe-forward-all`, `moe-hotset`, `moe-routing-profile`,
`vision-forward/chat`; rusty `manifest|dump-f32|artifact-report|quantize-export`.
PH-1305 qwen35 parity is **dense 1B only** and does not transfer to MoE.

## 3. Gaps (exact, not vague)

| Gap | Ticket / evidence |
|---|---|
| Real GGUF expert binding (byte-range → Q4_K/Q6_K dequant → `ExpertCache` load closure) | **PH-406** open; `docs/roadmap.md:161` names `src/moe/forward.rs` — file does not exist |
| qwen35moe runner (routed+shared MoE FFN, residual/norm, embedding/output head, KV+GDN state, argmax) | **PH-1206** open; `runner35.rs:222-224` dense FFN; CLI binds runner only for `arch=="qwen35"` (`rusty.rs:1037,1268`) |
| serve dispatch for `arch=="qwen35moe"` + real `active_experts` | **PH-1306** open |
| Model semantics in GSV lane (attention/GDN/norm/KV/GDN state/vocab) | `moe_forward.rs:1138-1147` — deliberately absent; do not "fix" by faking |
| Runtime/quality gate | **G13** `t-1795273000000000015` blocked — owner-approved RAM node required |
| Vision branch: `src/vit.rs`, trained vision→text alignment | **PH-1202/1203/1204/1205** open; 4-token prefix degenerates text; separate from token path |
| Orchestration: 1 proof-bearing peer, `concurrent=1`, single JSONL claim slot, hub restarts | SCALE-1 parked until ≥10 active peers; proof bootstrap recipe in `VERSUS_MINIMAL_CLUSTER_FEASIBILITY_2026-11-27.md` §6.1 |
| RAM/disk | 7.4 GB total / ~0.58 GB free; `L:` 7.3 GB free — no second pack, no resident 21 GB model; owner declined extra node |

## 4. Adapted plan — ordered, with gates

### Lane A — what we can keep green without owner go
**A0 (done): numerics lane.** Keep `moe-forward --pack` / `moe-forward-all
--layers 2..4` with proof bootstrap; gate `max_err_model < 1e-4`, pack/GGUF
bit-identical, checkpoint resumes. Evidence §1. No semantic changes here.

**A1 (prep, no node): pure expert-range loader helper in rusty-engine.**
Add a std-only helper that maps `(layer,expert,role)` → GGUF/pack byte range
+ Q4_K/Q6_K dequant into `ExpertWeights`, unit-tested on synthetic blocks and
cross-checked against the already-registered `moe_pack` manifest. It is the
pure half of PH-406 and needs no 20 GB load. Gate: fmt/clippy `-D warnings`/
full test; parity with GSV golden `< 1e-4` **when** a node exists. This is the
first thing to implement next.

**A2 (prep, no node): qwen35moe block-composition sketch/tests.** Pure state
structures (residual + RMSNorm + attention/GDN state + routed/shared FFN)
with deterministic toy weights, proving topology against the dense runner on
a 1-expert-degenerate configuration. Gate: test parity with `moe_block_forward`
`PH-403b` at 1e-5; no real model, no token claim.

### Lane B — owner-gated execution (do not start without explicit go)
**B1 = PH-406:** wire A1 into `moe_layer_forward_cached`'s `load` closure,
real GGUF/pack tensors, `ExpertCache`; needs disk/RAM gate. Gate: real-tensor
canary vs GSV hub golden `< 1e-4`.
**B2 = PH-1206:** qwen35moe runner composing q35/gdn + routed+shared MoE FFN +
norm/residual + embed/output head + KV/GDN state; first argmax token. Gate:
greedy parity vs llama.cpp qwen35moe oracle (PH-1305 parity does **not**
transfer).
**B3 = PH-1306:** `arch=="qwen35moe"` dispatch in `rusty serve` + real
`active_experts`; wire-level chat smoke.
**B4 = G13:** owner-approved runtime/quality gate — load/infer smoke,
NaN/Inf, quality eval, parity. Only after green may "runnable full LLM" be
claimed.

### Lane C — parallel vision, never merged into A/B
PH-1202/1203 `src/vit.rs` → PH-1204 trained alignment → PH-1205 real-image
E2E; still bounded by G13. No image→answer claim.

### Universal gates after every code step
`cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`, product tests
exit 0, GSV `loc-audit --stretch-96` (steady 99.68%), `cargo xtask sync`,
one commit in the product repo with `Gsv-Actor/Gsv-Ide/Gsv-Model` trailers,
push last. No quality wording anywhere before G13.

## 5. Immediate next actions (concrete)

1. Implement A1 (pure expert-range loader) as a ticket in rusty-engine with
   synthetic TDD — no node needed.
2. Reconcile `docs/roadmap.md` PH-405/407 checkboxes (done in this commit).
3. Record the GSV gate honestly: `cargo test --lib` 609/0 green; full-suite
   rebuild blocked by the daemon-locked `gsv-xtask.exe`, and the isolated
   `CARGO_TARGET_DIR` workaround trips two non-hermetic watchdog tests.
   Optionally fix those two tests to be env-hermetic (separate small ticket).
4. Ask owner explicitly for B1/B2 go + node; until then keep A0/A1/A2 green.
   Do not attempt BENCH-2 or a second pack.

## 6. Documentation drift fixed here
- rusty-engine `docs/roadmap.md`: PH-406 marked owner-gated with the real
  hook (`moe.rs::moe_layer_forward_cached`), PH-407 marked superseded with the
  measured synthetic A/B and owner decision. PH-404a noted in HANDOFF.
- GSV HANDOFF/NEXT carry this plan as the canonical next-step reference.

Evidence for the research half: file:line references above (read-only source
audit) plus today's gates in §1. No weights were loaded, no model was run,
no token was generated. Ticket VERS-PLAN done.
