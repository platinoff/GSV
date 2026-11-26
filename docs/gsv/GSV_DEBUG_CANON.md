# GSV_DEBUG_CANON — канон дебагінгу (2026-11-26)

**Статус:** канон-план, сумісний зі сталевими станами проєктів. Джерело методу:
`.agents/skills/systematic-debugging/SKILL.md` + `debug/SKILL.md` (дзеркала в
`.cursor/skills/` і `.opencode/skills/` через `cargo xtask mirrors`).

## 1. Сталеві стани (звірено 2026-11-26)

| Що | Значення |
|---|---|
| GSV | HEAD `4396bd6` (WASM-1 native Q4_K dequant), `origin/main` синхронний; робоче дерево — незакомічений BENCH/STREAM slice (див. §7) |
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

## 5. WASM-прискорення (канон 2026-11-26)

- **Пріоритет:** важкі числові шляхи → `crates/mesh-wasm` (Rust → wasm32-SIMD), JS — лише glue/fallback.
- Наявні ядра: `mesh_matmul64`, `mesh_matvec[_partial]`, `mesh_softmax`, `mesh_layernorm`, `mesh_mlpblock`, нове **`mesh_dequant_q4_k`** (144 B → 256 f32 на блок).
- Artifact-backed `weights_matvec` у `/wgpu` тепер викликає `mesh_dequant_q4_k` нативним кодом; JS `dequantQ4K` лишається fallback-ом. Phone-піри тимчасово йдуть через golden-tested JS-маtvек (див. G47/G48) і підхоплять новий `.wasm` після reload сторінки.
- Build/disploy: `cargo build --release -p mesh-wasm --target wasm32-unknown-unknown` → `cp target/wasm32-unknown-unknown/release/mesh_wasm.wasm ui/assets/mesh-wasm.wasm`; далі `cargo build --bin gsv-server` + `POST /api/update/apply`.
- Верифікація: `cargo test -p mesh-wasm` (native, scalar kernel), контракт сторінки (`mesh_dequant_q4_k` marker), живий canary `weights_matvec` (`g-7353` PASS `1.39e-17`) і `/assets/mesh-wasm.wasm` розмір/наявність.

## 6. Huge-expert Q4_K — другий root cause (2026-11-26)

Канар `g-7360` на `blk.0.ffn_gate_exps.weight` (2048×131072, 8 shard) дав
`max_err_model ≈ 622–796` на 8/8 — **не** зсув блоку (як у §3), а субнормальний
f16 у WASM `mesh_dequant_q4_k`: починав `e = exp = 0` замість канонічного
`e = 113` (bias-1) з GSV `wgpu_dequant::f16_to_f32`, через що `d`/`min`
масштаби роздувались ~2^14. Виправлено, додано golden-тест проти GSV
`dequant_q4_k` (subnormal `d=2^-24`, `min=0.5`, scales `0x21`, nibble 1..16 →
`-16.499998…`), `.wasm` перезібрано (27 462 B). **Не workaround, а parity-фікс.**

## 7. Streaming MoE (канон 2026-11-26)

- Повний 21 ГБ 4-квант **не вантажиться цілком** у 7.7 ГБ RAM (реально ~1.7 ГБ
  вільно); `weight_store::read_chunk_artifact` уже читає range з дзеркала з
  sha256 — як і передбачав owner, «підкачується не все».
- Головний блокер — **не RAM і не bandwidth, а відсутній expert-aware forward
  orchestrator** (router → top-8 → per-expert range → shard dispatch → combine),
  плюс тимчасовий числовий баг із §6 (виправлено, чекає reload пірів).
- План/оцінки: [`research/GASV_STREAMING_MOE_PLAN_2026-11-26.md`](research/GASV_STREAMING_MOE_PLAN_2026-11-26.md); огляд двигунів + скетч `moe_forward.rs`: [`research/GASV_ENGINE_LANDSCAPE_MOE_ADAPTATION_2026-11-26.md`](research/GASV_ENGINE_LANDSCAPE_MOE_ADAPTATION_2026-11-26.md); data-plane/SSD-mirror + WiFi/WGPU hardware + autonomy ladder research: [`research/GSV_STREAMING_OFFLOAD_RESEARCH_2026-11-26.md`](research/GSV_STREAMING_OFFLOAD_RESEARCH_2026-11-26.md).
- Харнес: `cargo xtask moe-bench` (BENCH-1, unit green) → `data/moe_bench.jsonl`;
  BENCH-2 (sweep 2/4/8 × gate/up/down) і BENCH-3 (чесний smoke: partial pack
  `g2b-q4_0` не запускний — `missing required key 'gsv.attention.head_count'`).
- Реалістична межа ~<1 tok/s на 2 пірах; жодного quality/full-LLM-claim без G13.

## 8. Дисципліна

Один commit на репо + push + самарі; не `git add -A`; MSYS2 bash; Rust-first; ваги/секрети
не в git; перед комітом `git diff --check`, fmt, clippy, test, fingerprint з `Gsv-*` trailers.
