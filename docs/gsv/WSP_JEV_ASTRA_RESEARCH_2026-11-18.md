# WSP / Jev / Astra — ресерч + кластерний план (2026-11-18)

**Статус:** research/design only — **жодного коду, жодного патчу ваг**. Доки owner не дасть go,
це документ-план (як ECO-9.1/9.2). Автор-власник: GSV ecosystem (агенти цього бокса).
**Мета:** (1) як «jev-система» (Jev/NanoJev, System One Models, TypeSafe AI) перевіряє свої можливості;
(2) як **GPT-6 Astra** (MIT CSAIL, «Sparks of Weight-Space Programming») виправила ваги **Qwen3.5-0.8B** — і що з цього переноситься в наш кластер;
(3) як за допомогою **поточної хмарної моделі** (Zen free: Flint 40B → GLM 5.3 → DeepSeek 1017) + **завантажених GGUF** вибрати й створити **мапінг розміщення** (placement mapping) пірів для покращення **MoE**.

---

## 1. Джерела

| Джерело | URL | Що взято |
|---|---|---|
| MIT CSAIL blog post (2026-11-02) | rikiyat.github.io/blog/weight-space-programming | Повний текст «Sparks of Weight-Space Programming» |
| Replication/mirror md | github.com/lim6112j/weight-space-programming | 372 lines raw (Exp. 1–8 + додатковий) |
| dspy-wsp typed-decisions README | github.com/cmpnd-ai/dspy-wsp — results/2026-11-07-typed-decisions-astra | Astra vs NanoJev/d1-2B/d1-omni-600M hard numbers + «paper's rule» |
| NanoJev (HF) | huggingface.co/C-Tianyu/NanoJev | Capability verification protocol, SHA256, matched tasks |
| Jev (TypeSafe) | typesafe.ai/blog/introducing-system-one-models-and-jev | System One / RLCD / verification philosophy |
| latent.space coverage | latent.space/p/ainews-… | Контекст + Machine-readable references |
| Локально: WSP brief / sketch | `poolAI/docs/development/WSP_WEIGHT_SPACE_PROGRAMMING_BRIEF.md` · `WSP_HYBRID_ARCHITECTURE_SKETCH.md` | ECO-9.1 verdict 55% · ECO-9.2 sketch · guards |
| Локально: MoE landscape | `docs/gsv/MOE_MODELS_LANDSCAPE_2026-11-18.md` · `R4_FULL_FLEET_DISPATCH_2026-11-18.md` | rect 40/40 block-op, shortlist |

---

## 2. Як Jev / NanoJev перевіряє свої можливості

**Jev** (TypeSafe, ранній доступ 2026-09-15): System One модель — typed probabilistic
decisions, parallel sampler, RLCD (Reinforcement Learning for Calibrated Decisions),
**не генерує рядки**. Верифікація — не benchmarks у класичному сенсі:
- **Workflow evals** проти референсних ймовірностей найсильніших моделей (Astra + Fable 5.1 як reference average);
- **Type-safety by construction** (schema match гарантовано → 0% галюцинацій-типів by claim);
- **Hallucination rate** вимірюється емпірично лише для LLM-конкурентів, своє — конструктивне;
- **Cost/latency**: $0.042/MTok input, 70–500 ms e2e, 100x дешевше frontier; Pareto-frontier проти тренованих промптом LLM.

**NanoJev** (open nano-replica, Qwen3-0.6B backbone, MIT) — точний протокол верифікації можливостей:

| Елемент протоколу | Деталь |
|---|---|
| Фіксовані matched tasks | Maze (10 cases) · Snake (8) · ViZDoom Basic (128) · Predict Position (128) |
| Frozen cases | однакові recorded structured observation interface для всіх систем |
| Умови | controller ε = 0.1, sampling seed 17 |
| Checkpoint selection | **тільки на dev split**: 4 matched runs (hard/soft PP targets × 2 LR), `hard_lr1e5` @ step 400; **test і OOD не торкаються при виборі** |
| Артефакти | SHA256_MANIFEST.json + повний inference source + dataset (C-Tianyu/NanoJev-Data); independent replay checks |
| Результат vs Jev | Maze 4/10 (Jev 7/10) · Snake 8/8 (= Jev) · Basic **128/128** (Jev 56/128) · Predict Position **27/128** (Jev 11/128) — NanoJev обходить Jev на 2 з 4 задач |
| Untuned Qwen3-0.6B baseline | 2/10 · 0/8 · 56/128 · 11/128 |

