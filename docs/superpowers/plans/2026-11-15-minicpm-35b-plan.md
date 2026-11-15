# Plan: MiniCPM-V 4.7-35B-A3B (35B-A3B) — rusty-engine MoE flagship

**Дата:** 2026-11-15 · **Owner:** скачати на диск (наявність), далі — розселити експертів по девайсах
**Продукти:** `rusty-engine` · `gsv-wgpu` · `gsv` · `poolai` · `llama-rs`
**Статус:** stage = model on disk; **запуск тут і зараз неможливий** (RAM)

## 1. Завантажено й верифіковано (2026-11-15)

| Файл | Розмір | sha256 (збігається з HF) | Примітка |
|---|---|---|---|
| `data/models/MiniCPM-V-4_7-35B-A3B-Q4_K_M.gguf` | 21 166 255 008 B (~19.7 GiB) | `9ecf91e814d7e9adc64325b1b65f8628a9b1bb8da4f61a92869ca7489bfd888e` ✅ | основний MoE |
| `data/models/mmproj-model-f16-35b.gguf` | 1 119 575 840 B (~1.0 GiB) | `bff3b688e14c300eca273092c37c50a0ddfcc2054e079af1a0480edeed5d20f1` ✅ | vision projector (1B mmproj окремо: `mmproj-model-f16.gguf` Oct 24) |

Модель = MoE: 35B усього / ~3B активних (8 із 256 experts на токен). Architecture `qwen35moe`;
llama.cpp підтримує цей формат з PR #29416. Діагностика без запуску: `rusty inspect`.

## 2. PH-1201 spike (виконано 2026-11-15, inspection only)

`rusty inspect` показав (зчитано, не виконано обчислень):

- `architecture = qwen35moe` · **40 блоків** · **256 experts**, `expert_used_count = 8` ·
- Embedding 2048 · vocab 248144 · tokenizer `qwen35` · context 262144 · M-RoPE zyxt,
  section_order `[11,11,10,0]` · quantization_version=2 · file_type=15 (Q4_K_M) ·
  733 tensors · 19.5 GiB ваг (ffn_*_exps)
- `ssm` GDN hybrid: conv_kernel=4, state 128, group 16, `full_attention_interval=4`
  · recurrent layers + full-attn every 4th · `ffn_gate_inp.weight` [2048,256] F32 (роутер ваг)
- Не виконувалось: 35B на цьому блоці (7.7 GB RAM, ~0.8 вільних під час збірки/serve).

## 3. Далі (передбачає виділений вузол ≥ 24 GiB RAM або cloud-варіант)

**LIVE ДЕМО НА 35B (2026-11-15, група `g-73`):** піри виконали реальну 35B-вагу
`blk.0.ffn_down_shexp.weight` (Q6_K, 1 shared-expert блоку 0, dims [512,2048]) —
обидва шарди over WebGPU на Android, `max_err_model` ≈ 1.4e-6 / 7.2e-7 (хаб верифікував).
**Mesh ⇔ 35B-ваги працює.** Це ще не MoE forward (немає router/GDN/40-блокового циклу),
але транспорт/доріжка дозріла.

1. **PH-401..404** (MoE core, на вузлі): router top-k · expert FFN shared/specialized · ExpertCache LRU.
2. **PH-406**: 35B E2E greedy-паритет (mmap partial/sharding на вузлі).
3. **PH-1204..1206**: vision / serve / 35B node.
4. **MC-1** ExpertParallelDispatch (mesh) — тільки після PH-401+ на 35B.
5. **Uk-якість**: 35B має суттєво кращу нативну Uk-перфоменс, ніж 1B.

## 4. Чого НЕ робити

- Не запускати 35B тут — RAM цього блока не вистачить (~0.75 GB вільних під навантаженням).
- Не чіпати `data/models/*` (gitignored); не комітити ваги.
- Не плутати ммproj-и 1B та 35B: використовувати `mmproj-model-f16-35b.gguf` для 35B.

## 5. Stop-умови (виконано 2026-11-15)

- ✅ sha256 обох 35B GGUF збігається з HF
- ✅ `df -h /s` — ≥ 80 GB вільних після скачування
- ✅ `rusty inspect` на 35B показує arch/headers (40 blocks, 256 experts, qwen35moe)
- ✅ dump `blk.0.ffn_down_shexp.weight` → 4 MiB fp32 зареєстровано на хабі
- ✅ **LIVE: mesh на реальних 35B-вагах** (група `g-73`) — обидва шарди `done`,
  `max_err_model` ≈ 1.4e-6 / 7.2e-7 (валідація на хабі)
- ❌ повноцінний 35B-inference на цьому блоці — не робити (RAM; потрібно виділений вузол)
