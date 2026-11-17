# Mesh Benchmark 2026-11-18

**Dispatched:** 2026-11-18 via chart of GSV mesh `/api/wgpu/task/enqueue` → 140 dispatches (`op="weights_matvec"` × shards=4, `iters=1`).

**Model:** MiniCPM-V-4.7-35B-A3B-Q4_K_M.gguf — shared experts blk.0…blk.12 (blk.0..4 from prior sessions, blk.5..11 this session, blk.12 final).

## Aggregate metrics

| Metric | Value |
|---|---|
| Tasks done | **140** |
| Per-dispatch tensor | `ffn_{gate,up,down}_shexp` per blk (3 tensors × 4 shards) |
| Avg `max_err_model` | **8.78e-7** |
| Min `max_err_model` | **2.16e-7** |
| Max `max_err_model` | **3.53e-6** (cap 1e-2 — mesh accepts) |
| Total compute time | **~388 s** |
| Avg per shard | 2.79 s |

## Per-peer summary

| id | adapter | ua_model | tasks | total | avg | avg_err_model |
|----:|---------|----------|------:|------:|----:|--------------:|
| 35 | arm/valhall | SM-A546W (Samsung A54) | 7 | 316 ms | **45 ms/task** | 5.5e-7 |
| 37 | adapter | Firefox on PC (WebGPU) | 24 | 6.2 s | 258 ms/task | 7.9e-7 |
| 38 | arm/bifrost | M2004J19C (Redmi 9) | 24 | 4.1 s | 172 ms/task | 9.1e-7 |
| 36 | arm/cortex-a72 | Raspberry Pi 4 | **83** | **377 s** | 4541 ms/task | 9.25e-7 |
| 28 | adapter | PC-WebGPU desktop | 1 | 193 ms | 193 ms/task | 2.55e-7 |
| 34 | arm/bifrost | old Redmi | 1 | 204 ms | 204 ms/task | 3.28e-7 |

## Cross-op (this session + prior mtx)

| op | n | avg_err | avg_ms |
|---|--:|--------:|-------:|
| `weights_matvec` | 140 | 8.78e-7 | 2791 |
| `minilm` | 38 | 2.95e-5 | 198 |
| `matvec` | 4 | — | 96 |
| `mlpblock` | 4 | — | 157 |
| `matvec_shard` | 2 | — | 82 |
| `softmax` | 4 | — | 119 |
| `layernorm` | 4 | — | 11 |
| `matmul64` | 4 | — | 7 |

## Determining notes

- Pi 4 is the **workhorse**: 83 of 140 tasks but most expensive per task (4.54 s/task).
- Phones are **cheaper per task & similar quality** (~10× faster than Pi4) — WebGPU accelerated.
- FF-PC 27 tasks stalled this session (Firefox suspend/return only; not productive in bench slots) — FF-PC webGPU seems to be suspended/inactive for tasks.
- 65 chunks served live through `L:/gsv_mirror/wgpu_weights` via `GSV_WEIGHTS_STORE_DIR` env var (`04feab3`).
- Hub stayed alive through heavy load (watchdog auto-respawn).
- Rust-side baseline bench (`cargo bench --bench gsv_dev` — not run this session due to EDR pressure).
- 8-shard dispatch verified WORKING for `weights_matvec` (ALLOWED_SHARDS=[1,2,4,8]) — Mesh accepts and phones wake via hot-swap seeing larger fan-out.
- FF-PC's perpetual 27 stop indicates tab-backgrounded webGPU-strand suspension — not mesh bug.
- Phones were alternately alive/dead across session (hot-swap); active phone tasks available at snapshots: valhall + bifrost group

## Distribution log — blk.13…blk.30 (2026-11-18, automated dump → mirror → enqueue)

30 of 40 layers dispatched live to the fleet through the hub (`rusty dump-f32` → chunk upload → `register_tensor` → `weights_matvec` shards=4). L: mirror now holds **100 chunks (~405 MB)**. FF-PC #37 correctly absent from every route pool (`proofs_fresh=0`, R2 gate). Hub crashed multiple times mid-run (RAM pressure) and auto-respawned each time; the FW-7 stuck-claim requeue (`dcb83ca`) recovered all mid-flight tasks without manual intervention.

