# Plan: MiniCPM mesh — розселення MiniCPM по девайсах + WSP weight-fixing у Rust-екосистемі

**Дата:** 2026-11-15 · **Owner:** «розселення мініцпм по девайсам через rusty-engine / WebGPU / poolAI та покращення ваг шарів через NanoJev за відповідями хмарних моделей» · **Продукти:** `rusty-engine` · `gsv-wgpu` · `gsv` · `poolai`
**Статус:** open · **Канон:** цей документ · **Board:** `docs/gsv/tickets.jsonl`

## 1. Goal

Розселити малу мультимодальну модель **MiniCPM-V 4.7-1B** (qwen35-гібрид: 18 GDN + 6 full-attn, vocab 248 094) по доступним девайсам середовища Rust — phone WebGPU (Android) · PC GPU (wgpu/DX12) · CPU 12 threads · llama-rs/llama.cpp — так, щоби:

1. **Один інференс** (генерація токенів) проходив через девайсів, а не лише на CPU;
2. **Судячи точності не падала**: parity проти Qwen2.5-golden fingerprint + Uk-golden fixtures, `max_err_model` ≤ 1e-2 на кожному віддаленому виклику;
3. **Ваги покращувались точково**, а не рехпойнтом: WSP-патчі (NanoJev-подібна guided-by-cloud-oracle абляція), лімітовані block-bitmap patch-at-dequant (rusty-engine), grid-кворум і canary; router/gate ваги завжди frozen.

## 2. Не-цілі

- Не міняємо великі моделі (мініцпм-1B-воркер та 35B-A3B — окремі тікети; 35B — PH-1206, через ExpertCache).
- Не комітити GGUF / ваги / Uk-golden фікстури з реальними вагами.
- Не фоташити весіла на off-box; все внутрішнє, localhost/LAN.
- ECO-7.4 (Rust 1.100 re-gate 2026-11-12 → підтверджено; не змішуємо з цими тікетами).

## 3. Стан середовища (виміряно, 2026-11-15)

| Факт | Значення |
|---|---|
| MiniCPM-V 4.7-1B | qwen35-гібрид; `runner35` генерує детерміновано (244 тести + 7 ignored); live prefill 961 tok ≈ 5.3–6.4 s (12 threads); decode 10–27 tok/s (залежить від фону) |
| GSV-WGPU mesh | смужньо: A–J закриті; 7 ops з verified op-echo + hub model check (`max_err_model`); `GET /api/wgpu/capability` — профілі/квоти/роутинг (sticky → fallback), audit `wgpu_quota_audit.jsonl`; live `:9999` rev 516 |
| ECO-9.1 / ECO-9.2 | done: WSP feasibility вердикт 2026-11-09 — гібрид ~55% ≥ owner-порогу 55 → ECO-9.2 arch-sketch done; API Engine kinds (`ActivationAudit`/`WeightPatch`) схвалено гіпотетично |
| WSP відкриті | WSP-G1 (GSV hub — task envelope bridge) · **WSP-G1-rusty ✅** (block-bitmap patch-at-dequant apply lane, rusty-engine `6e19d3b`, real-model Q6_K smoke ok) · WSP-3 (poolAI — cloud-oracle delta patch pack + grid ActivationAudit/WeightPatch eval gate) |
| MiniCPM відкриті | PH-1202..1206 (vision tower / serve / 35B), PH-404 (ExpertCache LRU), PH-405 (greedy-parity), PH-407 (tok/s bench vs llama-rs), PH-1305/1306 done |
| Cloud lane | OpenCode free models: **Flint 40B → GLM 5.3 → DeepSeek 1017** (fallbacks Big Pickle · DeepSeek V4 Flash · Nemotron Ultra Free); 429 → switch; OmniRouter `/api/omni/route` вибір моделі |

## 4. Архітектура шарів

```
[Cloud models via OmniRouter: Flint 40B / GLM 5.3 / DeepSeek 1017]
        │ weight-fix candidates (WSP patches, NanoJev-style guidance)
        ▼
[poolAI grid — ActivationAudit / WeightPatch job kinds; quorum + canary]
        │ signed verdicts → staged rollout
        ▼
[rusty-engine :8090 — qwen35-hybrid runner35 (CPU, 12 threads)]
        │ per token: MoE expert matvecs offloaded (top-k) as shard groups
        ▼
[GSV hub :9999 — mesh dispatcher, provider row `rusty-minicpm-mesh`, ledger]
        │ task envelopes: matvec_shard groups on REAL Q8 weights (ContentAddress store)
        ▼
[browser peers: phone WebGPU (Android) · PC wgpu/DX12] + [llama-rs nodes]
```