**Cross-check тим самим форматом, як у dspy-wsp (те, що ми реально можемо повторити локально):**
- 25 forward passes · 300 validation cases · 400 test cases · **paper's rule** (writer не рахує head сам — тільки forward-pass tool; `simulate_head=False`, `--strict`);
- paired gain over plain model ± SE; **ReAnchor** як парна процедура поверх WSP;
- вимірюють **двома різними метриками**: most-likely-option accuracy vs program-answer accuracy (вони різні!).

---

## 3. Як Astra виправила ваги Qwen (Experiment 8 + додатковий експеримент)

**Проблема (з model card Qwen3.5-0.8B, офіційне попередження власника моделі):** thinking loops.
Базова модель: **85 з перших 96 practice-відповідей доходять до ліміту 32 768 токенів**.

| Крок Astra | Деталь |
|---|---|
| Контекст | Python-kernel над вагами (без GPU) · practice sets GSM8K (7 469) + MATH (7 275), **не test** · бюджет 10 / 25 / 50 forward passes (avg batch 64, max 512) · ніякого training/gradient info · ніякого написання training-коду |
| Діагностика | Looping trace до **одного напрямку в MLP шарів 15 і 19** (одна «loop direction») |
| Фікс | Відняти цей напрямок з **64 output-columns у кожному з двох шарів** → **131 072 ваги = 0.016 % від 0.8B** («much like removing a behavior by deleting a single direction», ref. Ardītī et al. 2024 refusal direction) |
| Зразки | temperature 0.6, top-p 0.95, top-k 20, presence penalty 1.5 (налаштування з Qwen model card) |
| Eval | MATH-500, GSM8K + 6 інших бенчів, thinking on, до 32 768 токенів; 16 семплів AMC23 / 4 семпли MATH-500, Minerva, OlympiadBench, GPQA-D / 2 семпли GSM8K, IFEval / 1 семпл MMLU-Pro-1k |

**Результат (50 forward passes):**

| Метрика | Base | Astra-edited | Δ |
|---|---:|---:|---:|
| MATH-500 | 7.9 % | **48.1 %** | +40.2 |
| GSM8K | 13.8 % | **67.1 %** | +53.3 |
| IFEval | 21.0 % | **62.3 %** | +41.3 |
| Дійшло до token limit | 93.0 % | 33.1 % | −59.9 |
| (10 passes) | 12.3 / 25.7 / 49.0 / 87.2 | | |
| (25 passes) | 14.1 / 25.1 / 39.9 / 86.4 | | |

**Ключове обмеження (додатковий експеримент, Gemma 3 270M):** модель, яка майже не зациклюється
(8.5 % vs 93 %), Astra покращити **не змогла** — всі помилки були «звичайні міркувальні помилки» без
єдиного вимикача; єдиний зафіксований рух ≈ ±1 пункт MATH-500, який сам Astra назвала непідтвердженим.
**Закон:** *Editing weights could remove an obstacle that hid an ability, but it could not add an ability the model lacked.*

**Що Astra написала проти NanoJev (typed decisions, MLX, MacBook Pro, 1 conversation, 25 forward passes, 300 val):**
- Plain → WeightSpace: NanoJev 39.2 % → 52.4 % (+13.3 ± 1.1); +ReAnchor → **56.4 % (+17.3 ± 1.3)**;
- d1-omni-600M: 50.9 % → 66.9 % (+16.1); +ReAnchor → **69.1 % (+18.3 ± 1.3)**;
- d1-2B: 60.6 % → 74.0 % (+13.4); +ReAnchor → **77.9 % (+17.3 ± 1.3)**;
- `gpt-6.1-sol` (слабший writer): NanoJev +12.1 → 51.3 %, d1-2B **0** (−0.6) → модель-володар якості вирішує;
- Under **paper's rule** (writer не рахує head): d1-omni-600M WSP gains shrink to +2.8 ± 0.7 — тобто частина «магії» була не в вагах, а в доступі до обчислення head; це чесний мінус, який ми маємо враховувати.
- NanoJev run: Astra змінив **тільки `scalar.weight` (1 024 ваги)**, 68 ходів, 41 Python run, 167 хв, $6.92.
- d1-2B run: `layers.23.ff.w2.weight`, **всі 15.7M entries rank-9**, 65 ходів, $5.42.

