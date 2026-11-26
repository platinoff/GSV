# LLM-двигуни 2025–2026 → адаптація під GSV/Rust 1.100/WASM (2026-11-26)

**Статус:** research + plan, будь-яка імплементація — окремими тікетами. **Межа:** G13 blocked; жодного runnable-full-LLM/image→answer/quality-claim.

Джерела: 4 паралельні read-only research-агенти (primary docs/PR/papers + локальний інвентар `file:line`). Попередній канон: [`GASV_STREAMING_MOE_PLAN_2026-11-26.md`](GASV_STREAMING_MOE_PLAN_2026-11-26.md), [`GSV_DEBUG_CANON.md`](../GSV_DEBUG_CANON.md).

## 1. Що є в ландшафті (стисло, з чесними межами)

| Клас | Представники | Механізм | Чи придатне нам |
|---|---|---|---|
| Static CPU offload | llama.cpp `--cpu-moe` / `-ot exps=CPU`, ik.cpp |_regex_ закріплює експертів за CPU; mmap, без перерозподілу слотів | Надто грубо: 1 tensor 151 МБ > 256 МБ phone buffer, цілий шар ~450 МБ |
| GPU expert LRU cache | llama.cpp #29887, #30966 | LRU найгарячіших експертів у VRAM; 89 % hit → 2.2× (RTX 5090, 93.7 ГБ Q4_0) | **Ідея так, цифри ні** — 16–32 ГБ VRAM у авторів |
| Slot pool / remap | HF `moe-expert-streaming` #50717, memoriaru #28248, ik.cpp PoC | один пул слотів на шар, table expert-id→slot, `grouped_mm` без змін | **ADOPT** (див. §3) |
| Three-tier cache | SLRU fork, MoE-Infinity EAM, Strata | VRAM↔RAM↔SSD, профілювання роутингу, prefetch | **ADOPT control-plane**, SSD/VRAM tier — ні |
| EP / all-to-all | vLLM EP+EPLB, llm-d WideEP, SGLang, TRT-LLM | logical→physical expert map, DeepEP/NIXL | **REJECT**: Wi-Fi, 256 МБ phone, без NVLink/RDMA; AdaOffload Observation 1: TP > EP для hybrid |
| P/D disaggregation | llm-d, KubeRay, Kthena | KV transfer через NIXL, gang-scheduling | N/A: наш хаб тримає стан, піри stateless |
| Browser P2P | SwarmLLM → Pooled, LLMlet | суміжні шари, 4–10 КБ f16 hidden state / token, WebRTC | **REJECT як транспорт**: немає верифікації (SECURITY.md прямо каже), порушує environment security |
| WebGPU in-tab | WebLLM/MLC, WeInfer, LlamaWeb, gpu.cpp | ціла модель в одному tab, OPFS→WebGPU 4×1 МБ, dequant у registers | **ADOPT ідеї**: compact hidden state, buffer reuse, WGSL dequant |
| Verification research | VeriLLM, TOPLOC, Freivalds | Merkle commitments + VRF sampling | Наша golden-x вже сильніша практично; ADOPT лише cheap sampling пізніше |
| **_polystrata_** | `simplexai-labs/PolyStrata` (MIT, VecSzn, на базі `Niko1221/Strata`) | «спілка під різне залізо» = union backends (CUDA/HIP/Metal/Arc/Strix Halo) + 2 engines (`strata`, `strata-poly`), **не** кластерний планувальник. `qwen35moe` підтримується напряму | **ADOPT ідеї**, бінарники REJECT (8 ГБ dGPU + 17–32 ГБ RAM мінімум) |

### PolyStrata — окрема довідка (за запитом owner)

