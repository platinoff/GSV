# NanoJev → GSV/Rust — feasibility study (2026-11-18)

**Status:** feasibility / research — **no code, no downloads yet, owner gate for Tier 2**.
**Question:** що з NanoJev реально можливо в екосистемі зараз (і далі), беручи до уваги стан
кластера/пірів/бокса.

## 1. Що таке NanoJev (перевірено 2026-11-18, джерела в §6)

- C-Tianyu/NanoJev — «nano replica of Jev»; один чекпоінт для 4 задач: **Maze (10), Snake (8),
  ViZDoom Basic (128), Predict Position (128)**.
- Backbone: **Qwen/Qwen3-0.6B@c1899de289a04d12100db370d81485cdf75e47ca**, SFT на mixed-task CE.
- Формат: `best.safetensors` + `config.json` + `backbone_config/` + `tokenizer/` — **немає GGUF,
  немає PyTorch-runtime у цьому робочому просторі (kit rule: Rust-only)**.
- Choice head: softmax над candidate set; boolean/ordered-score request types.
- Архітектура Qwen3-0.6B: 0.6B, **28 layers**, hidden 1024, GQA (потрібно звірити кількість
  heads/KV з `config.json` щойно буде доступ). Vocab ≈ 151 936.
- Ліцензія: MIT (sidecar) + Qwen base (apache-2.0) — для research/eval прийнятно.
- SHA256_MANIFEST.json у релізі — основа для верифікації завантаження.

## 2. Що у нас є / чого немає (стан 2026-11-18)

| Компонента | Статус |
|---|---|
| NanoJev weights локально | **відсутні** (find по target/live/data — 0 збігів) |
| GGUF Qwen3-0.6B | у кластері немає; на HF є офіційний `Qwen/Qwen3-0.6B-GGUF` (apache-2.0) |
| `llama_serve.exe` (PC) | **мертва** (silent exit 0 / DLL import mismatch — секція «Granite blocked» HANDOFF) |
| `llama_serve` (Pi4, aarch64) | **жива** (smoke ✓, Qwen2.5-0.5B 2026-11-17) — єдиний робочий llama.cpp вузол |
| rusty-engine arch support | **qwen35 (GDN-hybrid) + qwen2moe**; qwen3-dense — **немає**; safetensors — **немає** |
| BPE tokenizer | є, generic-capable (`tokenizer.rs`), але wiring заточений під qwen35 vocab 248 094 |
| RAM бокса | 7.7 GiB total, 89 % під навантаженням; EDR-тихі-смерті — відомий інцидент |
| Pi4 | alive, 3 GB RAM, wlan, підходить для control-прогону (повільно) |
| Python/PyTorch | **заборонено правилами GSV** для цього робочого простору |
| Zen writers доступні | **тільки Inkling 2 Small** (owner) — без окремого go не використовуємо |

## 3. Варіанти

### A. Tier 1 — «control path» через Pi4 + офіційний Qwen3-0.6B-GGUF (≈0.5 дня, 0 коду)
1. На Pi4: завантажити `Qwen/Qwen3-0.6B-GGUF` (Q8_0 або Q4_K_M; ~600 MB) у `llama-rs/models/`.
2. llama.cpp уже зібраний на Pi4 → запуск `/v1/chat/completions` reasoning/control smoke.
3. Зафіксувати: **architecture відомий працює на нашій llama-гілці**, швидкість tok/s, пам'ять.
4. **Обмеження чесно:** це **не** NanoJev. Це базовий Qwen3-0.6B — поведінка NanoJev вимагає
   його `best.safetensors` + Choice head + SFT-міркування. Використовуємо лише як
   референс-точку (конфіг/токенайзер/швидкість) і саніті-чек llama.so на aarch64.

### B. Tier 2 — Rust-нативний повний порт NanoJev (2–4 дні, ~1200–1600 LOC)
Компоненти (усі нові, у `rusty-engine` або новому crate `nanojev-runner`):
1. **safetensors reader** (mmap, без повного копіювання): header JSON → tensor views.
2. **Qwen3-dense forward** (28 L, GQA, SwiGLU, RMSNorm, RoPE): logits + hooks для Choice head.
   ~400–600 LOC; чисельна валідація проти `rusty dump-f32` logits на spike.
3. **Choice head**: candidate-set softmax; booleans/ordered scores; bool::false/true variants.
4. **Tokenizer**: rewire existing BPE loader під 151 936 vocab (перевірка special tokens з
   `tokenizer_config.json` NanoJev, не qwen35-спеціалки).
5. **Maze eval harness** (найпростіший env; Snake — друга черга; ViZDoom/Predict Position — out
   of scope, бо native libs/visual expert data).
6. **Sanity target**: ~4/10 Maze (як у релізі) — не гейт, не обіцянка.
Ризики: тонкі розбіжності в RoPE/GQA/rms-gates; токенайзер; RAM (0.6B bf16 ≈ 1.2 GB — mmap
обов'язково, без копій); натиск часу на боксі з EDR; ніякого Python fallback.

### C. Tier 3 — Snake + Predict Position (тиждень+)
Тільки після B; Predict Position потребує — upstream visual-expert дані («Sonic Doom visual
expert») — завантаження + формат; низький пріоритет.

### D. Не портувати NanoJev взагалі
35B-A3B mesh — виробничий шлях; NanoJev — дослідницький код, який не бере участі ні в одному
production feature. Якщо owner'у не потрібен артефакт чи die-go — відкласти або закрити.

## 4. Рекомендація

**Tier 1 зараз** (якщо owner схвалить завантаження на Pi4) — дає відповідь «чи запускається
Qwen3 в нашому стеку» без жодного рядка коду. **Tier 2 — тільки з явного go** і **не на цьому
нічному боксі** (RAM/EDR). **Tier 3 — не починати.** NanoJev НЕ підміняє 35B-трек, WSP-трек
і Game/RL-реплікацію.

## 5. Що НЕ робимо

- Без Python/PyTorch навіть тимчасово (kit rule).
- Без конверсії safetensors→GGUF «наосліп» (нема чим/не потрібно).
- Без реклами «NanoJev працює в GSV» до сходження Maze eval.
- Без Zen-дзвінків без owner-апруву.
- Без впливу на 35B mesh роботу; окремий репозиторій-артефакт, окремий банд.

## 6. Джерела

- HF: `C-Tianyu/NanoJev` (README, TRAINING_RECIPE.md, SHA256_MANIFEST.json)
- dspy-wsp typed-decisions-Astra (`cmpnd-ai/dspy-wsp`, results/2026-11-07-typed-decisions-astra)
- GSV: `docs/gsv/WSP_JEV_ASTRA_RESEARCH_2026-11-18.md` (§2 Jev/NanoJev протокol + таблиці)
- `Qwen/Qwen3-0.6B-GGUF` (apache-2.0, canonical GGUF of the backbone)

## 7. Наступні рішення owner'а

1. Tier 1 дозволити? (завантаження ~600 MB на Pi4)
2. Tier 2 go / no-go (Rust-нативний порт; окремий банд у rusty-engine)?
3. Якщо no-go для обох — закрити NanoJev-трек і зафіксувати в HANDOFF «not pursued».