**Що це означає для нас:** Astra ≠ RL, ≠ fine-tune, ≠ training loop. Це *діагностичний+хірургічний*
інструмент поверх готового чекпоінту, з бюджетом forward passes і фіксованими eval-наборами.

---

## 4. Порівняння: Jev-verification vs Astra-WSP (abstract)

| Вимір | NanoJev/Jev (capability proof) | Astra WSP (weight fixing) |
|---|---|---|
| Ціль | довести/вибрати прийнятну поведінку | усунути причину зламаної поведінки |
| Метрика | task success на frozen cases (4/10, 128/128, …) | Δ на held-out бенчах + маркери-фікшни (token-limit) |
| Бюджет | датасет + dev-split selection | forward passes (10/25/50) + kernel над вагами |
| Верифікація | SHA256 манифест + незалежний replay | Eval на не-тренувальних сетах + порівняння «until token limit» |
| Що змінюють | навчання (SFT) → вибір чекпоінту | 0.016 % ваг (1 напрямок) |
| Ризик | overfit на 4 tasks (showcase 50×50 ≠ benchmark) | підгляд на test (заборонено протоколом) |

---

## 5. Наш кластер — що реально є (стан 2026-11-18)

**Локальні GGUF/артефакти:**

| Модель | Розмір | Де | Стан |
|---|---|---|---|
| MiniCPM-V-4_7-1B-Q4_K_M | 672 MB | `rusty-engine/data/models/` (копія) · `target/release/patched_{11952,22856,26568}.gguf` (3 WSP-триали) | reference/evaluation для runner35; **дві наших Inkling-guided спроби ROLLED BACK** (reasoning 43→33; l23 ffn_down zero / l15 attn_output zero) |
| MiniCPM-V-4_7-35B-A3B-Q4_K_M | 19.7 GiB | `rusty-engine/data/models/` | MoE 256.experts/8-top, 40 layers, shexp блоки вже на пірах (blk.0–blk.39) |
| qwen2.5-0.5b-q4_k_m | 491 MB | `rusty-engine/data/models/` | codegen FAIL (усе інертне) |
| Qwen2.5-Coder-1.5B-Instruct | ~1 GB | `llama-rs/models/` | codegen FAIL на бенчі |
| Qwen2.5-0.5B-Instruct | ~0.5 GB | `llama-rs/models/` | codegen FAIL |
| granite-3.1-3b-a800m-instruct-Q4_K_M | ~1.8 GB | `llama-rs/models/` | MoE, найкращий MoE в кластері (imatrix) |
| L: mirror `gsv_mirror/wgpu_weights` | ~180 MB / 46+ chunks | L: (ізольовано) | WeightStore mirror 35B shexp |

**Піри (mesh):** #35 valhall (arm, 100 MFLOPS hist), #36 pi4 (arm a72, **зараз alive=false у mesh capability block; Pi online earlier**), #37 Firefox PC (**ghost, alive=false, proofs_fresh=0** — не для роботи), #38 bifrost (102 MFLOPS, best_peer). 35B-A3B 40/40 блок-операцій виконано пірами (max_err ≤ 3.5e-6).

**Хмарні writers, які нам реально доступні (2026-11-18):** OpenCode Zen free — Flint 40B (1) ·
GLM 5.3 (2) · DeepSeek 1017 (3) (+ Zen fallbacks). **GPT-6 Astra у нас немає і не буде.**
Усі наші writers = frontier-lite; очікувана якість на рівні 「hypothesis testing」, не Astra-tier.
OmniRoute (:20128) і poolAI :8091 — policy-down, **не використовуємо**.

**Відомі інструменти в кластері:** WeightStore + block-op dispatch (FW), `/api/mesh/placement`
(greedy, readiness-gated), `/api/wgpu/*` WS push + telemetry + console, stand-smoke,
`ActivationAudit` (дизайн ECO-9.2; НЕ реалізовано), ExpertCache (PH-404 — open),
`block-bitmap patch-at-dequant` (дизайн; реалізовано лише offline rewrite через requantize у тріалах).