### Ключові рішення

1. **Engine 1B**: rusty-engine (чистий Rust, GGUF-first); oracle = Qwen2.5 fingerprint; numerical rollback first.
2. **Важлива поправка (2026-11-15, виміряно):** MiniCPM-V 4.7-1B — **dense** qwen35-гібрид
   (18 GDN + 6 full-attn, vocab 248 094, **без MoE-експертів**). Tensor-name prefixes: blk/*
   (318 тензорів), вихід `output` + `token_embd`. Отже «ExpertParallelDispatch» як top-k=2
   expert fan-out **не застосовний до 1B** — те саме розсічення ваг діє як
   **dense FFN offload** через `weights_matvec` (GSV-WGPU MC-2, `blk.ffn_down.weight`
   [3584×1024]). Справжній top-k=2 expert dispatch належить **35B-A3B** (PH-1206 трек).
3. **Що йде в меш**: лише expert/dense-матриці SwiGLU (gate/up/down) і `matvec_shard`/`weights_matvec`
   групи; GDN-стан і attention stay на CPU-шляху engine (перенесення GDN — інший бенд).
4. **Ваги в девайсі**: не toy-константи, а реальні Q8 блоки; розповсюдження — content-addressed hub store (sha256, chunked, cache у браузері).
5. **Перевірка**: кожен remote-результат має `max_err_model` vs CPU shard-reference і verified `op`-echo; інакше задача `failed`.
6. **WSP lane**: лише block-bitmap patch-at-dequant (rusty-engine) ≥ zero-downtime; llama-rs — offline rewrite + restart; все проходить Uk-golden + chat-fixture counterfactual eval (router/gate frozen), 2/3 кворум, canary, auto-rollback при regression.
7. **Cloud oracle**: кандидат від хмарної моделі (через OmniRouter `task=reasoning`, `prefer_free=true`) → WSP patch-pack → grid gate → rollback/keep. Обов'язковий фік у product docs (`WSP_WEIGHT_SPACE_PROGRAMMING_BRIEF.md`).
8. **Мовна гарантія**: Uk-golden fixtures — обов'язковий гейт кожного патча та кожного offloaded matvec (freeze router/gate, Uk-golden до/після).

## 5. Майлстоуни

| # | Milestone | Залежності | Тікети |
|---|---|---|---|
| M0 | Архітектура мережі + дроти mesh | — | **MC-7** (gsv-wgpu) |
| M1 | Store ваг на хабі | M0 | **MC-3** (gsv-wgpu) |
| M2 | Device-side real-weight inference | M1 | **MC-2** (gsv-wgpu) |
| M3 | ExpertParallelDispatch у rusty-engine | M2, WSP-G1 (hub), WSP-G1 (rusty) | **MC-1** (rusty-engine) |
| M4 | Provider row + telemetry | M3 | **MC-4** (gsv) |
| M5 | Hybrid prefill/decode split bench | M3 | **MC-5** (rusty-engine) |
| M6 | WSP gate on MiniCPM path | M3, WSP-3, ECO-9.2 | **MC-6** (poolai) |
| B12 | MiniCPM-V full serve (vision) | PH-1306 | PH-1202/1203/1204/1205 → PH-1206 |
| Perf | MoE core (LRU/offload/bench) | — | PH-404 / PH-405 / PH-407 |

## 6. Тікети

### Нові (цей план, 2026-11-15)
| Тікет | Board-id | Продукт |
|---|---|---|
| **MC-7** | t-1794758435656709900 | `gsv-wgpu` |
| **MC-3** | t-1794758435734724800 | `gsv-wgpu` |
| **MC-2** | t-1794758435803909000 | `gsv-wgpu` |
| **MC-1** | t-1794758435872050000 | `rusty-engine` |
| **MC-4** | t-1794758435965341800 | `gsv` |
| **MC-5** | t-1794758436045763000 | `rusty-engine` |
| **MC-6** | t-1794758436128993400 | `poolai` |

**Переглянута інтерпретація (2026-11-15, виміряно на GGUF):**
MiniCPM-V 4.7-1B — dense. Тому:

- **MC-1** застосовна до **MiniCPM-V 4.7-35B-A3B** (MoE) — експерти top-k=2. Для 1B еквівалент —
  dense FFN offload через `weights_matvec`/`matvec_shard` (вже зроблено в MC-2). MC-1 на 1B
  виконується лише як **перший крок** (інтеграційний гейт до 35B-B_track), не як оптимізація швидкості.
- **MC-4** і **MC-5** — без змін (35B) або можуть зупинятись на 1B-evidence тоді визначиться.

### Існуючі (залежності)
- **MC-7** `gsv-wgpu` — Mesh networking architecture & engineering (класи девайсів, budget, ordering/failover).
- **MC-3** `gsv-wgpu` — CentralWeightStore: content-addressed ваги на хабі (sha256, chunks, audit, cap).
- **MC-2** `gsv-wgpu` — Real-weight device-side inference (Q8 shard groups у браузері, max_err_model vs hub).
- **MC-1** `rusty-engine` — ExpertParallelDispatch: MoE expert matvecs як mesh-групи, top-k=2, CPU fallback, Uk-golden gate.
- **MC-4** `gsv` — provider row `rusty-minicpm-mesh` + mesh-vs-local tok/s telemetry + Galaxy provider card.
- **MC-5** `rusty-engine` — Hybrid prefill/decode split; bench на тому ж runner35 (CPU↔mesh), docs/benchmarks.
- **MC-6** `poolai` — WSP gate on MiniCPM path: cloud-oracle → patch → Uk-golden gate → rollback/keep.

### Існуючі (залежності)
- WSP-G1 (gsv) — task-envelope bridge; **WSP-G1-rusty ✅ 2026-11-15** (rusty-engine `6e19d3b`, block-bitmap patch-at-dequant); WSP-3 (poolai) — cloud-oracle/ActivationAudit eval gate.
- PH-1202/1203/1204/1205/1206 — MiniCPM-V serve + vision tower + 35B.
- PH-404/405/407 — MoE perf; PH-1305/1306 done; EN-5/EN-6 done.

### Залежність MC-тікетів

```
MC-7 (arch) ──► MC-3 (store) ──► MC-2 (real-weight inference) ──► MC-1 (ExpertParallelDispatch)
                                                                    │
                                              MC-5 (split bench) ◄──┘
                                                                    │
                                              MC-4 (provider row) ◄──┘
MC-6 (WSP gate) — після MC-1 і WSP-3 (незалежна ланка, малtiply-checked)
```

## 7. Метрики успіху

| Метрика | Ціль |
|---|---|
| tokens/s (CPU-only vs mesh) | mesh ≥ CPU-рівень при ≥1 offloaded expert; звіт `docs/benchmarks/MESH_SPLIT_BENCH.md` |
| `max_err_model` | ≤ 1e-2 на кожному remote shard group (hub CPU reference) |
| Uk-golden parity | 0 regressions на dispatch-able matvecs (до/після в Uk-golden fixtures) |
| WSP patches | ≥1 цільовий патч через cloud-oracle + gate; rollback-or-keep decision задокументовано |
| Provider row | chat via mesh з telemetry (mesh tok/s, peers, groups closed) у Galaxy provider card |

## 8. Ризики

- **Block-квантування**: одиничний FP параметр не патчиться без блоку (Q4_K_M superblock 32, Q8_0 block 32) → лише block-level patch + requant шум; гарди — golden eval і patch density limit.
- **Девайс-latency**: phone WebGPU валідний лише warm (device reuse) — холодний старт ~100–300 ms; перші токени можуть віддавати CPU (розклад failover).
- **Фонові task-стілерси**: старі піри (band-G/H сторінка) рахують fallback-шард — тому верифікація op-echo обов'язкова; інакше роутинг йде до них як до «швидких».
- **Топологія ваг / MoE**: абляція експертів б'є пропорційно по мовному кластеру (укр. — меншинний); freeze router/gate обов'язковий.
- **Довіра grid**: патч тими — hash + кворум + eval; без 2/3 підписів не apply.

## 9. Правила (легасі + нові)

- Один коміт = код + тести + docs; `data/*` не індексується; `git add -A` заборонено.
- Гейти: `cargo fmt --all` → `cargo clippy --all-targets -- -D warnings` → `cargo test` → `gsv-loc-audit --stretch-96` (≥ 96 %).
- Тікет → claim → робота → done з evidence; 1 band = 1 коміт; push на межі банду.
- Ваги/Uk-golden: не комітимо; fleet-level eval через `ActivationAudit` + `WeightPatch` job kinds.
