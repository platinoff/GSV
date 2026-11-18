# WSP pack-mint + serve --patch — canary/ablation evidence (2026-11-18, пізній вечір)

**Мета:** перевірити повний WSP-ланцюг (mint → parse → stage → patched serve) без жодного патчу «наосліп»,
і зібрати першу каузальну (а не кореляційну) інформацію про `blk.23.attn_output.weight` у MiniCPM-V-4_7-1B-Q4_K_M.

**Інструменти:** `rusty pack-mint` ([identity / zero / all-zero]) · `rusty serve --patch` (стадить `patched_<pid>.gguf`
копію поруч екзешника, оригінали не чіпає) · ChatML, temp 0. **Жодних Zen-дзвінків, жодного NN-коду, жодних патчів оригіналів.**

## Pack format (практика)

`<tensor> <block_idx> <hex-block-bits> <base_sha256>` — base sha256 звіряється при stage; mismatch = stage fail (безпечно).
Identity pack (0-байт-змінний `d90a2216…`) пройшов verify → доказ, що ланцюг працює end-to-end.

## Результати

Модель-оригінал (baseline, з попереднього слайсу): uk-fact-1 → «Киен.» · uk-fact-2 → «Іван Іванович Ох…» ·
math-1 → `<think>\nDle am I asked:` · loop-1 → англ. reasoning без фінальної відповіді (token-limit 160).

| Експеримент | Порт | Результат |
|---|---|---|
| A. Identity-canary (1 блок, без змін) | 8093 | **Ідентично baseline** (Киен. / Іван Іванович Ох / Dle am I asked:) ⇒ ланцюг pack→stage→serve коректний |
| B. Zero 1 блок blk.23.attn_output.weight | 8094 | **Ідентично baseline** (256/2 097 152 ваг = нічого не ламає) |
| C. Zero ВСІ 8192 блоки blk.23.attn_output.weight | 8095 | Виведення змінилось, але лишилось у тій самій категорії помилок: uk-fact-1 → «Киенєве місто є»; uk-fact-2 → «Іван Філіпоновitch»; math-1 → `<think>\nD'abord, calcule`; loop-1 → «многостроений добуток». **НЖОДЕН факт не розблоковано; жодна задача не виправилась** |

## Висновки

1. **WSP-пайплайн функціональний end-to-end** (canary identity → біт-у-біт ті самі outputs).
   Це те, чим ми володіємо і що можемо використати для осмислених ablation-студій — без Astra.
2. **Шар 23 attention output НЕ є «обов'язковим мостом фактологічності»**: повне його занулення
   міняє декодування, але не додає вміння. Узгоджується з попередньою знахідкою
   (shared-prefix artifact) і з Gemma-законом: модель не має вміння, яке можна «відімкнути».
3. **Single-block ablation = нульовий ефект** (як і очікується для 1 блок/8192).
4. Закритий маршрут: «знайти один напрямок/декілька колонок, які ховають знання, і відняти його» —
   на MiniCPM-1B цього немає; наші попередні 2 триали (layer-23/15 zeroing через Inkling) провалені
   тим самим експериментальним шляхом.

## Що реально можна робити далі (усі owner-gated)

- C→B-серія на **Granite-A800M** (llama.cpp, CPU): та сама методологія (identity canary → zero targeted FFN expert
  blocks), але на моделі з реальними MoE-експертами і, можливо, іншим розподілом ability-gap.
- Потребує: baseline battery на Granite (bake battery v1: потрібна uk-golden — від owner).
- **Не патчити оригінали GGUF** будь-коли; `patched_*.gguf` тільки копії (конвенція зберігається).
- Inkling-only режим: без Zen-дзвінків без owner-апруву.

## Cleanup

Усі тестові серви зупинено (Windows PIDs 12008/28208/26572; MSYS virtual PID ≠ Windows PID — taskkill приймає тільки Windows PID).
`target/release/patched_*.gguf` (6 копій × 672 MB) видалено. Оригінали `data/models/*.gguf` не чіпались. Пакети
(identity/zero/all) лишились у `/c/Users/plati/AppData/Local/Temp/opencode/pack_*.wsp` (~1.2 MB для identity, 8193 рядків для all-zero).
