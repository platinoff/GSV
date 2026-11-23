# Owner note — 4-квант + покращений блок зору на кластері (2026-11-23)

**Дослівно (UA):** «ми тянули ваги у з моделей що є у расті в здеркало і роббили
gsv versus ai 4 квант тепер треба покращений блок зору як inking 2 має,
запускаємо на кластері, перевіряємо роутінги, тестуємо бенчмаркуємо, зберігаємо
данні для покращення ваг, - нотатка»

## Що це означає як вимога

1. **Конвейєр, що вже є:** ваги з моделей у Rust-дереві → дзеркало `L:/gsv_mirror`
   → `gsv-versus-ai` **4-квант** (`gsv-versus-ai-full-q4k.gguf`, Q4_K/Q6_K,
   733/733 тензори, sha256 `2f37d609…`). Не перевизначати й не псувати канон F32.
2. **Ціль:** **покращений блок зору рівня Inkling-2**, приєднаний до 4-кванту.
   Не просто F16-mmproj «як є», а 4-бітний (`q4_k`) варіант.
3. **Робота на кластері:** запуск на живих Min-пірах (`#37` RDNA2 + `#39`
   bifrost), перевірка **роутингів**, тести, **бенчмарки**.
4. **Зберегти дані для покращення ваг** — тобто результати liner/роутинг/бенч
   мають бути записані як вимірювані артефакти, а не лише в чаті.

## Факти, з яких починаємо

| Факт | Доказ |
|---|---|
| `gsv-versus-ai-full-q4k.gguf` = 733 текстові тензори, **жодного vision-тензора** | `publish.json`, artifact-report, `GASV_A3` (`mmproj/vision → excluded`) |
| `mmproj-F16.gguf` = Inkling-2 vision block, 422 тензори, clip/`inkling2v`, F16/F32, **projection_dim 2304** | `rusty inspect` 2026-11-23 |
| Кластерний лейн уже вміє Q4_K/Q6_K artifact-backed `weights_matvec` з верифікацією `max_err_model<1e-4` | G20/G21/G22/G23, `src/boxes/wgpu_dequant.rs` |
| Inkling-2 **не геометрично сумісний** з versus-ядром (2304 ≠ 2048); сумісний `mmproj-model-f16-35b.gguf` (2048, той самий `general.name`) існує | `rusty inspect` обох projector-файлів |
| Жоден локальний сервер не вантажить mmproj (rusty/llama text-only; C++ llama-server не зібраний), PH-1202/1204/1205 відкриті | G18 verdict, `tickets.jsonl` |

## Що зроблено в цьому зрізі (G32)

- `rusty quantize-export` розширено: приймає F16 (ggml=1) і BF16 (ggml=30)
  джерела через уже наявний `quant::dequant` — **це дало 4-бітний блок зору**,
  а не F32-дамп.
- Утворено два versioned Q4_K companion-и в дзеркалі (git weights — заборонено,
  лежать тільки в `L:/gsv_mirror`):
  - `gsv-versus-ai-vision-q4k.gguf` — 311,613,052 B ( геометрія 2048, сумісна з versus );
  - `inkling2-vision-q4k.gguf` — 179,019,072 B ( блок зору Inkling-2, 2304 ).
- Далі: реєстрація Q4_K-тензорів як artifact-backed у GSV + канари на `#37/#39`,
  перевірка роутингів, бенчмарки, збереження вимірів.

## Live-результат (2026-11-23, Min-кластер)

Додано `cargo xtask vision-canary` (GSV-G32): реєструє Q4_K-тензор з дзеркала
як artifact-backed, ставить `weights_matvec` на живі піри й пише доказ у
`data/vision_q4_bench.jsonl` (gitignored) — роутинг + elapsed_ms + max_err_model.

| Артефакт | Тензор | Група | Шарди | `max_err_model` | Роутинг (peer_id за шардом) |
|---|---|---|---|---|---|
| `gsv-versus-ai-vision-q4k.gguf` (2048, сумісний) | `v.blk.0.ffn_up.weight` | `g-739` | 8/8 done | **1.39e-17** | `[37,39,40,37,37,39,37,40]` |
| `inkling2-vision-q4k.gguf` (блок зору Inkling-2) | `mm.1.weight` | `g-740` | 8/8 done | **6.94e-18** | `[39,40,37,37,37,37,37,37]` |

Обидва < `1e-4` — **реальні 4-бітні ваги зору виконуються на кластері**, з
перевіркою роутингів і збереженими бенчмарками (per-shard `elapsed_ms`: 18.7…
2633 мс для g-739; 38…149 мс для g-740). Канон F32 і 4-квант тексту не змінено.

## Продовження (G33) — блок зору як forward, а не лише окремі ваги

Додано **result bus** (`GET /api/wgpu/task/result?group=<id>` у
`src/boxes/wgpu.rs` + `server/mod.rs`): хаб тепер читає перевірені peer-частки
з `wgpu_tasks.jsonl` і повертає їх за `shard_index`. Це закриває відсутню
половину вагового лейна — результат кластера більше не «пишеться в JSONL і
забувається», а стає входом для наступного шару.

`cargo xtask vision-forward` (`src/boxes/vision_forward.rs`) виконує 4-токенний
(class + 3 patch) forward Inkling-2 Q4_K Vision Transformer: **усі лінійні
шари (patch_embd, Q/K/V, attn_out, ffn_up, ffn_down, mm.1, mm.2) ідуть на
кластер** через `weights_matvec`; хаб лишає LayerNorm, attention-softmax,
GELU та residual--scale. Ваги Q4_K range-сервуються з дзеркала як
artifact-backed (без F32-матеріалізації й без git-блобів).

| Прогон | Блоків | Cluster matvec-ів | shard'ів на matvec | Час | `max_abs` виходу | `ok` |
|---|---|---|---|---|---|---|
| 1 | 1 | 27 | 4 | 230 s | 2.61 | true |
| 2 | 3 | 75 | 2 | 413 s | 2.44 | true |

Обидва дали скінченний 2304-вимірний Inkling-2 vision-вектор; роутинг і
per-shard `elapsed_ms` збережено в `data/vision_q4_forward_bench.jsonl`
(gitignored). Повний 23-блоковий прогон оцінюється ~30–90 хв за поточних
shard-налаштувань — робиться окремим запланованим смугом, не в цій сесії.

## Межі (чесно)

Це **ваговий/роутинг-рівень**: кластер виконує реальні Q4_K-тензори зору з
перевіркою похибки. Повний `image → answer` forward потребує PH-1202/1204/1205
(ViT + image-prefix + multimodal serve) і G13; без них жодного quality-claim.
Gemma-law: покращення ваг має оцінюватись лише за збереженими вимірами, не за
«здається краще».
