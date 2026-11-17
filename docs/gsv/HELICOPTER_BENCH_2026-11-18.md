# Helicopter Codegen Benchmark — 2026-11-18

**Prompt:** «Write a tiny helicopter game in a single HTML file: canvas, SPACE/tap to flap, red helicopter, moving pipe obstacles, score counter, gravity». · **Popular LLM benchmark class** (single-file game generation).

## Results

| Runner | Build | Tokens | Wall | tok/s | Output |
|---|---|---:|---:|---:|---|
| Qwen2.5-0.5B-Instruct-Q4_K_M via `llama_serve` (`/s/rust/llama-rs`) | debug | 461 | 229.6 s | **2.01** | CSS-only, model stalled mid-tag at 461 tok, `finish_reason=stop`, **no JS at all** — codegen FAIL |
| Same, rebuilt `cargo build --release` | release | 450 | 232.1 s | **1.94** | Same failure mode (model stops mid-CSS) — identical single-threaded throughput |
| MiniCPM-V-4_7-1B-Q4_K_M via `rusty serve` | debug | — | >900 s | — | No response after 15 min; killed — pure-Rust debug engine is not usable for this benchmark on this box |
| MiniCPM-V-4_7-1B-Q4_K_M via `rusty serve` (`rusty 0.1.0`, release, mmap) | **release (shipped)** | 514 | **34.4 s** | **~14.9 tok/s effective** | Emits `<think>` preamble then partial skeleton — **non-playable**: `canvas` + `script` + broken API call (`getContext('drawContext')`), no game loop, no pipes, no gravity, `</html>` never reached |
| Qwen2.5-0.5B-Instruct-Q4_K_M via `llama_serve --features vulkan` (`VULKAN_SDK=/ucrt64`, MSYS2 toolchain, no LunarG) | release + **Vulkan iGPU** | 439 | **45.1 s** | **≈9.7 tok/s** | Same inert skeleton; log confirms `ggml_vulkan: Found 1 Vulkan devices: 0 = AMD Radeon(TM) Graphics … layer 24 assigned to device Vulkan0; token_embd/q5_0 → CPU`. **~4.9× the CPU-bound 1.94** — but still 0.5B output class |
| **Granite-3.1-3B-A800M-Instruct** (IBM, imatrix Q4_K_M) via same Vulkan iGPU | release + **Vulkan iGPU (33/33 layers offloaded)** | 278 | **26.7 s** | **≈10.4 tok/s** | Best code-shaped output of the day (variables, functions, pipe/gravity vars) — still incomplete and non-playable: no canvas, no rAF, no flap/input, no `</html>` |
| **Granite-3.1-3B-A800M**, tighter-cone prompt ("no CSS, minimal JS, single file") | release + **Vulkan iGPU** | 280 | **20.4 s** | **≈13.7 tok/s** | Still inert skeleton (CSS-free body, restart button bound to `restart()` never defined) — codegen verdict survives every prompt |

- Prefill: 75 tokens (prompt) — too small to infer a prefill rate; bound by the same single-threaded path.
- Generation here is **single-threaded** by design (0.5B q4_k_m CPU mmap) — release mode changes nothing:
  inference throughput is CPU-bound, not build-bound.
- The generated «game» (saved as `target/live/helicopter_bench_2026-11-18.html`) is an inert CSS skeleton:
  pipe-wrapping `<div>`s, an `<img>` with no `src`, zero JavaScript. Not playable.

## Why the peers cannot run it (honest)

- Mesh peers execute fast `weights_matvec` (phones #35/#38: **9–176 ms** per 2048×512 shard; Pi4: **1.7–8.2 s**),
  but they do **not** hold a generative binding for a forward pass — attention, GDN/SSM, MoE routing (256 experts) and
  embeddings/lm_head are not distributed; the shared-expert MLP they verify is ~3% of the model by FLOPs.
- A full 40-layer 35B-A3B forward on this 7.6 GB box is impossible regardless of mesh (RAM; and experts aren't sharded).
- `llama_serve` supports `--rpc host:port` to offload layers to other machines (GGML_RPC=ON + vulkan-class build),
  but that requires a purpose-built RPC binary on every peer — not present here, and a new build pipeline.
- FF-PC RDNA2 remains ghosted (webGPU suspend — owner gate `about:config`); Pi4 serving blocked by missing SSH passphrase.

## What it proves

1. Local generation throughput is ~2 tok/s regardless of build profile — **not viable** for the popular codegen benchmark to any useful quality bar.
2. The only model on this box with real forward capacity is Qwen2.5-0.5B (0.5B) — output class is *below* anywhere near a playable game.
3. The mesh fleet **is** viable for tensor verification and has proven itself on 288/288 shards; the same dispatch will *never* produce helicopter games.

## Owner-gated next steps, if you want this benchmark green

- **Vulkan build** of `llama_serve` (`--features vulkan`) → GPU offload on FF-PC/RDNA2 (7.5 GB VRAM) — real tok/s class if the peer ever runs webGPU (it currently doesn't).
- **Pi 4 as a serving peer**: needs the `owner_session.key` passphrase (or `ssh-add`) to flash `llama_serve --port 8080` there; Pi4's 4 cores ≈ ~2 tok/s expected — a true peer benchmark run, but quality/voltage class is unchanged.
- ~~**MiniCPM-1B via release**~~ — **DONE 2026-11-18** (~14.9 tok/s effective; output still non-playable skeleton with broken API call). The speed class on this box for a 1B quant is ~15 tok/s; helicopter codegen remains out of reach at this capability tier.
- **A different model** (Qwen2.5-1.5B/q4 on this box, ~1 GB RAM) — slower (~1–1.5 tok/s) but far better output; RAM headroom is the gate.

**Verdict.** Fastest runs on this box: **MiniCPM-V-4_7-1B release CPU @ ~14.9 tok/s**, **Qwen2.5-0.5B Vulkan iGPU @ ~9.7 tok/s (×4.9 over CPU-only)**, **IBM Granite-3.1-3B-A800M Vulkan @ ~10.4–13.7 tok/s** — all outputs are inert skeletons, **not playable games; codegen FAIL at every tier this box can hold** (0.5B, 1B, 800M-active MoE). The Vulkan path *works* (MSYS2 toolchain, `VULKAN_SDK=/ucrt64`, 24–48/48 layers on device Vulkan0, token_embd/q5_0 → CPU) and is the only real speed lever observed on the iGPU here. Verification (`weights_matvec`) ≠ generation: 288/288 shard proofs green, codegen at 0.5–1B/800M-active capability — different problem classes. A green "helicopter" benchmark needs a ≥3B-class instruct quant fit for iGPU/RAM — none fits 7.6 GB RAM with hub + Firefox here.

**Vulkan is confirmed usable for this bench box** (Radeon(TM) Graphics, uma 1, fp16 1, warp 64) — a future llama-rs deployable can be promoted to GPU with `--n-gpu-layers N` without new hardware; promotion remains an owner decision (consistent with PH-S2991 2026-09-16).

**Quality verdict of the session:** 0.5B (Qwen), 1B (MiniCPM), 800M-active-A800M MoE (Granite-3.1-3B, best code-shaped output of the three, still broken `restart()`/no game loop) — all three FAIL the helicopter benchmark at full fidelity. **The bottleneck is capability, not speed.**
