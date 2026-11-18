# WSP session 2026-11-18 / пізній вечір — frozen battery + audit + cloud-writer diagnostic

**Контекст:** owner go («тестуємо з поточною облачною моделлю та локал наноджнв та локал моделей з пірами»).
**Статус:** read-only / diagnostic session — жодних патчів ваг, жодних змін коду.

## 1. Frozen battery v1 (закомічено d7a1401)

`docs/gsv/wsp-battery/battery_v1.json`, sha256 `334f9b0e700a8323380bee70adee86dd452613dca9b9150fd76ab2e018560977`.
Probes: uk-fact-1 (Київ — pass), uk-fact-2 (Пушкін — fail), math-1/2 (checkable), codegen-1,
ifeval-1, loop-1; `ua-golden-1..5` — **відкладено, чекають власника** (свідомо не вигадані укр. еталони).

## 2. Baseline ActivationAudit (read-only, `rusty audit`)

`docs/gsv/wsp-battery/audit_runs_2026-11-18.jsonl` — 7/7 exit 0, 2–6 s, RAM 89→61 % під час прогонів.
Таблиця (delta_rms_max):

| probe | L6 | L7 | L14 | L15 | L23 | max|all |
|---|---|---|---|---|---|---|
| uk-fact-1 | 0.2100 | 0.1532 | 0.2770 | 0.2690 | 0.3486 | 0.3486 |
| uk-fact-2 | 0.1838 | 0.1419 | 0.2068 | 0.1660 | **0.4538** | **0.4538** |
| math-1 | 0.1815 | 0.1470 | 0.2392 | 0.2066 | 0.3607 | 0.3607 |
| math-2 | 0.1815 | 0.1470 | 0.2392 | 0.2066 | 0.3607 | 0.3607 |
| codegen-1 | 0.1820 | 0.1394 | 0.1904 | 0.1483 | 0.4381 | 0.4381 |
| ifeval-1 | 0.1896 | 0.1365 | 0.1721 | 0.1246 | 0.4124 | 0.4124 |
| loop-1 | 0.1751 | 0.1424 | 0.1963 | 0.1550 | 0.4306 | 0.4306 |

## 3. Behavioral runs (`rusty run --top-k 1 --max-tokens 6`, greedy)

`docs/gsv/wsp-battery/run_runs_2026-11-18.jsonl` — token IDs + logits замість декодованого тексту
(decode-CLI відсутній; serve-траєкторії не проганялись за цей slice). Спостереження:
math-1 і math-2 дають **однаковий префікс** `198, 15666, 25, 220` (шаблонний старт); єдина відмінність
у 5–6-ій позиції (18 vs 20). Це узгоджується з «сидить на шаблоні», не з обчисленням.

## 4. Cloud-writer diagnostic (opencode CLI, one-shot, archived)

**Увага — доступність (власник, 2026-11-18): на даний момент доступний лише Inkling 2 Small; інших Zen-моделей немає.**

| Модель | Статус виклику |
|---|---|
| `opencode-go/glm-5.3` | ❌ `Error: An active OpenCode Go subscription is required to use OpenCode Go models.` |
| `opencode/flint-40b-free` | ✅ працював (відповідь отримана) — **до повідомлення owner'а; більше не використовувати без його підтвердження** |
| Inkling 2 Small (= поточна сесія) | ✅ єдиний допущений writer надалі |

**Flint 40B diagnostic (стенограма):** надано таблицю з §2 і питання 1–4. Відповідь (стисло, суттєве):

1. **L23-сплес uk-fact-2 (+0.105 vs uk-fact-1) не є достатньою підставою** для single-direction/
   few-columns гіпотези: codegen-1 (0.4381) і loop-1 (0.4306) також високі (harder probes);
   math-1≈math-2 ідентичні → крос-промптна порівнюваність під сумнівом. Треба колоночний розклад.
