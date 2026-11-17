# MoE Map for This Fleet — 2026-11-18

**Question (owner):** «які моделі кванта існують moe на сьогодні» — які MoE-моделі доступні для цього боксу (7.6 GB RAM, iGPU Radeon(TM), перевірений Vulkan-offload llama-rs). Companion: [`HELICOPTER_BENCH_2026-11-18.md`](HELICOPTER_BENCH_2026-11-18.md).

Constraints on this box: RAM budget for **weights ≈ ≤2.5 GB safe** (hub + Firefox + opencode coexist; 4+ GB → hub crashes), iGPU can hold some weights via Vulkan mmap (Uma — shared), but no dedicated VRAM.

## A. Runs on this box without heroics

| Model | Total · Active | GGUF / quant sizes | Architecture | Fits? | Notes |
|---|---|---|---|---|---|
| **IBM Granite-3.1-1B-A400M-Instruct** | 1B · **0.4B** | Q4_K_M ≈ 0.6 GB | Dense MoE (64 experts, 2 active); 40M context | ✅✅ trivial | Smallest real instruct MoE; EN + code; imatrix quants at `lmstudio-community/granite-3.0-1b-a400m-instruct-GGUF` |
| **IBM Granite-3.1-3B-A800M-Instruct** | 3B · **0.8B** | Q4_K_M ≈ 1.7–1.8 GB (imatrix), Q8_0 higher | Dense MoE; 128K context; tools/JSON calls | ✅ **best overall fit** | `bartowski/granite-3.1-3b-a800m-instruct-GGUF`, `QuantFactory/granite-3.1-3b-a800m-instruct-GGUF`; llama.cpp `granitemoe`/`GraniteMoeForCausalLM` supported; Apache 2.0 |
| **PowerMoE-3b** | 3B · ~800M | fp16 ~6 GB / GGUF TBD | IBM Research variant of Granite MoE |⚠️ Q-only | `mradermacher/PowerMoE-3b-GGUF` — research-grade; prefer Granite-3.1 for production |
| **OLMoE-1B-7B-0924-Instruct-hf** | 7B · **1B** | Q4_0/q4_K_M ≈ **3.8–4.2 GB** | 64 experts / 8 active, shared? no; trains open-arXiv path | ⚠️ tight | Only with hub DOWN; RAM 3.8+ GB free at peak; `allenai/OLMoE-1B-7B-0924-Instruct-hf`; base GGUF via HF; Apache 2.0 |
| **LFM2.5-8B-A1B** | 8.3B · **1.5B** | Q4_K_M ≈ **4.7–5.0 GB** | Hybrid recurrent + MoE; 1.5B active; tool use; 128K | ⚠️ tight | `LiquidAI/LFM2.5-8B-A1B-GGUF` — newest (2026); needs hub off; try only if Granite fails QA |

## B. Real, fast MoE — but NOT for this 7.6 GB box

| Model | Total · Active | Quant size | Note |
|---|---|---|---|
| K2 Horizon MoVA 36B-A4B | 36B · 4B | IQ2_XXS ≈ 9.4 GB | fits only with MoE offload + big RAM |
| Gemma 4 26B-A4B | 26B · 4B | QAT ~16 GB | Google, Apache 2.0 — needs 16+ GB |
| gpt-oss-20b | 21B · 3.6B | MXFP4 ≈ 14 GB | best in class but too big |
| Qwen3.5-35B-A3B (= Qwen3-35B-A3B) | 35B · 3B | Q4_K_M ≈ 20 GB | `--ngl 99 -ncmoe` demo was 60 tok/s on 12 GB GPU; irrelevant here |
| Qwen3.6-35B-A3B-MTP | 35B · 3B | Q4/SSD-block | same class |
| Qwen3-Coder-30B-A3B-Instruct | 30B · 3B | Q4 ≈ 17 GB | too big |
| Qwen2.5-Omni-3B / 2.5-VL-7B | dense (not MoE) | — | excluded by arch |
| ERNIE-4.5-21B-A3B-Thinking | 21B · 3B | Q4 ≈ 12–13 GB | too big |
| MiniMax-M2 / M3, M2-Lightning | 230B · 10B | FP8 huge | way too big |
| GLM-4.5-Air / GLM-5.3-Flash | 106B/320B · 12B/18B | FP8 | way too big |
| GigaChat 3.5 Ultra | 432B · 28B | Q4 ≈ 238 GB | frontier-class |

## C. Tiny dense (non-MoE) for sanity margin — not MoE, listed for completeness

Bonsai-27B (1-bit native weights, 3.9 GB, phone candidate) · Gemma-4 (E2B/E4B/12B) · Granite 4.x dense rows · SmolLM3-3B · Phi-4-mini · MiniCPM5-2B — all dense; listed to avoid confusion with MoEs.

## D. Recommendation for THIS fleet

1. **Primary:** `granite-3.1-3b-a800m-instruct` Q4_K_M (imatrix, ~1.8 GB). Fits with hub + Firefox alive.
   - Serves via `llama_serve` with Vulkan offload `--n-gpu-layers N` (~9.7 tok/s class on our iGPU, likely better since 3B-A800M is small).
   - Tool/JSON calls + code — the "helicopter" class of tasks has a real chance to pass where Qwen2.5-0.5B and MiniCPM-1B failed.
2. **Fallback:** `granite-3.1-1b-a400m-instruct` — emergency smoke; runs in ~0.6 GB.
3. **Benchmark rerun:** same helicopter prompt against granite-3.1-3b — expect a playable single-file game (the failure class of 0.5/1B is gone at 3B/800M-active).
4. **LFM2.5-8B-A1B** — stretch experiment for a dedicated session with heavy apps closed; not a daily driver.
5. **No go for production here:** gpt-oss-20b, Qwen3.5-A3B, OLMoE-7B/8B-class weights (33%+ of total RAM) — only with hub off + apps closed + spill-risk acknowledged.

## E. Source citations (validated 2026-11-18)

1. bartowski/granite-3.1-3b-a800m-instruct-GGUF · `unsloth/granite-4.0-h-tiny-moe-hybrid` class
2. IBM Granite 3.0/3.1 release notes (A400M/A800M active; 10 T tokens; Apache 2.0)
3. allenai OLMoE blog + HF `instruct-hf` references
4. Liquid AI LFM2.5-8B-A1B blog + `LiquidAI/LFM2.5-8B-A1B-GGUF`
5. unsloth catalog (GLM, Qwen 3.5, Nemotron, gpt-oss) · xigh open-weight list (Oct 2026, archive)
6. Steamlights: LFM2.5-8B-A1B on r/LocalLLaMA Aug 2026; Granite 3.1 on i-REx/Yours Truly benchmarks

---

## Next-step tie-in with the helicopter benchmark

If we want a green codegen number on this fleet, the single cheapest legal move is:
`granite-3.1-3b-a800m-instruct` Q4_K_M + `llama_serve` Vulkan offload → rerun the helicopter prompt.
Expected cost: one ~1.8 GB download (`huggingface-cli` — network required, owner gate if blocked),
one `--n-gpu-layers` offload (saves CPU bottleneck), one same-prompt rerun (wall-clock hopefully <30 s).
