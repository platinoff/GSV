# GSV Speed Research — 2026-11 (ECO-6.4 first pass)

Source: web-research pass 2026-11-06 (docs.rs / GitHub / blogs). Owner ask: "web-research last speed frameworks, compare what significantly accelerates app performance; create update plans".

## 1. Pure-Rust LLM inference engines (vs llama.cpp FFI / wrapper)

| Engine | GPU | CPU | Server | Notes |
|---|---|---|---|---|
| ALICE-LLM 1.6.0 | wgpu (Metal/Vulkan/DX12), Q4_K/Q5_K/Q6_K/Q8_0/Q1_0 fused matvec, batch-4 speculative (1B draft + 8B verify = 5.89x, 90% accept) | NEON/AVX2/AVX-512, hybrid DeltaNet+Attn | axum + SSE, OpenAI-compatible | Research codebase; llama.cpp FFI overhead negligible at tok/s (0.004–0.04% @ 4 tok/s) |
| Frink | Metal (dense rows 0.98–1.10x vs llama.cpp; tg128 ahead on 8/12), CPU behind (1.41–5.06x behind), MoE parity | CPU/Metal | `frink serve` OpenAI + Anthropic API, no Python | Metal caught llama.cpp & passed; CPU gap = per-op Rayon fork/join vs persistent pool |
| infr | Vulkan-first (coop-matrix GEMM), Metal ~1.3–1.5x llama.cpp, CPU ref | Vulkan/Metal/CPU | `infr serve` axum+SSE, OpenAI /v1 + tool calls | Vulkan-first for no-CUDA-GPU targets; llama.cpp still the default for "run GGUF on my Mac" |

Assessment: pure-Rust engines are past llama.cpp on Metal (Frink) and approaching on GPU; llama.cpp remains the pragmatic default. FFI overhead at current tok/s is negligible.

## 2. Async runtime / serving stack

- Tokio: canonical (150M+ downloads/month, axum/reqwest/sqlx ecosystem lock-in). Recommendation stands.
- async-std: discontinued (2025). smol: minimal/composable, not for mainstream HTTP.
- Compute-bound inference: `tokio::task::spawn_blocking` + dedicated blocking pool (or Rayon for engine-internal parallelism) — never block async workers.
- Benchmarks: no-runtime Rust (mio, 2-thread) ~3900 sessions/vCPU beats C++ tier; async Axum/Tokio 3300 — best balance for LOC/perf.

## 3. Update plans (ECO-6.4 → ECO-2 bands follow-ups)

- gsv: tokio/axum chain on latest (done in ECO-2 band gsv); keep spawn_blocking discipline.
- gsv-wgpu: wgpu 30 + pure-Rust GPU path candidates (ALICE-LLM-style) — evaluate post-G2.
- llama-rs: llama-cpp-2 0.1.156 band open (B-3) — FFI overhead negligible, no engine swap forced by perf.

## 4. Next research tracks (ECO-6.4 sub-items)

## 5. Latest forks / MoE experimental-4bit research (2026-10/11 deep pass)

Sources: GitHub ggml-org/llama.cpp PRs + discussions, oMLX benchmarks, NVIDIA Model-Optimizer, fusion-mlx, MiLo.

- PR #31188 (merged 26.10): CUDA fuse MoE gate/up + GLU in MMQ — +5.7~17% tok/s on MoE Q4_K_M (DGX Spark, RTX 5090).
- Fork vahpetr/llama.cpp-s1-fork (Strix Halo/Vulkan, Qwen3.8-Flash-Next 177B MoE): ~190 patches; prefill 550→474 t/s @8k, cold fill 64→44 min; GPU mem +0.8 GiB.
- PR #30112 (merged 8.10): MoE expert cache over multiple GPUs (2x4090): decode 60→64 t/s.
- PR #31291: FP4 E2M1 KV cache (RTX 5090/Blackwell) — closed w/o GPU access (4x KV compression vs FP16).
- PR #31497: speculative — do not apply MoE cache to draft (merged; segfault fixed).
- NVIDIA Model-Optimizer: GLM-5.3-Flash MoE (321B; 304B routed experts) at IQ2_S PTQ / QAD / NVFP4 — ~130GB (vs FP8 328GB).
- oMLX: Gemma-4-26B-A4B-it OptiQ-4bit-REAP-14B (M4, 4bit, 26 TG/t/s); Qwen3.5-4B-MLX-4bit (4bit).
- fusion-mlx: MoE shared-expert activation cache (DSA), 4-bit convert; MiLo (LoRC compensators + INT3 kernel).

## 6. Update plans (ECO-6.4 -> ECO-2 bands follow-ups)

- CPU backend replacement (Frink Rayon-pool fix) for llama-rs CPU lane.
- MoE expert-residency runtime (router+KV+memory co-design) — only strong reason to write runtime from scratch.

Next sub-tickets: ECO-6.5 (dedup), ECO-6.6 (updates+tests/push), ECO-6.7 (eco-format+txt cleanup+MCP), ECO-6.8 (ripgrep).