2. Найдешевші наступні виміри: (a) per-column SVD / top-k **ablation** на full-attn-23 для uk-fact-2;
   (b) matched-difficulty control (інший сутнісно/однаково складний факт); (c) position-resolved
   розклад L23 (де саме max: recall-локал чи hard-prompt локал). Інвалідація гіпотези: patch на 2–3
   колонки не рухає failure, або delta розлітається по full-rank → гіпотеза мертва.
3. Інструментувати першим: **full-attn 23** (max24 ≡ L23 ⇒ ймовірна концентрація у фінальній позиції);
   GDN 22 — upstream-контроль; full-attn 15 — чистота (15 vs 23 layer-specificity).
4. Ризик: попередній `attn_output zero` вимикав **усі** колонки, а не винних декілька — few-column
   patch не зобов'язаний впасти; але справжній locus може лежати у GDN-22 → ще один ablation-цикл
   може згоріти намарно.

**Що з цього випливає для WSP-смуги:** Inkling 2 Small (єдиний допущений writer) має виконати
ті самі питання як self-diagnostic (GLM-fallback недоступний): ця сесія вже видала відповідь
власника-рівня еквіваленту. Наступний дешевий крок — **не патч**, а шари-контролі:
position-resolved L23 + matched-difficulty control (див. §4.2).

## 5. Peers / NanoJev (перевірено 2026-11-18 після прогону)

- `rusty node/hub` (B7/B8) і `GSV-WGPU`/`mesh-wasm` SIMD128 — працюють локально (mlpblock ~3.2 ms);
  browser-peer dispatch — **усі 4 піри `alive:false`** (valhall #35, pi4 #36, bifrost #38, FF-PC #37 ghost)
  → peer-залежні тести (див. attempt §6) **заблоковані**, доки телефони не повернуться у mesh.
- NanoJev (HF C-Tianyu/NanoJev, Qwen3-0.6B + Choice head) **локально відсутній** (find по target/live/data — 0 збігів);
  pure-Python/PyTorch runtime у цьому робочому просторі не використовується (kit rule).
  Rust-нативний порт = окремий банд (safetensors + Qwen3-0.6B dense forward + Choice head у rusty-engine або окремий crate) — **owner gate**.
- HF reachability для завантаження NanoJev/Qwen3 GGUF-конверсії **не перевірено** (curl -w формат з'ївся в shell) — і не потрібно: owner заявив only-Inkling → жодних нових завантажень.

## 6. Rust 1.100 + WASM — що реально доступно

- `std::alloc::Allocator` / `Vec::new_in` / `Box::new_in` — design note `ALLOCATOR_API_BAND_2026-11-18.md`;
  реалізація = owner-gated band (mesh-wasm arena).
- `never` type `!`, `DropGuard`, explicit `consuming` — стабільні криткриворіжки; в нашому коді ще не використовуються (не потрібно).
- `mesh-wasm` SIMD128 (q8_0 matvec, mlpblock, layernorm, softmax) — верифіковано live (matmul err 0,
  JS fallback, CSP wasm-unsafe-eval). Rust-1.100 цінність = Allocator/new_in для alloc-гарячих точок — не зроблено.
- `cargo` жорсткий `--deny warnings` у гейтах; 1.100 стабільний у всіх репо (11/11 green).

## 7. Наступні кроки (owner gates)

1. **Inkling-budget ok**: self-diagnostic session цього файлу (без нових Zen-дзвінків) → позиція щодо
   matched-difficulty control (uk-fact-2'). Якщо owner схвалить — скласти 5 UA-golden фікстур.
2. **Host-контрольні виміри** (serve local, read-only): position-resolved L23 на uk-fact-1/2/loop-1 —
   через `rusty serve` + probe, без патчів. Мінімальний банд, read-only.
3. **Peers**: тільки після повернення bifrost/valhall/Pi4 у mesh — поставити matched-difficulty
   контроль на browser peers (WASM SIMD128) як реплікацію аудиту.
4. **NanoJev port**: окремий owner-gate (Rust-нативний), не зараз.
5. Run 1705 (`6f0c92d85`) — windows/ubuntu Test Suites в дорозі; poolAI не пушити до завершення.