---

## 6. Порівняння і план: cloud writer + local models → placement mapping → MoE improvement

### 6.1. Що змінюється / що ні — чесний вердикт

- **«Astra-якість» нам недоступна.** Найближчий за якістю writer, який у нас є, — GLM 5.3
  (Zen free, 3 черговий пріоритет для коду). Astra-tier результатів (+40 MATH-500) очікувати не можна;
  реалістична ціль — **iterate + measure** на замороженому battery.
- **Gemma-закон діє й тут:** MiniCPM-1B/Qwen2.5-* у нас усе інертні в codegen — це «ability is
  lacking», а не «obstacle». WSP їх не полагодить. Кандидат на WSP = лише «unlock»-сценарій
  (щось, що модель вже вміє, але не показує через дефект у вагах).
- **Наш найкращий кандидат-на-«loop»**: MiniCPM-V-4_7-1B (runner35 = **qwen35-hybrid**: 18 GDN-шарів
  + 6 full-attention, vocab 248 094 — архітектуру підтверджено B13 `PH-1301..1304` у rusty-engine)
  на math/thinking-поведінці; аналог Astra Experiment 8, але з GLM/Inkling як writer.
  **Базовий вимір зроблено** (`docs/gsv/wsp-battery/WSP_AUDIT_RUNS_2026-11-18.md`): uk-fact-2
  (відома провальна UA-фактичність) дає layer-23 full-attn `delta_rms_max=0.4538` — вище за всі
  решту промптів (uk-fact-1 0.3486, codegen-1 0.4381). Причинність **не доведено**; потрібні
  uk-golden battery (власник) + budget'ed editorial passes.
  Попередні 2 спроби (Inkling 2 Small Free-guided) **відкочено** — це нормально: Astra-рівень
  треба репетирувати на frozen eval, перш ніж вірити.
