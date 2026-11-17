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