- Репо: `github.com/simplexai-labs/PolyStrata`, реліз `v0.2.20-poly.5` (2026-11-25). «Спілка» як окремий артефакт/flag **не існує** — це метафора об'єднання бекендів.
- Strata engine: router 10/24576, adaptive expert cache в VRAM (~700 experts/ГБ), CPU рахує misses паралельно, SSD n-gram table, prefetch наступного шару по PCIe, MTP speculative.
- `strata-poly`: `Engine` в `src/poly/engine.hpp`, GGUF read-in-place, architecture-hook table; `qwen35moe` = наше сімейство. 64 tok/s на RTX 4070 Laptop 8 ГБ + 32 ГБ RAM (Q4_K_M 21 ГБ) — **інший клас заліза**, нам не ціль.
- Взяти: expert-profile (`--dump-routing` → hot list), адаптивний бюджет у одиницях expert-blob, router-lookahead prefetch з watermark gate, `MADV_SEQUENTIAL`-подібні seq range reads, правило `pcie-frac` ( stream vs compute on CPU ).
- Відкинути: CUDA/HIP/Metal-бінарники, NVFP4/CUDA graphs, mmap/page-cache tier (у браузера немає), MTP/TailSkip, 2–3 GPU layer split по PCIe.

## 2. Що вже є локально (не писати з нуля)

| Примітив | Де | Нотатка |
|---|---|---|
| Router math | `rusty-engine/src/moe.rs:200` `route_noaux_tc`, `:97` `top_k_sigmoid` | pub, synthetic F32; bias/noaux не парсяться з GGUF |
| Expert FFN/combine | `moe.rs:288` `expert_ffn`, `:306` `combine`, `:397` `moe_layer_forward_cached` | pub; тільки F32; closures не async |
| ExpertCache | `rusty-engine/src/cache.rs:51` `get_or_load`, `:169` `set_pinned` | LRU, std-only, не тримає byte ranges |
| MoE GGUF metadata | `rusty-engine/src/model.rs:366-391`, `rusty.rs:325-374` `manifest` | масив dims `[rows,ff,n_experts]`; `artifact-chunks` тільки цілий тензор (`:1861-1931`) |
| Stream dequant | `GSV/src/boxes/wgpu_dequant.rs:269` `dequant_q4_k_range`, `:290` `tensor_raw_range` (pub(crate)), `:334` `tensor_f32_range` | вже block-aligned, golden-тести |
| Artifact range read | `GSV/src/boxes/weight_store.rs:100` `read_chunk_artifact` (**private**), `:194` `register_tensor` | sha256 fail-closed; треба зробити `pub(crate)` |
| Dispatch + result bus | `GSV/src/boxes/wgpu.rs:1973` `task_enqueue`, `:1489` `task_result`, `:2594` `group_results`, `:1781` block op | proof gate, EWMA, quota 67 МФ; x_spec `onehot:i:n` |
| Оркестрація-шаблон | `GSV/src/boxes/vision_forward.rs:312` matvec, `:454` report, `:1037` checkpoint/resume | готовий патерн «хаб тримає мале, піри рахують велике» |
| Бенч | `GSV/src/boxes/moe_bench.rs:152`, xtask `:730-804` | gate/up PASS 130.98/187.06 MFLOPS, max_err 3.47e-18 |

**Критичний факт для архітектури:** `runner35.rs:222-224` завантажує **dense** FFN — `qwen35moe` runnera немає (BENCH-3). Отже ми не «додаємо до існуючого runnera», а будуємо **новий orchestrator у GSV**, використовуючи rusty-engine лише як reference для математики (без path-dep).

## 3. Скетч архітектури — `src/boxes/moe_forward.rs` (новий модуль)

### 3.1 Слої та контракт

```
┌───────────────────── HUB (Rust 1.100, tokio/axum) ─────────────────────┐
│ ForwardRequest ──▶ plan::build_plan ──▶ [LayerPlan; 40]                 │
│   router (2 МБ/шар, F32) ──▶ top8_experts ──▶ ExpertAddr(layer,e,role)  │
│   place::score_peer (alive ∧ proof_gate ∧ EWMA ∧ cache ∧ max_storage)  │
│   exec::enqueue_group (Semaphore ≤2, ⌀ 8 shard/tenant, lease 300/1200) │
│   verify::check_partials (max_err_model < 1e-4) ──▶ combine            │
│   small state: attention/GDN/RMSNorm на хабі, checkpoint на шар        │
└──────────┬───────────────────────────────▲──────────────────────────────┘
           │ ExpertTask JSON               │ Partial + max_err_model
           ▼                               │
   L: mirror ──range+sha256──▶ #43 PC WGPU (2 ГБ) · #39 phone (256 МБ)
                                WASM SIMD kernel + golden JS fallback
```