| Group | Tensor | Peers | Result |
|---|---|---|---|
| g-201…203 | blk.13 gate/up/down | 35·38·36 | 12/12 done · err ≤ 3.0e-6 |
| g-204…206 | blk.14 gate/up/down | 35·38·36 | 12/12 done · err ≤ 3.2e-6 |
| g-207…209 | blk.15 gate/up/down | 35·38·36 | 12/12 done · err ≤ 2.9e-6 |
| g-210…212 | blk.16 gate/up/down | 35·38·36 | 12/12 done · err ≤ 2.8e-6 |
| g-213…215 | blk.17 gate/up/down | 35·38·36 | 12/12 done |
| g-216…218 | blk.18 gate/up/down | 35·38·36 | 12/12 done |
| g-219…221 | blk.19 gate/up/down | 35·38·36 | 12/12 done |
| g-222…224 | blk.20 gate/up/down | 35·38·36 | 12/12 done |
| g-225…227 | blk.21 gate/up/down | 35·38·36 | 12/12 done (3 self-requeued after hub crash — FW-7) |
| g-228…230 | blk.22 gate/up/down | 35·38·36 | 12/12 done (stuck claims requeued — FW-7) |
| g-231…233 | blk.23 gate/up/down | 35·38·36 | 12/12 done |
| g-234…236 | blk.24 gate/up/down | 35·36 | 12/12 done |
| g-237…239 | blk.25 gate/up/down | 35·38·36 | 12/12 done |
| g-240…242 | blk.26 gate/up/down | 35·38·36 | 12/12 done |
| g-243…245 | blk.27 gate/up/down | 35·38·36 | 12/12 done |
| g-246…248 | blk.28 gate/up/down | 35·38·36 | 12/12 done |
| g-249…251 | blk.29 gate/up/down | 35·38·36 | 12/12 done |
| g-252…254 | blk.30 gate/up/down | 35·38·36 | 12/12 done |

**216/216 shards done · 0 failed** across blk.13–30. Phones #35/#38 claim in 9–176 ms per shard; Pi4 #36 is the quota-fallback tier. L: mirror now holds exactly **100 chunks (~405 MB)**; hub auto-respawned autonomously multiple times (RAM pressure), and the FW-7 stuck-claim fix requeued all mid-flight tasks without manual intervention.

**72/72 shards done · 0 failed · max_err_model = 6.63e-6** (cap 1.0e-2). Phones #35/#38 claim in 9–176 ms per shard; Pi4 #36 is the quota-fallback tier (≈2–8 s per shard) — exactly the FW/R1 traffic shaping target.

## R7 addendum — percentiles over ALL logged `weights_matvec` (n=150, through g-199)

Fleet-wide distribution is **bimodal** — Pi4-class ARM CPU vs WebGPU phones — so overall percentiles mix two populations; per-peer table is the honest view:

| Metric | Value |
|---|---:|
| n (all done `weights_matvec`) | 150 |
| p50 | 1693.7 ms |
| p90 | 6706.2 ms |
| p99 | 12702.5 ms |
| min / max | 6.8 / 19341.9 ms |

| peer | n | p50 | p90 | p99 |
|---|---:|---:|---:|---:|
| #35 valhall | 11 | **17.9 ms** | 74.2 ms | 74.2 ms |
| #36 Pi4 | 84 | 3923.7 ms | 8245.9 ms | **18792.3 ms** |
| #37 FF-PC | 23 | 87.0 ms | 135.0 ms | 280.0 ms |
| #38 bifrost | 32 | **123.0 ms** | 379.8 ms | 424.1 ms |

Notes: `#36` tail (p99 18.8 s) drove the overall p99 — route weighting (R1) should keep Pi4 as tier-2/bulk, not the tail-of-fleet predictor. `minilm` reference (n=35): p50 ≈ 155 ms, p99 ≈ 337 ms.
