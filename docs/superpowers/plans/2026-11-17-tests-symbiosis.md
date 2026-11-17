# План — P6: тести у всіх плагінах під нову архітектуру (symbiosis)

**Дата:** 2026-11-17 · **Власник:** platinoff · **Стан:** draft для review · **Виконавці:** GSV team (agent per phase)

## Контекст і чому саме зараз

Екосистема перейшла на канон «тато і мама» (GSV = операційний план, poolAI = FM/FM-журнал),
але тестові набори залишились з попередньої архітектури — вони верифікують факти, які вже
не відповідають цільовому стану, або дублюють перевірки. Це і є те «не потрібне», що займає
місце (build artifacts до 36 GB) і час.

**Правила:**
- Ніколи не послаблювати ratio (≥ 95% formal, ≥ 96% stretch) без власника.
- Кожен знести-або-лишити виклик обґрунтовується архітектурним інваріантом (що кожен тест має перевіряти).
- Дубліс провід/повтор — один висновок, а не два тести на те саме.
- Нікуди не чіпати до зеленого базового прогону (`cargo test` — \[2026-11-17: GSV 1061 exit 0, 498 lib\]).

## Фази

### P6.0 — Інвентар (GSV, poolAI, llama-rs, rusty-engine, telenetis, LinFS, ORR, sport-forcast, coreboard)
Для кожного репо — таблиця: `tests/*.rs` · `src/**/test_*` · `#[cfg(test)] mod` · кількість, покриття, власні провайдери/харнеси, что саме вони перевіряють проти canon (AGENTS.md / AGI_PATH / symbiosis).
Deliverable: `GSV_TEST_INVENTORY_2026-11.md` (markdown, у GSV `docs/vision/` або superpowers/plans).

### P6.1 — GSV (host): нормалізація
Групування тестів за контрактом: **hub/server** (:9999) · **mesh** (`/api/wgpu/*`) · **MCP** (openbot, 60/60) · **ui cards** · **vision/xp** · **xtask**. В рамках групи — прибрати дублюючі фікстури, покласти contract-style на hub-провайдер замість кількох мок-провайдерів, упевнитись, що `--check` (vision/ratio/fingerprint gate) — зелений.
Результат: менше, але сильніших тестів; ratio лишається ≥ 96%.

### P6.2 — poolAI: переписати під нові ролі
Поточні тести претендують на «все окремо»; фіксують поверхневі ознаки: кожна підсистема з FM-каталогом покривається тестами, FM ↔ code-діапазон збігаються. Стартувати від FM §5.12 (`FUNCTION_MANAGEMENT.md`) + symbiosis (`tickets.jsonl`, `products --state/--wiring`). **Працює лише у poolAI CI / окремому вузлі**, не локально (RAM). Зробити по модулях: `orchestrator/` → `raid/` → `discovery/` → `ui-admin/`. Кожен блок — окремий коміт у poolAI з FM-рядком.

### P6.3 — Прикладні плагіни (engines + commutation)
llama-rs, rusty-engine, telenetis, LinFS, ORR, sport-forcast, CoreBoard: ті ж канони — тільки тести проти власних provider/wire, без «GSV»-чеків у кодовій базі плагіна. У кожного свій fix-band.

### P6.4 — Знесення мертвих тестів
Тести, які перевіряють фічі, що **вилучені** (ніколи більше не підімкнуто): duplicate провайдери/харнеси, fixed-port assertions (`:8765`, `:8091`), User-scope MCP, pre-symbiosis FM/tickets розділення. Виняток: тести-індикатори регресій, що підтверджують «ще відомо» — лишаються. Ті самі видалення корелювати з ревізіями vision/FM.

## Ворота (кожна фаза)
- `products --state` зелений · `products --wiring` ok · `stand-smoke` 87/87
- ratio ≥ 96% stretch
- `cargo test` exit 0 у відповідному репо
- без копіювання чужого архітектурного коду в GSV (плагіни лишають своє своїм)

## Ризики
- **RAM:** повні CI-прогони poolAI не локальні — виносити на CI/окремий вузол.
- **Ratio:** видалення тестів vs ratio gate — контроль §6.1: спочатку видалити провайдери, не тести, рахувати «чистіше але зелено».
- **FM/vision drift:** кожна переробка FM-рядами; `vision-sync --check` обов'язково перед push.
- **Зона видимості:** не писати CI poolAI на GSV; не тягнути poolAI тести в GSV.

## Тікети
- P6.0 inventory · P6.1 GSV normalize · P6.2 poolAI rewrite · P6.3 plugins · P6.4 dead-test removal · P6.5 final gate (ratio + stand-smoke + exit 0)
