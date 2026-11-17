# Framework Landscape Research — 2026-11-18 (multi-source)

**Date:** 2026-11-18 · **Scope:** які фреймворки/ядра значно прискорюють інференс і dispatch у 2026, і що з них переноситься на стек екосистеми (GSV mesh · rusty-engine · llama-rs · telenetis · WebGPU page-agent). **No code is changed here.** Companion: [`AUTONOMOUS_RESERVE_2026-11-18.md`](AUTONOMOUS_RESERVE_2026-11-18.md).

Залучено: vLLM / SGLang / llm-d 1.0 / Ray Serve LLM / KubeRay · llama.cpp (+ WebGPU) · MLC WebLLM / transformers.js / wllama · mistral.rs / Candle / Burn · speculative decoding (EAGLE-3/P-EAGLE/MTP) · WASM SIMD/relaxed-SIMD · MoE kernels (GroupedGEMM/FlashInfer/DeepGEMM).

---

## 1. TL;DR — що *значно* прискорює, у порядку сили ефекту

| # | Техніка | Виміряний ефект | Джерело |
|---|---|---|---|
| 1 | **Speculative decoding EAGLE-3** (draft head, α≈0.7–0.8) | **3–4×** (H100/H200); до 6.5× на коді; P-EAGLE (Amazon, B200) ще **+1.69×** над EAGLE-3; MTP у Qwen3.6-27B дає 1.38×@N=3 | vLLM/Spheron/AWS/Morph |
| 2 | **Browser WebGPU inference maturity** (WebLLM / llama.cpp-WebGPU / wllama) | 41–51 tok/s Llama-3.1-8B q4f16 на M3 Max (**~71%** нативного MLC); **66.8 tok/s** M3 Pro; WeInfer **3.76×** vs WebLLM (buffer reuse + async pipelines); WebLLM 2.4× vs torch-webgpu | arXiv 2412.15803 · WebLLM · 2605.20706 · 2604.02344 |
| 3 | **WASM relaxed-SIMD** (llama.cpp PR #19590) | **~2×** над 128-bit SIMD (який сам **12.5×** над scalar) | llama.cpp · Chow 2026 (NTU) |
| 4 | **Prefix-cache reuse** (SGLang RadixAttention) | **20–40%** нижчий TTFT на prefix-heavy (multi-turn agents, RAG); cache hit 52–74% у Arena | SGLang · Spheron 2026 |
| 5 | **MoE GroupedGEMM / FlashInfer FP8 / DeepGEMM** (vLLM/SGLang) | MoE активація ≈ 6–13% параметрів; packed **GroupGEMM** — стандарт; FP8+FP8-KV дає найдешевший токен | vLLM · AMD MoE Playbook · DeepGEMM |
| 6 | **llama.cpp WebGPU офіційний backend** (2026, Gerganov) | In-browser GGUF — **будь-яка** модель llama.cpp з GPU; WGSL matmul + FlashAttention kernels; worker + model cache | 2605.20706 · ggml-org · wllama |
| 7 | **Rust ML inference neutrality** (mistral.rs/Candle) | mistral.rs 0.7 на crates.io: multimodal · UQFF · ISQ · competitive з llama.cpp Q8 prefill/decode | mistral.rs · Medium/RedLineSoft |
| 8 | **JIT-specialized kernels** (dasLLAMA example) | claims 1.2–2.8× над llama.cpp CPU/Metal через auto-tuned JIT; marketing-рівень, не перевірено — **watch, don't adopt** | dasLLAMA |

## 2. Стан 2026 за напрямами (деталі)

### 2.1 Серверні рушії (для контролю baseline)

- **vLLM V1**: RayExecutorV2 default (vLLM 0.21+); async scheduling; prefix caching full; P/D disaggregation; WideEP (phi-tiny-MoE 4P4D бив vllm-router, TPOT 13.6 vs 14.8 ms @256).
- **SGLang**: RadixAttention; prefix-heavy виграш; MoE-throughput within 10% vLLM; часто **перший** на  нові архітектури (DeepSeek-папери).
- **TensorRT-LLM**: найнижчий inter-token latency, найповільніший feature cadence.
- **llm-d 1.0 (CNCF)**: WideEP GA — router → **per DP rank**, `DisaggregatedSet/LeaderWorkerSet`, DeepEP/MoRI/RDMA all-to-all 32×H200 NVL72; GAIE CRDs; E2E CI nightlies.
- **Ray Serve LLM**: HAProxy (C) ingress + **Direct Streaming** (decouple routing control-plane from SSE data-plane), 4.4× prefill / 24× decode throughput vs попередні (Ray 2.56 + vLLM 0.22).
- **KubeRay/Kubernetes**: `LeaderWorkerSet` для multi-node MoE; DRA (Device Resource Allocation); PodDisruptionBudgets; HPA за чергою, не за CPU.

**Висновок для магістралі:** наш hub (GSV) — це нескінченна живить суміш dispatch + mesh; server-engines конкуренти нам не є. Але **predicted-latency routing** (llm-d) ми вже імплементували (R1) — аналог fit confirmed.

### 2.2 Браузер / WebGPU (наш флот!)

- **llama.cpp WebGPU** — тепер офіційно у ggml (Gerganov 2026-05). WGSL: matmul + **FlashAttention**; працює з будь-якою моделлю llama.cpp; wllama (worker + cache). Для **нашого peer-агента** це окреслює напрямок: не ми одні з WebGPU матmul — екосистема дозріла до повних моделей у вкладці.
- **WebLLM** (MLC): 41–51 tok/s Llama-3.1-8B q4f16 на M3 Max; малi моделі — 60+ tok/s M3 Pro; Chrome-оптимізовано; quota 1 GB maxBufferSize (адаптер відмовляє → 256/128 MB fallback) — для наших телефонів важливо (ARM Mali/Apple GPU — свій `maxBufferSize`; enrich телеметрію якщо буде peer fallback).
- **WeInfer (WWW'25)**: 3.76× vs WebLLM — **buffer reuse + async pipelines**, без зміни моделі. Прямий рецепт, що варто спробувати у наш WGSL matvec (див. FW-1).
- **WebGPU dispatch overhead study (arXiv 2604.02344)**: вимірювання 4 вендори × 3 браузери; dispatch cost ≠ 0 — на малих matmul-dispatches overhead домінує → **batch/fusion виграш** (FW-1/FW-2).
- **Dispatch-уровневий висновок**: для 4 MiB tensors інференс через WebGPU — швидший за CPU, але Chrome WebGPU suspend (наш FF-PC ghost) — **архітектурно відомий, але не специфічний для нас дефект**; silent device.lost — канонічна причина (FW-4).

### 2.3 WASM / SIMD

- **WASM relaxed-SIMD** ~2× поверх SIMD 128-bit (12.5× над scalar); llama.cpp PR #19590 (dot-product).
- **Rust `wide`/`pulp`**: portable SIMD traits — стабільні; `std::simd` nightly; вручну написаний WGSL у peer-агенті — єдиний лишився hot path, де SIMD-техніки ще не застосовані (FW-3).

### 2.4 Rust ML рушії (для rusty-engine / llama-rs)

- **mistral.rs 0.7**: `mistralrs-serve`; multimodal; **UQFF** format; **ISQ** (instant quantization on-fly); SMQ (super-quant); бенчи: UQFF q8 ≈ llama.cpp Q8_0 prefill/decode на паритеті; BF16 vs vLLM BF16 — номінально.
- **Candle** (HF): GGUF/HF-safetensors нативно; inference-first; 1B quants на CPU прийнятно; 2.1 ms vision (p50).
- **Burn**: повний DL toolkit (training), backend-portable; inference трохи повільніше за Candle.
- **Turboquant-rs**: quant w/o overhead; niche.
- **Atenia / від-raw runtime'и**: show-and-tell; не бекенди для прод.

**Висновок:** значного Rust-native драйвера для MiniCPM-35B-A3B, якого не маємо в rusty-engine/llama-rs, немає; **ISQ/UQFF** — єдина інакше-перевірена польза (Gemma-3-4B в rusty-engine для edge testing). Не брати як код сьогодні.

### 2.5 Спекулятивне декодування

- EAGLE-3 α≈0.7–0.8 → 2–6.5× залежно від домену (код краще за MT-Bench).
- **P-EAGLE** (Amazon, B200, vLLM): K=7 parallel drafting, 1.69× над EAGLE-3, +30–31% над HumanEval, +31% SPEED-Bench, +13% MT-Bench.
- **MTP** (DeepSeek/Qwen3.6-27B): 1.38×@N=3, 1.05×@N=7, 0.95×@N=15 — скорочення returns.
- **Визначення**: працює тільки з великою batch-size моделлю **плюс** draft head; **на телефонах не застосовується**; найближчий consumer — серверний serve path (rusty-serve orchestrator?), не mesh.

### 2.6 MoE kernels

- **GroupedGEMM** (DeepGEMM/FlashInfer/CUTLASS): стандарт; токени на експерта пакуються; FP8 MXFP4/MXFP8 на B200.
- **activation density**: <1% → EP=0; >3% → EP=1 (AMD MoE Playbook); DeepSeek 3.13% — EP=1; MiniCPM-35B-A3B (8/256 MoE) ≈ 3.1% — **на межі; EP=1 виправданий для throughput-баксових пулах**.
- **shared experts** (MiniCPM shexp): завжди активні; **ніколи** не EP-sharded; replicated/linear-sharded — канон.

---

## 3. Що з цього збігається з нашою екосистемою (мапа)

| Наш компонент | Фреймворк сьогодні | 2026 migration candidate | Owner gate? |
|---|---|---|---|
| GSV hub (axum/tokio) | пул dispatch, `route_peers` EWMA | — | — |
| Peer WebGPU page (`wgpu.rs::page()`) | **hand-written WGSL**, cuda-hand kernels | **WeInfer-патерни** (buffer reuse, async pipelines) + WeInfer Fusion (fused SiLU/dequant) · **wllama-style worker + precache** | ні |
| Peer WebGPU device lifecycle | none (silent) | **`device.lost` handling + auto re-join**; FF-PC-class ghost явність | ні |
| Peer capabilities | cores/memory/UA | + `features` (shader-f16, subgroups, maxBufferMB), `device.lost` counter → hub route-hint tier; FF-PC resume behavior **explicitly measurable** | ні (additive join field) |
| llama.cpp Vulkan backend | не використовується в hub | llama.cpp-WebGPU — один із конкуруючих шляхів 35B у вкладці; **RFC-level**, не код | owner |
| rusty-engine | Rust + GGUF MiniCPM-V1B file serve | UQFF/ISQ mistral.rs-path для Gemma-3-4B **benchmark-only**; DeepSeek MTP draft — нетиповий для 1B | owner |
| mesh dispatch correctness | golden-x + max_err_model | — (фаворит-індустрія цього не має; KEEP) | — |
| stand-smoke post-deploy | 87 checks, no page markers | + card `/wgpu` page assert (IDB markers + read-before-fetch) — авто-захист R3 | ні |
| bench harness (`gsv-dev`) | no `weights_matvec` bench op | add `weights` op benchmark + report into `docs/…BENCHMARK` | ні |
| spec decode / eagle / MTP | — | no consumer today | owner |

---

## 4. Висновки (3 головні)

1. **Наш WebGPU peer-агент — не відстає від індустрії, але має відомий клас дефектів, які індустрія вже вирішує стандартно** (WeInfer buffer reuse; silent `device.lost`; 16 MiB chunk streaming без IDB — вже вирішено R3; FlashAttention/UQFF-формат — не потрібні до повних моделей у вкладці). Усі ці — **autonomous fixes** без owner gates.
2. **Спекулятивне декодування і EP — поза нашими constraints** (телефони, GPU-профілі, 40-шаровий forward у вкладці не існує); відкласти.
3. **llama.cpp WebGPU** — стратегічний signal: коли завгодно великий forward у вкладці — є готовий стек; для спостереження; не замінювати нашу page-агент, вона інтегрована з hub dispatch + verification.

## 5. Джерела (validated 2026-11-18)

1. vLLM spec-decode / P-EAGLE (AWS) / Spheron Eagle-3 numbers
2. SGLang 2026 comparisons (Spheron · morphllm · tensormesh · jarvislabs · Yobitel)
3. llm-d Wide-EP (CNCF) / Ray Serve LLM direct-streaming (Anyscale 2026-06) / Qwen3-0.6B + Phi-tiny-MoE WideEP results
4. llama.cpp WebGPU backend + wllama + Llamas-on-the-Web paper (arXiv 2605.20706) · PR #19590 relaxed-SIMD
5. WeInfer (WWW'25, ACM 3696410.3714553) · WebLLM arXiv 2412.15803 · WebGPU dispatch overhead arXiv 2604.02344
6. mistral.rs 0.7 (GitHub benches) · Candle vs Burn (rust4ai) · RedLineSoft Candle post
7. AMD ROCm vLLM MoE Playbook · DeepSeek-R1 FP8/EP=1 rules · activation density table
8. cabo-parallel task framework comparisons (Exo/Ray/vllm-router/llm-d predicted latency)