### 3.2 Типи (реалізація — окремий тікет)

```rust
pub enum ExpertRole { Router, Gate, Up, Down, SharedGate, SharedUp, SharedDown }
pub struct ExpertAddr { pub layer: u16, pub expert: u16, pub role: ExpertRole }
impl ExpertAddr {
    pub fn tensor_name(&self) -> String;   // blk.N.ffn_{gate,up,down}_{exps,shexp}.weight
    pub fn alias(&self) -> String;         // <=128 chars, registry-safe
    pub fn dims(&self) -> [u64; 3];        // [2048,512,256] | [2048,512] | [512,2048]
    pub fn byte_range(&self, data_start: u64, tensor_off: u64) -> (u64, u64);
    // start(e) = data_start + off + e*vals/256*blk_bytes   (blk=144 Q4_K | 210 Q6_K)
    // len(e)   = vals/256*blk_bytes  → 589_824 | 860_160 B
}
pub struct ExpertTask { pub op: Op, pub addr: ExpertAddr, pub x_spec: XSpec,
    pub shards: usize, pub group_id: String, pub lease_secs: u64, pub byte_range: (u64,u64) }
pub struct PeerCapability { pub peer_id:u64, pub alive:bool, pub proofs_fresh:u32,
    pub ewma_ms:Option<f64>, pub cached_shards:Vec<String>, pub max_storage_bytes:u64, pub f16:bool }
pub struct StateBudget { pub max_kv_tokens:usize, pub max_hot_experts:usize, pub max_inflight_groups:usize }

pub fn manifest_offsets(rusty:&Path, artifact:&Path) -> Result<HashMap<String,(u64,u32)>,String>;
pub fn register_expert_range(data_dir:&Path, addr:&ExpertAddr, start:u64, len:usize, ggml_type:u32) -> Result<Value,String>;
pub fn router_logits(router_f32:&[f32], hidden:&[f64]) -> Result<Vec<f64>,String>;   // port moe.rs:97
pub fn top8_experts(logits:&[f64]) -> Vec<(usize,f64)>;
pub async fn dispatch_layer_ffn(client:&reqwest::Client, base:&str, layer:usize,
    hidden:&[f64], chosen:&[(usize,f64)], shards:usize, wait:u64) -> Result<Vec<f64>,String>;
pub fn combine(routed:&[Vec<f64>], weights:&[f64], shared:Option<&[f64]>) -> Result<Vec<f64>,String>;
pub fn state_budget(n_full:usize, n_recur:usize, kv_dim:usize, gdn_bytes:usize, free:usize) -> StateBudget;
```

**Важлива геометрія:** один expert `[2048,512]` = 2 МФLOP → **1 shard** (замість 8 для цілого `[2048,131072]`), жодного `onehot` x-hack на 262 КБ. Це знімає квотний тиск і є прямим наслідком per-expert addressing.

### 3.3 Wire JSON (сумісний із тим, що хаб уже приймає)

```json
{"kind":"MoeLayerPlan","request_id":"req-20261126-1","layer":0,
 "routes":[{"token":0,"experts":[{"id":17,"weight":0.31},{"id":203,"weight":0.19}]}],
 "tasks":[{"op":"weights_matvec","tensor":"blk.0.ffn_gate_exps.weight",
           "x_spec":"onehot:17:131072","shards":1,"group_id":"g-7371-L0-E17-gate",
           "byte_range":[1048576,1703936],"sha256":"9f2c…","lease_secs":300}],
 "combine":{"gate":"Sigmoid","top_k":8,"shared":true},
 "checkpoint":{"next_layer":1,"kv_budget_bytes":104857600}}
```

### 3.4 Планувальний цикл (порядок кроків)

