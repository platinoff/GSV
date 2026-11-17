# Autonomous Reserve — 2026-11-18 (adapted plan)

**What:** adapted framework-landscape findings ([`FRAMEWORK_LANDSCAPE_RESEARCH_2026-11-18.md`](FRAMEWORK_LANDSCAPE_RESEARCH_2026-11-18.md)) → concrete reserve of work **without owner gates**, in execution order. Consumed in any `абракадабра`/`agi` drain session.

**Rules of the reserve:** one item = one focused slice; each slice = its own commit; any slice that needs wire-protocol change or a new dependency parks back here with a note, never improvised inline.

---

## Execution order

### FW-1 (S) — stand-smoke lock for the peer page (R3 protection)

- **What:** extend `gsv-http-stand-smoke` (CARDS → page checks) or a new `cargo xtask stand-smoke --check-wgpu`: assert `/wgpu` contains `indexedDB`, `gsv-mesh-weights`, `idbGet(c)` **before** `api/wgpu/weights/chunk`, `IDB_CAP`, `const CACHE={}`.
- **Why:** R3 peer cache was verified live once — prevent silent regression from future page edits.
- **Effort:** 1 h · **Risk:** none · **Depends:** nothing.

### FW-2 (S–M) — WebGPU `device.lost` handling + auto re-join

- **What:** peer page JS: `device.lost.then(info => …)` → (a) mark all in-flight tasks as `failed` with reason, (b) teardown pipelines, (c) re-`peer_join` + poll loop resumes; (d) new optional `device_lost_count` in `peer_join` body + JSONL (`PEERS_FILE`) + `/sys` peer row; (e) hub demotes peers with `lost>0` in last 10 min out of the heavyweight route pool *for one lease* (👻 protection, beyond R2 proofs).
- **Why:** FF-PC ghost refusal to serve MiniCPM is exactly this class (silent loss/suspend); today invisible. Industry standard: `adapter.instance lost` + health re-probe.
- **Effort:** 4–6 h · **Risk:** low–medium (page JS + additive join field; hub ignores unknown keys).
- **Depends:** FW-1 (regression lock first).

### FW-3 (S) — GSV bench: `weights_matvec` reference path

- **What:** add bench op `weights` in `benches/gsv_dev.rs` measuring hub-side CPU reference `matvec` + `owner reference check cost` (the bottleneck of every mesh dispatch verify path); `cargo xtask record-speed` re-baseline.
- **Why:** every peer proof eventually lands against this CPU path (524 288 FLOPs/iter × 4 shards/tensor) — first measured profile of the verify path; today it's invisible in speed index.
- **Effort:** 2 h · **Risk:** none.

### FW-4 (M) — Peer `features` probe + capability tier in mesh routing

- **What:** peer page JS join body gains `features: ['shader-f16','subgroups','maxBufferMB:2048', …]` (additive, bounded by hub); hub: stored in peers JSONL; `/sys` shows it; `ranked_peers` tier bonus for `shader-f16` (q4f16/weight decode potential) and `subgroups` (warp-level matvec) — route ordering only, gate unchanged.
- **Why:** Mali/Adreno/Apple-GPU f16 support is the single biggest WebGPU lever for `weights_matvec` math; zero risk to correctness (golden-x keeps err small) — routing-only hint.
- **Effort:** 4–6 h · **Risk:** medium (page + hub telemetry + ranking tier).
- **Depends:** FW-2 (lost counters inform tier), FW-1.

### FW-5 (L) — WeInfer-class page-kernel optimization — **ANALYSED & PARKED** (2026-11-18)

- **What the page already does** (verified against `wgpu.rs::page()`): per-op runner caching (`getRunner`/`CACHE`), **single compute dispatch per op** (`mlpblock` is one fused WGSL kernel with SiLU inline — `WGL_BLOCK`), per-session GPU buffers (no per-task allocation), weights tensor cached in WASM/WebGPU per session, **R3 persistent chunk cache** (IndexedDB), WASM SIMD128 fallback (`runWeightsMatvecWasm`), full wasm/JS cross-check (`wasmRef*`).
- **Remaining theoretical wins and why they don't fit this granularity:**
  1. *Command-buffer batching across a block* — tasks arrive as singletons from the hub; cross-task batching **is** R4 (owner gate).
  2. *SiLU fusion everywhere* — already done inside the block kernel (`WGL_BLOCK`).
  3. *Buffer reuse / async pipelines* — buffers ARE reused via `CACHE`; sub-millisecond wins on this problem scale.
- **Decision: build nothing.** The WWW'25 WeInfer gap (3.76×) applied to a full-model browser runtime, not to an op-per-task dispatched proof-gated mesh. Revisit only after R4 lands.
- **Effort:** analysis only · **Risk:** n/a

### FW-6 (S) — Autonomous-reserve bookkeeping

- **What:** this file + the research doc maintained; each completed item moves to a `Done` section with commit ref; new items discovered during drains are appended with ID `FW-n`.
- **Effort:** ongoing · **Risk:** none.

---

## Done (moved out of reserve)

| ID | What | Commit |
|---|---|---|
| (research baseline) | R1 routing · R2 stale-proof gate · R3 IndexedDB chunk cache · R7 percentiles · R5 parked | `3ba2353`…`4148823` |
| FW-1 | stand-smoke `wgpu_page_r3` (88 checks, live `wgpu_page_r3=ok`) | `a3d0810` |
| FW-2 | `device.lost` watch + `device_lost_count/reason` in poll body → hub peer row + /sys pill + history JSONL | `a3d0810` |
| FW-3 | bench op `weights_matvec_reference` (4 MiB synthetic tensor in `gsv_dev`); verify path measures **6.13 ms/iter** (parse-dominant) — candidate for memoization cache later | bench committed `a3d0810` cycle |
| FW-4 | page `features` probe (shader-f16/subgroups/maxBufferMB) → `device_status` → /sys + `ranked_peers` in-tier f16 nudge | `c7fff7c` |
| FW-5 | **PARKED** — analysis only: page already does per-session runner + buffer caching, single-fused-dispatch kernels, SiLU inline, R3 persistence, WASM fallback | `AUTONOMOUS_RESERVE` §FW-5 |

---

## Not in reserve / owner-gated (for clarity)

- **R4** multi-tensor `weights_matvec_block` envelope — wire change, owner.
- **R8** FF-PC `about:config` — owner.
- **Vision rev mirror** (516 vs 499) · **ECO-8.2** llama-cpp-sys bump · **mesh-node remote** · **Qwen3.8-27B GDN/SSM** — owner.
- **Speculative decoding (EAGLE-3/P-EAGLE/MTP)** — no consumer; server-only; owner when rusty serve ≥ 35B.
- **llama.cpp WebGPU backend adoption** — RFC first; changing the 35B peer path is an architecture decision; owner.

## Effort vs payoff envelope

| ID | Effort | Payoff | Solo? |
|---|---|---|---|
| FW-1 | S | regression-lock | ✅ |
| FW-2 | S–M | ghost root-cause visibility | ✅ |
| FW-3 | S | measure verify path | ✅ |
| FW-4 | M | routing signal (f16/subgroups) | ✅ |
| FW-5 | L | 2×+ peer throughput | ✅ (with rollback discipline) |
| FW-6 | ongoing | canon hygiene | ✅ |
