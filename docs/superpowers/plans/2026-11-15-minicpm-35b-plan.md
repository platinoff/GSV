# Plan: MiniCPM-V 4.7-35B-A3B (35B-A3B) — rusty-engine MoE flagship

**Дата:** 2026-11-15 · **Owner:** скачати на диск (наявність), далі — розселити експертів по девайсах
**Продукти:** `rusty-engine` · `gsv-wgpu` · `gsv` · `poolai` · `llama-rs`
**Статус:** stage = скачування (цей документ); **запуск тут і зараз неможливий**

## 1. Що зроблено (2026-11-15 вечір)

Запущено завантаження двох офіційних GGUF (Apache-2.0, OpenBMB, `openbmb/MiniCPM-V-4.7-35B-A3B-gguf`):

| Файл | Розмір | sha256 (HF API) | Примітка |
|---|---|---|---|
| `MiniCPM-V-4_7-35B-A3B-Q4_K_M.gguf` | 21 166 255 008 B (~19.7 GiB) | `9ecf91e814d7e9adc64325b1b65f8628a9b1bb8da4f61a92869ca7489bfd888e` | основний (MoE, qwen35moe, 8 експертів на токен тощо) |
| `mmproj-model-f16.gguf` | 1 119 575 840 B (~1.0 GiB) | `bff3b688e14c300eca273092c37c50a0ddfcc2054e079af1a0480edeed5d20f1` | vision projector |

Модель = MoE: 35B усього / ~3B активних на токен; архітектура `qwen35moe`; llama.cpp підтримує з PR #29416.

**Important limit (чесно):** ця машина — 7.7 GB RAM, ~0.8 GB вільних під час збірки/serve.
**Запустити 35B навіть з mmap тут неможливо** (падіння в своп/kill; MemFree 750–840 MB).
Модель на диску = підготовка до **виділеного вузла**, не до локального serve.

## 2. Обсяг робіт (порядок)

1. **PH-1201** (spike): `rusty inspect` на 35B — структурна перевірка qwen35moe
   (per-layer router, expert weights, M-RoPE metadata). Без запуску. Потрібен тільки download + inspect.
2. **PH-401..404** (MoE core, є на WO-дошці): top-k router за замовчуванням, expert FFN,
   shared-expert, ExpertCache LRU — саме для 35B.
3. **PH-406** (35B E2E greedy-паритет): очікується на виділеному вузлі / після PH-1305-проходу;
   тут результат — детектор ну і тест corrupt-реліза.
4. **PH-1204..1206**: vision / serve / 35B node / ExpertCache.
5. **MC-1** ExpertParallelDispatch (mesh) — тільки після PH-401+ ядра на 35B.

## 3. Чого НЕ робити

- Не запускати 35B на цьому блоці — воркер-відпади, а 35B-mmap у 6.9 GB вільних → занадто нижньобортовий swap.
- Не міняти каталог `data/models/` одночасно з downloads (уникати PING UPDATE HALF).
- Не комітити `data/models/*` (ігнорується, вернино).

## 4. Stop-умови (перед PH-1201)

- sha256 обох файлів збігається з таблицею вище.
- `df -h /s` після завантаження — ≥ 80 GiB вільних залишилось.
- `rusty inspect` на 35B показує 24 повні шари, `qwen35moe` arch keys, >= 8 experts.
- Виклик `poolai`/`GSV` — після того, як скачування закінчиться; цей документ фіксує статус.