1. `manifest_offsets` через `rusty manifest` (без змін rusty-engine).
2. Router на хабі (2 МБ/шар) → top-8 → `ExpertAddr` для gate/up/down + shared.
3. **Slot-pool remap** (HF/Strata ідея): N ≥ 2×top-k = 16 слотів/шар, спільних для всіх 40 шарів; router-id → slot-id до enqueue; prefill не витісняє decode-слоти.
4. **Per-expert byte-range** реєстрація (0.59/0.86 МБ замість 4 МБ chunk), sha256 зберігається.
5. **Router-lookahead prefetch** на 1–2 шари вперед, поки поточні шарди в дорозі; тільки при free RAM вище watermark (ідея Strata `STRATA_EXPERTS_SEQ_READAHEAD` + HF).
6. Score пірів: `alive ∧ proof_gate`, EWMA α=0.3, cache-affinity (prefix analogue), `max_storage_bytes` cap, blacklist за історією `max_err_model`; fail-open → CPU на хабі для малих тензорів.
7. Group enqueue з `Semaphore(≤2)` і per-peer concurrency (зараз 1 → **головний резерв швидкості**: compute 131–187 MFLOPS vs wall 270–280 c).
8. `check_partials` (`max_err_model < 1e-4`) → `combine` → attention/GDN/norm на хабі → checkpoint (`vision_forward.rs:1037`) → 40 шарів → logits.
9. EPLB-аналог: ковзне вікно hit-ів, якщо Gini високий — міняємо лише mapping логічного expert'а на byte range (без фізичного переміщення ваг).
10. Жодного quality-claim: будь-який вихід = numerics evidence тільки.

## 4. Що адаптувати з WebGPU/WASM (черговість)

| Пріоритет | Зміна | Де | Ефект |
|---|---|---|---|
| P0 | `x_spec` у items `weights_matvec_block` | `wgpu.rs:1781-1864`, `:1662-1717`, page `:3584-3604` | −262 КБ × 8 на задачу |
| P0 | streaming `weights_matvec_reference_segment` замість full-tensor memo в block verify | `wgpu.rs:1687` | −~1 ГБ f32 materialization |
| P0 | device-limit-aware `per` rows | `wgpu.rs:1921`, `:3633-3660` | 256 МБ phone vs 2 ГБ PC |
| P1 | збільшити/виміряти concurrency з proof-gate збереженим | `wgpu.rs:362` | ціль: wall 270 c → ~10 c |
| P1 | grouped-expert WGSL kernel (Pooled `moe_gsort` мотив) | page `:3425-3432`, `:3584-3604` | один task = gate+up/down кількох експертів |
| P1 | SIMD `i8x16_swizzle` LUT dequant у `mesh-wasm` (ABI v2) | `crates/mesh-wasm/src/lib.rs:194-231` | ~4× менше WASM пам'яті; **обов'язково re-golden** f16 `e=113` |
| P2 | sampled verification (VeriLLM commit-then-sample) | `wgpu.rs:1585-1638` | зняти CPU тиск на huge shard |
| P2 | offline shell `/wgpu` + wasm у Service Worker (size stamp) | `src/boxes/sw.rs:13-19` | менше cold-start нагрузки на хаб |

**Відкинуто свідомо:** WebRTC layer-range (втрачає golden-x), exo/llama.cpp RPC (нема верифікації, trusted network), FP4/NVFP4/Marlin/DeepGEMM (телефони мають fp16/f32), CUDA graphs/Metal events, спекулятивне декодування (нема MTP блоку в нашому артефакті), protobuf (JSON + serde вже контракт), нові crate dependencies.

## 5. Реальна межа (чесно)

- Найближчий локальний вимір: llama-rs Qwen3-30B-A3B IQ2_XXS на 7.4 ГБ RAM = **0.497 tok/s, TTFT 298.5 c**. Наш conjecture: ~1.5–3 c/токен bandwidth + ~10–15 c/токен compute на 2 пірах ⇒ **<1 tok/s**.
- Усі 50–90 tok/s у огляді — RTX 4070/5090 + 17–120 ГБ RAM або Mac M5 32 ГБ. PolyStrata на 8 ГБ dGPU дає 64 tok/s **лише з 32 ГБ RAM**; у нас 1.7 ГБ вільно. Це не ціль і не бенчмарк-орієнтир.
- K8s/k3s/Wasmtime-компоненти — **owner-gated / N/A** тут (телефони не є подами k8s, Pi мертва, 2026-11-19 план позначено OBSOLETE); наші EPP/EPLB-еквіваленти вже живуть у `ranked_peers` + `proof_gate` + EWMA.
- G13 blocked, G19 open, PH-1204 потребує навченого вирівнювання. Streaming forward змінює «транспорт працює» на «модель рахує токен», але **не** дає права на quality-claim.

