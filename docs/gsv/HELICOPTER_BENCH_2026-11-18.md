# Helicopter Codegen Benchmark — 2026-11-18

**Prompt:** «Write a tiny helicopter game in a single HTML file: canvas, SPACE/tap to flap, red helicopter, moving pipe obstacles, score counter, gravity». · **Popular LLM benchmark class** (single-file game generation).

## Results

| Runner | Build | Tokens | Wall | tok/s | Output |
|---|---|---:|---:|---:|---|
| Qwen2.5-0.5B-Instruct-Q4_K_M via `llama_serve` (`/s/rust/llama-rs`) | debug | 461 | 229.6 s | **2.01** | CSS-only, model stalled mid-tag at 461 tok, `finish_reason=stop`, **no JS at all** — codegen FAIL |
| Same, rebuilt `cargo build --release` | release | 450 | 232.1 s | **1.94** | Same failure mode (model stops mid-CSS) — identical single-threaded throughput |
| MiniCPM-V-4_7-1B-Q4_K_M via `rusty serve` | debug | — | >900 s | — | No response after 15 min; killed — pure-Rust debug engine is not usable for this benchmark on this box |
| MiniCPM-V-4_7-1B-Q4_K_M via `rusty serve` (`rusty 0.1.0`, release, mmap) | **release (shipped)** | 514 | **34.4 s** | **~14.9 tok/s effective** | Emits `<think>` preamble then partial skeleton — **non-playable**: `canvas` + `script` + broken API call (`getContext('drawContext')`), no game loop, no pipes, no gravity, `</html>` never reached |

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

**Verdict.** Fastest real run on this box is **MiniCPM-V-4_7-1B release @ ~14.9 tok/s** (514 tok / 34.4 s), and its output is an inert mouse-trap: `think` preamble, broken `getContext('drawContext')`, zero game logic — still a FAIL. The peers' speed class (`weights_matvec` shards at 9–176 ms) is unrelated to codegen throughput: verification ≠ generation. A green "helicopter" benchmark needs either a bigger quant held in VRAM (Vulkan offload on FF-PC, owner gate) or a ≥3B instruct model — neither fits 7.6 GB RAM here.
