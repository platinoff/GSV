# GSV VDT kit — точка входу для будь-якого Rust-проєкту

**Status:** Accepted (owner 2026-08-17 · band 127 **PH-S1909…S1918**) · **Workspace:** **`S:\rust\GSV`** or `gsv.code-workspace`
**Deciders:** власник

## Context

Cursor / OpenCode підвантажують **правила і скіли з відкритого workspace**.
Git-канон спільного кіта — **цей репо** (`.cursor/rules/`, `.agents/skills/`, marketplace skills).
PoolAI тримає лише продуктовий шар + вказівник на kit.

Наслідок до band 127: відкрити `S:\rust\GSV` як папку = агент не бачив `абракадабра`.
Після drain: відкривати GSV (або `gsv.code-workspace`); PoolAI як єдиний корінь — fallback.

Власник: **GSV = точка входу** для розробки будь-якого Rust-проєкту (спочатку PoolAI + GSV).

Це узгоджено з боксами GSV (Toolchain, IDE, SLI, Tracker): сервер уже є мета-шаром над
продуктами; тепер і **агентський кіт** має жити тут.

## Decision

1. **Git-канон спільного шару** — лише `S:\rust\GSV`:
   - `.agents/skills/` (Abracadabra + generic marketplace: architecture, TDD, debug, …)
   - `.cursor/rules/` — generic VDT (S0 диск, MSYS2, git, ролі, rust style)
   - дзеркала `.cursor/skills/` і `.opencode/skills/` (Windows: **copy**, не symlink)
2. **Cursor відкривати на GSV.** `абракадабра` / `abrakadabra` **спочатку** сканує environment
   (`scripts/list-vdt-products.sh`) і питає з **яким із видимих проєктів** працювати.
   `PRODUCTS.md` збагачує зареєстровані (HANDOFF / тести / ratio), не є єдиним списком опцій.
   Робота в дереві того репо (`S:\rust\poolAI`, `S:\rust\GSV`, далі — нові).
3. **Продуктовий шар лишається в продукті** (не переїжджає в GSV):
   - PoolAI: FM, concept, `test-ci`, Playwright admin, OpenAPI gap, Galaxy, 90–95% ratio
   - GSV-продукт: `gsv-server`, бокси, 95–100% ratio, порт 9999
4. Новий проєкт на цій машині: покласти git-репо під `S:/rust/<name>` (discover підхопить).
   Рядок у `PRODUCTS.md` — лише якщо потрібен зареєстрований drain (Rust: HANDOFF + PH-S*; node: AGENTS + `npm test`, без PH-S*). Спільний кіт не копіювати.

## Split (що куди)

| Шар | Де | Приклади |
|-----|----|----------|
| **Shared VDT kit** | `S:\rust\GSV` | `abracadabra`, S0/`df`, MSYS2 bash, Conventional Commits, no `git add -A`, no Python product files, generic rust (`?` / no `unwrap` in product), architecture/TDD/debug skills |
| **Product canon** | репо продукту | PoolAI FM §5.12 / `NEXT_SESSION_PROMPT.md` / `runtime-stack-policy`; GSV `GSV_ROLES.md` / `GSV_TECH_ROADMAP.md` |
| **User-global** | не канон | `~/.cursor/skills/` лише якщо OWNER явно поставить install; git-джерело завжди GSV |

## Cursor constraint (обов’язково)

| Відкрита папка | Що бачить агент |
|----------------|-----------------|
| `S:\rust\poolAI` | правила/скіли PoolAI (поточний стан) |
| `S:\rust\GSV` | правила/скіли GSV (kit + продукт) |
| multi-root `*.code-workspace` (GSV перший + продукти) | кіт з GSV + файли продуктів у сайдбарі |

Наступна сесія: **File → Open Folder → `S:\rust\GSV`**, потім у чат: `абракадабра` або `abrakadabra`.

Не відкривати PoolAI як єдиний корінь, якщо ціль — GSV-кіт: інакше drain знову піде в FM PoolAI.

## Options considered

### A. GSV = git-канон, copy в кожен продукт (відхилено як основна модель)

Працює на Windows, але роз’їжджається (три копії skills). Залишається лише як fallback
для репо, яке **ніколи** не відкривають через GSV.

### B. Install у `~/.cursor/skills` (опційно пізніше)

Працює в будь-якому вікні Cursor. Не git-канон; легко застаріти. Може бути PH-S* після
стабілізації kit.

### C. GSV workspace = точка входу (прийнято)

Один відкритий корінь = один кіт. `абракадабра` сканує environment і питає з яким
проєктом працювати. Multi-root workspace — у сайдбарі продукти; discover бачить і
workspace folders, і git-сусідів під `S:/rust`.

## Consequences

- Легше: новий Rust-репо підключається реєстром, без копіювання 20 skills.
- Важче: хто відкриває лише PoolAI — бачить product rules + thin pointer, не повний kit.
- PoolAI `.agents/skills/` — **вказівник** «kit = GSV» + `poolai-documentation`; marketplace canon тут.
- Нумерація PH-S* лишається спільною з PoolAI FM, доки OWNER не розділить журнали:
  хто дрениться першим (`gsv` vs `poolai`) — той бере наступні PH-S1909….

## Action items (band 127 — drained)

**PH-S1909…S1918** (після GSV band 126 / S1908).

| Sprint | Фокус | Acceptance |
|--------|-------|------------|
| **PH-S1909** | Canon | цей файл Accepted; рядок у `GSV_TECH_ROADMAP` + HANDOFF/NEXT |
| **PH-S1910** | Abracadabra host | `.agents/skills/abracadabra/` у GSV: крок 0 = який **продукт** (poolai \| gsv \| …) |
| **PH-S1911** | Generic skills | copy marketplace skills з PoolAI `.agents/skills/` (без `poolai-documentation`) |
| **PH-S1912** | Generic rules | `.cursor/rules/` VDT: session, roles, MSYS2, git, rust-generic, cursor baseline |
| **PH-S1913** | Client mirrors | `.cursor/skills/` + `.opencode/skills/` identical copies |
| **PH-S1914** | Product registry | `docs/gsv/PRODUCTS.md` (root, handoff, test cmd, ratio) |
| **PH-S1915** | Workspace | `gsv.code-workspace` (GSV + PoolAI folders) |
| **PH-S1916** | PoolAI thin | PoolAI skills/rules: product-only; pointer «kit = GSV» |
| **PH-S1917** | AGENTS / roles | `AGENTS.md` + `GSV_ROLES.md` описують entry-point, не лише gsv-server |
| **PH-S1918** | Band close | `cargo test` + loc-audit ≥96% · vision-sync · один commit + push |

**Не переносити в shared kit:** PoolAI FM/DIGEST/concept, `poolai-documentation` skill,
`poolai-testing-policy` (test-ci / Playwright admin), Galaxy/OpenAPI bins.

## See also

- Ролі сесії: [`../GSV_ROLES.md`](../GSV_ROLES.md)
- Roadmap: [`GSV_TECH_ROADMAP.md`](./GSV_TECH_ROADMAP.md)
- PoolAI (продукт): `S:/rust/poolAI/docs/development/NEXT_SESSION_PROMPT.md`