- **35B-A3B MoE** — WSP-lane не для роутерів: **freeze router/gate** (ECO-9.2 guard #1).
  Латеральний простір для WSP там — expert FFN блоки, і тільки після ActivationAudit.

### 6.2. Frozen evaluation battery (Jev-style, щоб наші прогони були порівнянними)

Мінімум, який треба зафіксувати **до** будь-якого патчу (SHA256 манифест кожного набору):
1. UA/UK golden: 50 фікстур укр. мови (до/після) — власний набір;
2. Math mini-set: 100 задач (GSM8K-style, UA/EN), фіксовані;
3. Thinking-loop probe: 20 prompts; критерій = частка answers що впираються в token limit;
4. Instruction-following mini-set: 25 prompts (IFEval-style, спрощено);
5. Code probes: 30 синтетичних задач (наші — будь-що маленьке), очікуваний fail-baseline збережений;
6. Uk-n. golden fixture reuse з rusty-engine (уже існує).

Кожна прогонка патчу: фіксовані зразкування (T=0.6, top-p 0.95, top-k 20, presence penalty 1.5 — як у Astra Exp.8 для порівнянності).

### 6.3. Placement mapping (мапінг розміщення) — пропозиція

| Клас ваг/тензорів | Де живе | Піри | WSP? | Механізм |
|---|---|---|---|---|
| **MoE router/gate (ep-router, gate_inp)** | 35B backbone | **НІКУДИ** (freeze) | ❌ ніколи | — |
| **Expert FFN (ffn_gate/up/down_shexp, 256 experts)** — blk.0–39 | WeightStore chunks (L: mirror) | bifrost #38 (primary) · valhall #35 (secondary) · FF-PC #37 — **виключено** | ❌ patch заборонено до ActivationAudit | block-op dispatch (R4) |
| **Dense hybrid 1B (GDN + 6 full-attn)** | rusty-engine serve | local CPU (runner35) | ✅ кандидат №1 (loop пошук) | patch-at-dequant bitmap (дизай.) / offline GGUF rewrite |
| **Granite-3.1-3B-A800M (Q4_K_M, MoE A800M-active)** | llama-rs/models | local CPU або Pi4 (matvec-only) | ⚠️ тільки після Uk-golden + не без owner | llama.cpp offline rewrite |
| **Qwen2.5-Coder-1.5B / 0.5B** | llama-rs/models | local CPU | ❌ (ability lacking — WSP не допоможе) | — |
| **Attention/KV/GDN state** | local CPU | — | ❌ (не в скоупі) | — |
| **mmproj (vision)** | rusty-engine | local CPU | ❌ | — |

**Як вибирати піра під блок:** greedy + readiness-gate (уже в хабі, `proof_gate`), додатково:
- `patch_tier` колонка в карті розміщення: `never-patch` / `audit-candidate` / `patch-ready`;
- 35B expert-блоки → browser peers тільки (Pi4 виключено, як у R4);
- bandwidth/latency EWMA (`ewma_elapsed_ms`) для пріоритетів.

### 6.4. Покращення MoE через пірів — конкретна схема (docs-only, owner-gated)

**Фаза A (audit, без патчів):** ActivationAudit по shexp-тензорам 35B-A3B через mesh-пірів —
збір per-expert mean/std/z-score статистик на фіксованому probe-наборі (UA golden + math mini).
Вихід: таблиця `{blk, tensor, expert_id?, drift_z, spike_flag}` → candidates list.

**Фаза B (діагностика, GLM/Inkling writer):** Python-kernel над CAN-мережами (сегменти лоад-юнітів);
запит до GLM 5.3: «знайди напрямок, що женеться за loop/overfitting у шарі L» — на зразок Astra Exp.8,
але (а) тільки на 1B dense моделі спершу; (б) бюджет 10→25→50 forward passes; (в) кожен крок — через Uk-golden.

**Фаза C (apply, behind feature-flag):** block-bitmap patch-at-dequant у rusty-engine
(`patched_*.gguf` зберігати поруч, ніколи не перетирати оригинал — поточна практика) або
staged offline rewrite → restart (llama-rs). Rollback = скинути bitmap.

**Фаза D (validate, quorum):** 2–3 піри застосовують патч на свої репліки, проганяють battery,
підписують Result; прийняття = кворум. У нас немає Solana-econ до цього — **без on-chain**, просто `.jsonl` voting log у `data/`.

**Фаза E (MoE-specific):** Після audit — hot-expert cache (PH-404 ExpertCache LRU + NVMe offload)
з пріоритетом для experts, що дали спайки в audit. Це "покращення MoE без патчу ваг" — найдешевший
і найбезпечніший крок, повністю в нашому мануальному контролі.

### 6.5. Що НЕ робимо в цій смузі

- Ніяких змін router/gate-ваг (ни 35B, ні Granite).
- Ніякого codegen-у на Astra-рівні на наших writers (ability gap — див. Gemma-закон).
- Ніяких патчів без Uk-golden + sense-check «hit token limit ↓».
- Ніякого перемішування з ECO-8.2/8.3/9.x.
- Ніяких patch-merge на живий хаб поза `patched_*.gguf`-копією; original не чіпаємо.

### 6.6. Питання до owner (gate)

1. Який перший таргет: (a) MiniCPM-1B loop-unlock (ближчий аналог Astra Exp.8), (b) Granite A800M audit-кандидат, (c) 35B shexp ActivationAudit?
2. Який uk golden battery — хто веріфікує склад (50 фікстур)?
3. Чи дозволяємо патчі на 1B (672 MB версії) як тренувальний майданчик?
4. Бюджет Zen-дзвінків на діагностику (GLM 5.3 — скільки сесій?).
5. Чи продовжуємо після першого rollback? (нині: 2 невдачі, 0 успішних патчів).

---

## 7. Підсумок

Наш кластер вже має **всю механіку** для WSP-експериментів у малій формі (WeightStore, block-op,
placement, patched-gguf конвенція). Чого немає — **таланту Astra**: GLM/Flint/DeepSeek не діагностують
ваги так глибоко. Тому реальний план = **Jev-style верифікація + Astra-style метод**, але
результати вимірюємо нашими метриками, і очікуємо малий виграш. Найдешевший і найнадійніший
крок для MoE — **ActivationAudit → ExpertCache LRU** (фаза A+E), без жодного патчу ваг.

Далі по плану: тільки **docs-only** роботи (ActivationAudit spec, battery manifest) + рішення owner'а.
