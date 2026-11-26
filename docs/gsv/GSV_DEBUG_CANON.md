# GSV_DEBUG_CANON — канон дебагінгу (2026-11-26)

**Статус:** канон-план, сумісний зі сталевими станами проєктів. Джерело методу:
`.agents/skills/systematic-debugging/SKILL.md` + `debug/SKILL.md` (дзеркала в
`.cursor/skills/` і `.opencode/skills/` через `cargo xtask mirrors`).

## 1. Сталеві стани (звірено 2026-11-26)

| Що | Значення |
|---|---|
| GSV | HEAD `12bc953` (GASV G32–G49), `origin/main` синхронний, рабоче дерево чисте |
| rusty-engine | HEAD `34b3363` (PH-1204 `--vision-at`, PH-1205 `vision_prefix_b64`), чистий |
| Екосистема | 14 registered Rust products + 2 unregistered git; усі мають `rust-toolchain.toml` (11 × `stable-x86_64-pc-windows-gnu`, 2 × `stable` host-agnostic); факточний rustc **1.100.0** всюди |
| Кластер | pіри `#43` PC + `#39` bifrost + `#44` valhall `alive/ready`; `#43` authoritative для Q4_K reshape matvec |
| Артефакт | `gsv-versus-ai-full-q4k.gguf` 733/733, sha `2f37d609…`; канон F32 `25fadfbb…` не змінено |
| Vision | Q4_K companions 2048 / 2304 / 1024; повні forwards 23/23, 27/27, 27/27 × 4 токени; `vision_prefix_1024_mt4_fixed.f32` (архітектурно виправлений) |
| Гейти | `cargo test` 52 suites / 0 failed · clippy `-D warnings` · loc-audit 99.66 % · rules-check 5/5 · sync rev 516 · MCP tool_count 61 |

## 2. Метод (не змінювати)

1. **REPRODUCE** — чіткий мінімальний випадок (команда + вхід + очікуваний/фактичний результат).
2. **ISOLATE** — знайти найменшу одиницю, що відтворює баг (unit-тест або окремий endpoint).
3. **DIAGNOSE** — root cause ДО будь-якого фіксу; 3+ невдалих фикси ⇒ переглянути архітектуру, а не продовжувати патчити.
4. **FIX** — TDD: спершу падаючий тест, потім最小 fix, потім повний гейт і один commit.

Red flags (STOP → Phase 1): «проблема на рівні користувача», хаотичні патчі без тесту, «працює на одному пірі».

## 3. Розбір останнього багу (еталон застосування канону)

**Симптом:** `weights_matvec` Q4_K на 1152×588 tensor, шарди rows 288..576 / 864..1152 —
`max_err_model 4.0757e9 / 5.6174e9` на всіх трьох compute-шляхах пірів; `#43` на інших
shard-ах проходив. Гіпотези (застаріла сторінка, WebGPU, WASM SIMD, JS dequant, зіпсований
IndexedDB cache) були **спростовані** одна за одною — помилка була біт-ідентична.

**Root cause:** `tensor_raw_range()` рахував `lo = value_start * 144 / 256` — байтовий
зсув **всередині** Q4_K блоку, коли `value_start` не кратний 256 (288×588 = 169344 ⇒
95256 замість block-aligned 95328). `dequant_q4_k_matvec` і `tensor_f32_range` вважають,
що зріз починається з цілого блоку, тому хаб-референс для цих рядків був хибним.

**Fix:** `lo = (value_start / 256) * 144`. TDD-тест
`q4k_segment_588_cols_matches_full_reference` падав (`seg=0.043` vs `full=-60.625`)
до фіксу й проходить після. Тикет `t-1795680000000000000` → done.

## 4. Відкриті питання (не виправлялись без go owner'а)

- **PH-1204** — навчене вирівнювання 4 × 1024 vision-токенів до embedding-простору MiniCPM 1B; повна глибина Q4_K сама не дає image-grounding.
- **G13 / G19** — runtime/quality gate (blocked) і Astra-style patch (open, без безпечного кандидата).
- **Розсилка планів** — historical `- [ ]` у `2026-08-17-always-on-galaxy.md`, `2026-08-18-mcp-always-on-catchup.md` (tool_count 31 → 61), `2026-11-08-poolai-wsp-feasibility.md`; фактично виконані, чекають закриття окремими docs-тікетами.

## 5. Дисципліна

Один commit на репо + push + самарі; не `git add -A`; MSYS2 bash; Rust-first; ваги/секрети
не в git; перед комітом `git diff --check`, fmt, clippy, test, fingerprint з `Gsv-*` trailers.
