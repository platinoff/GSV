# GSV_RESEARCH_RUST_GPU_EDGE.md — прискорення стека: сервер / GPU / edge (2026-09-15)

Ресерч на запит власника: «останні Rust-фреймворки, які значно прискорять
програму (un-bottleneck), webgpu, wasm, grid-використання poolAI» + режим
транспортів telenetis (LAN full / ngrok view-only). Флагмани перевірені на
2026-06..09 джерела; вердикти під кожен шар наші, не маркетингові.

## 1. Серверний web-шар (gsv-server :9999, telenetis :9800, poolai :8091 — axum/tokio)

Джерела: ishtms/rust-framework-bench (rewrk, release); rust-lang forum
2026-01 (fuji-184 hello-world 8-thread бенч); actix-vs-axum 2025-10.

| Фреймворк | req/s (hello) | Примітка |
|---|---|---|
| Viz / **Axum** / Poem | ~182–184k | axum у топ-3; tokio-stack |
| Actix-web | ~154–166k | повільніший на heavy |
| ntex (tokio) | ~159k | на par з actix |
| ntex + **neon-uring** | 23–25k проти 17–18k axum (single-thread) | **лише Linux io-uring** |

**Вердикт:** міняти axum на ntex на **Windows-хості сенсу немає** — io-uring
перевага недоступна (monoio/compio-клас теж Linux-only), а на our scale
(≤десятки SSE/ws-клієнтів) HTTP-фреймворк не є бутлнеком взагалі. Реальні
безкоштовні виграші: `tower-http` **br/gzip** для /api/* JSON (у telenetis вже
є limit-шар; додати compression), ETag на snapshot, менші payload (див. §4).

## 2. GPU без нової RAM: llama.cpp **Vulkan** на Vega 7 iGPU хоста

llama.cpp тримає Vulkan-backend з ролю «AMSoC/hybrid inference» — Vega 7
(Radeon Graphics на 5700U) ділить ту самі 7.4GB, ALЕ має ~2× проточну
пропускну здатність RAM на ваги, ніж CPU-літі. Орієнтир з бенч-сцени
2026: Lunar Lake iGPU pp=20–25 tok/s для 7–8B Q8; Adreno X1-85 wgpu-llm
INT8 32.8 tok/s на TinyLlama (Vulkan!) — ARM-iGPU-клас живий. Для нас це
** spike, не rewrite**: пере-збірка llama.cpp з `-DGGML_VULKAN=ON` всередині
llama-cpp-sys (CMake define), `llama_speed` tg-дельта на (a) 1.5B fast-tier,
(b) 30B-A3B IQ2_XXS (MoE-експерти лишаються на CPU — Vulkan бере attention/
shared). Сумісний наш патченний 0.1.154 build. Очікування: :8082 tg ×1.5–3;
deep-tier уваг: shared-вага still > VRAM-free, тому асинхронним і лишається.

## 3. wgpu / Rust-GPU (довгий приціл)

Стан 2026: wgpu v29 (03.2026) → v30, MSRV-політика стабільна, WebGPU = W3C
Candidate Recommendation Draft. Існуючі LLM-рушії на wgpu: **wgpu-llm**
(TinyLlama-only, WGSL compute, без GGUF-квантів), vulkanforge (AMD RDNA4
FP8), ggml вже має власний WebGPU-бекенdnative. **Вердикт: wgpu не є
акселератором для GSV/poolAI в 2026** — GGUF-шлях уже вкривається ggml
(Vulkan/native + WebGPU у браузері), а писати WGSL-ядро під нашу модель —
місяці заради того, що дає §2 за дні. Тримати в беклозі як кандидат на
«уніфікований GPU-шар», не в план банда.

## 4. Edge/WASM на телефоні (A54) — і «economic data transfer»

Ключові дані:
- **LlamaWeb** (llama.cpp WebGPU-backend для браузера, arXiv 2605.20706,
  05.2026): decode +45–69% проти WebLLM/Transformers.js, пам'ять −29–33%,
  23 формати ваг, 16 девайсів 8 вендорів.
- **deltanet.wasm** (llama.cpp→Emscripten): Qwen 3.5 0.8B Q4_0 ≈ **54 tok/s**
  на Ryzen 7600/Node, peak ~1GB RAM, 2.2MB wasm-бін, стрімінг через
  step-loop; **DeltaNet/SSM-опраці working** (ggml_ssm_conv/scan) — тобто
  гібридна архітектура сімейства Qwen3.8 у браузері вже крутиться.
- Chrome WebGPU на Android = Adreno-only (Mali, як у A54, off); WASM-SIMD —
  так. A54 CPU ~3–6 tok/s на 0.5–0.8B Q4 через WASM.

**Вердикт для telenetis:** Mini App може тримати **0.5–0.8B client-side**
(deltanet.wasm-подібний рантайм) для миттєвих підказок/completion без мережі
= «економічна передача даних» в лоб; мережею піде лише deep-запит. Для
телефона A54 це межова історія (peak ~1GB), тому статус — **після** режиму
транспортів (§5), не в першому спілкі.

## 5. Режими транспорту telenetis/GSV (LAN vs ngrok)

Точка зору safety, яку власник просив («у боксі локал — повний доступ, через
ngrok — only view»), вже наполовину вбудована:
- GS V `security::gate_post` відхиляє POST з не-локального Origin; LAN-режим
  (band 233) відпускає приватні мережі, ngrok-fqdn — все одно foreign origin;
- poolAI RPC-канали: upstream llama.cpp прямо каже **не вішати rpc-server у
  відкриту мережу** — наш план і так LAN-only (192.168.2.x) + host-only 56.x.

Недоліки, які закриває спринт: UI (Galaxy + Mini App) не **знає** про режим —
кнопки мутуючих дій видно і через тунель (клієнтський fetch впаде на 403,
але UX погано); SSE/WS лишають балунг через 400ms-інтервали на тунелі;
ngrok free = рандомний домен (WEBHOOK_URL у .env зараз привʼязаний).
План: `transport_mode` = origin-host класифікація (loopback|lan|tunnel) на
сервері в health + UI-бейдж, tunnel ⇒ `view-only` (client-side disable
data-action + server already 403), тунель-режим: подовжений poll, gzip,
SSE-only critical events. Див. ROADMAP **band 235 PH-S2990/2991**.

## 6. Un-bottleneck рейтинг (для нашого заліза)

1. **Deep-tier MoE swap** (:8080 → 30B-A3B) — ×11 tg за нуль RAM: PH-S2989.
2. **Transport modes + gzip/compact payloads** — UX+трафік: PH-S2990.
3. **Vulkan spike** (iGPU attention/shared) — невизначений, але потенційно
   ×2–3 на :8082/pp: PH-S2991.
4. WASM 0.8B у Mini App — «edge cache відповідей»: PH-S2992 (після 2).
5. Фреймворк-своп/PGO/wgpu — **ні**: виміряно або недоступно на Windows.

## Джерела

- ishtms/rust-framework-bench (GitHub, rewrk-бенч); rust-lang forum
  «highest-performance rust backend» (2026-01); actix-vs-axum benchmark
  (2025-10).
- llama.cpp docs: `tools/rpc/README.md` (master, 2026; insecure-warning,
  local-cache `-c`, proportional weight/KV split), `docs/android.md`
  (Termux + SME2/AMX runtime-detect).
- LlamaWeb — arXiv:2605.20706v1 (20.05.2026).
- deltanet.wasm (GitHub, 03.2026): 54 tok/s @0.8B Q4 WASM, SSM-ops working.
- wgpu: v29.0.0 (03.2026) → 30.0.0; wgpu-llm (04.2026, TinyLlama-only,
  Adreno X1-85 Vulkan 32.8 tok/s); rustify wgpu-2026 огляд (09.2026,
  WebGPU = CR Draft, rust-gpu archived 10.2025).
- OpenBenchmarking llama.cpp iGPU/pp-ряди (04–07.2026).