## 6. Тикетізація (наступні кроки)

1. **FWD-1** ✅ `071d081` — `moe_forward.rs` скелет: `ExpertAddr`/`manifest_offsets`/`register_expert_range`/`router_logits`/`top8_experts`, 5 unit-тестів + ignored-діагностика (gate e17 = `963314560..963904384`, 589 824 B).
2. **FWD-2** — per-expert registration + 1-shot canary: `register_expert_range()` + `cargo xtask moe-expert-canary --expert 17 --role gate --shards 1` (реєструє 589 824 B зріз як artifact-backed tensor `[2048,512]`, ставить `weights_matvec onehot:0:512`, відхиляє `max_err >= 1e-4`). Сингл-шард enqueue повертає `task_id` без `group_id` — grou зводиться з `mt-N → g-N`.застряглі claimed-задачі попередніх діагностик (`g-7367`) очищуються в operational даних (gitignored); valhall `#44` owner забрав із флоту — працюємо `#43` + `#39`.
3. **FWD-3** ✅ — P0/WebGPU адаптації: `x_spec` у block items, streaming segment reference замість full-tensor memo, device-storage guard (256 МБ, `shard_rows_within_storage`). TDD 2 тести green. Живий block canary `g-7385` (`q4k/e17/gate`, x_spec, `#43`): `done`, `max_err 3.47e-18`.
4. **FWD-4** — 1-layer coherent forward (router → top8 → gate/up/down → combine) з checkpoint; numerics only. Знайдено й виправлено баг `tensor_f32`: F32 тензор із явним `ggml_type:0` не читався (match-arm крив за `dtype=="f32"` лише при `qtype=None`) — regression-тест додано.Детермінований (`--seed`) hidden vector, 24 matvec'и на реальних байтах артефакта, checkpoint `data/moe_forward_checkpoint.json`. НЕ семантичний forward моделі — жодного quality-claim (G13).
5. **FWD-5** — EPLB/hot-set + lookahead prefetch + concurrency tuning. Чисті helper'и в `moe_forward.rs`: `routing_stats` (histogram + Gini + нормалізована ентропія + hot set, пишеться в checkpoint кожного forward'у), `should_prefetch` (watermark gate), `concurrency_decision` (wall vs compute → 1|2). TDD 3 тести green. Чесний закон: concentrated routing дає виграш від hot-set, flat — ні; спершу вимірюємо Gini. Живі forward'и seed 1.0/2.0: однакові top-8 `[26,39,53,97,102,131,135,248]`, `entropy_bits 0.375` (=log2(8)/log2(256)), `gini 0.969` — але `--seed` був скалярним множником hidden, тож не змінює вибір, а Gini по одному токену завжди високий (top-k=8). Рішення про hot-set відкладено до багатотокенного профілювання зі змінним hidden-вектором. NUMERICS ONLY, без quality-claim (G13).

6. **FWD-6** ✅ — багатотокенне профілювання: `deterministic_hidden(seed,token)` (справді змінний, wrapping-hash), `routing_profile(router,N,seed)` (hub-only, без кластера), `load_router_f32`, CLI `cargo xtask moe-routing-profile --tokens N --seed S`. TDD 2 тести green. Живі профілі 64 токени, seeds 1/2/3/7: `entropy_bits 0.377–0.380`, `gini 0.969`, **`hot_count 8` стабільно** — роутинг концентрується на ~8 експертах попри змінні hidden → **гарячий набір 8–16 експертів/шар вартий**, але самі expert ids відрізняються між seeds ⇒ потрібен динамічний hot-set з профілю, не статичний. NUMERICS ONLY, без quality-claim (G13).
6. BENCH-2 залишається open: down-шард блокується від'єднанням телефонів (environmental), не логікою.
