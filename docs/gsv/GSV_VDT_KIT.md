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
The rust folder that holds this crate is the **host**; every other tree is a **portable plugin**.
Environment security first. Maximum Rust. Session law: `agi`. Canon: [`GSV_AGI_PATH.md`](./GSV_AGI_PATH.md).

Це узгоджено з боксами GSV (Toolchain, IDE, SLI, Tracker): сервер уже є мета-шаром над
плагінами; тепер і **агентський кіт** має жити тут.

## Decision

1. **Git-канон спільного шару** — лише `S:\rust\GSV`:
   - `.agents/skills/` (Abracadabra + generic marketplace: architecture, TDD, debug, …)
   - `.cursor/rules/` — generic VDT (S0 диск, MSYS2, git, ролі, rust style)
   - дзеркала `.cursor/skills/` і `.opencode/skills/` (Windows: **copy**, не symlink)
2. **Cursor відкривати на GSV.** `абракадабра` / `abrakadabra` / `agi` **спочатку** сканує environment
   (`cargo xtask products`) і питає з **яким із видимих проєктів** працювати.
   `PRODUCTS.md` збагачує зареєстровані (HANDOFF / тести / ratio), не є єдиним списком опцій.
   Робота в дереві того репо (`S:\rust\poolAI`, `S:\rust\GSV`, далі — нові).
3. **Продуктовий шар лишається в продукті** (не переїжджає в GSV):
   - PoolAI: FM, concept, `test-ci`, Playwright admin, OpenAPI gap, Galaxy, 90–95% ratio
   - GSV-продукт: `gsv-server`, бокси, 95–100% ratio, порт 9999
4. Новий **плагін** на цій машині: git-репо-сусід під rust-фолдером `S:/rust/<name>` (discover підхопить).
   Рядок у `PRODUCTS.md` — лише якщо потрібен зареєстрований drain (Rust: HANDOFF + PH-S*; node: AGENTS + `npm test`, без PH-S*). Спільний кіт не копіювати. Безпека середовища (MCP sandbox = цей crate) не розширюється деревом плагіна.
5. **Проводка кіта по вікнах (kit-wiring band 2026-10-15).** `agi` має вести однаково скрізь:
   - **Per-window:** кожен зареєстрований плагін тримає кореневий `opencode.json` — `instructions`
     = GSV `AGENTS.md` → локальний `AGENTS.md` → NEXT/HANDOFF (global-first порядок), `skills.paths`
     = `S:/rust/GSV/.agents/skills`, `mcp.gsv_mcp_openbot` = stdio `target/live/gsv-mcp.exe --repo-root S:/rust/GSV`.
   - **User-scope fallback:** глобальний `~/.config/opencode/opencode.jsonc` несе той самий кіт
     (instructions + skills.paths + MCP stdio) — тож `agi`/Step 0r спрацьовують і в папці **без**
     власного `opencode.json` (навіть поза `S:/rust`). Folder-config має пріоритет (deep-merge),
     тому per-plugin pick не ламається. Локальний provider `gsv` перелічує `lama-2.8`/`lama-1.5`/`rusty-05`.
   - **Крок після правки конфіга:** config читається один раз на старт — перезапустити opencode.
     Правило «folder MCP only» стосується Cursor User-scope; OpenCode user-config — легітимний шлях для kit-less вікон.

## Split (що куди)

| Шар | Де | Приклади |
|-----|----|----------|
| **Shared VDT kit** | `S:\rust\GSV` | `abracadabra`, S0/`df`, MSYS2 bash, Conventional Commits, no `git add -A`, no Python product files, generic rust (`?` / no `unwrap` in product), architecture/TDD/debug skills |
| **Product canon** | репо продукту | PoolAI FM §5.12 / `NEXT_SESSION_PROMPT.md` / `runtime-stack-policy`; GSV `GSV_ROLES.md` / `GSV_TECH_ROADMAP.md` |
| **User-global** | хук на цій машині (owner 2026-10-21) | `%USERPROFILE%\.cursor\hooks.json` → `gsv-cursor-rules` чіпляє живі `AGENTS.md` + `.cursor/rules/` хаба лише до вікон, чий корінь лежить у rust-середовищі `S:/rust`. Папка поза ним — без хаб-правил. Вікно самого GSV хук не дублює. Не User MCP. Не копія кіта в плагін. Refresh після правок правил: `cargo build --release --bin gsv-cursor-rules` → копія `target\release\gsv-cursor-rules.exe` у `%USERPROFILE%\.cursor\hooks\` (новий чат підхоплює; cloud agents хук не виконують). |

## Cursor constraint (обов’язково)

| Відкрита папка | Що бачить агент |
|----------------|-----------------|
| папка під `S:\rust` (poolAI, rusty-engine, …) | локальні правила папки **плюс** хаб-глобали з user hook `sessionStart` |
| папка поза `S:\rust` | лише правила тієї папки; хаб не чіпляється |
| `S:\rust\GSV` | правила/скіли GSV (kit + продукт); хук мовчить, щоб не дублювати |
| multi-root `*.code-workspace` (GSV перший + продукти) | кіт з GSV + файли продуктів у сайдбарі; хук мовчить |

Наступна сесія: **File → Open Folder → `S:\rust\GSV`**, потім у чат: `абракадабра`, `abrakadabra`, або `agi`.

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
workspace folders, і git-сусідів під `S:/rust`. **GSV перший** у `gsv.code-workspace`,
щоб folder MCP хаба був завжди. Плагін відкритий наодинці в Cursor = без хаб MCP
(не ставти User-scope MCP).

**Global first / local product only.** Плагін тримає тонкий `AGENTS.md`
([`PLUGIN_AGENTS.md`](./PLUGIN_AGENTS.md)). PoolAI = генетичний батько multi-agent
DNA, коли це домінує по релевантності.

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

## Multi-window workflow (Kit-Eco 6, owner 2026-10-15)

2+ вікна (OpenCode / Cursor / Grok) на одному jail — один цикл на вікно, одна дошка:

1. **presence** — `gsv_tickets_presence` на старті сесії (`agi` Step 0e); хартбіт продовжує
   **лише власні** in_progress leases; `squad_cap` = member_count — зайвих не пускать.
2. **pick / 0r** — Step 0: скан + вибір продукту; папка поза списком → реєстраційне питання 0r.
3. **claim** — `gsv_tickets_next` → `gsv_tickets_claim`; чужий `claimed_by` **не чіпати**,
   stale повертає в open **тільки** `gsv_tickets_reclaim` (lease TTL дивитись у board-wire).
4. **drain** — працювати в **вибраному** дереві; паралельний cargo між різними `target/` —
   можна, два cargo на один `target/` — ні (file lock).
5. **done** — `gsv_tickets_done` з нотаткою (+1 rank claimerу; error −1); далі одразу
   `gsv_tickets_next` для наступного action цього вікна.
6. **sync** — telegram-relay on: Godfather dual-message (human line + JSON `{v:1,kind:sync}`);
   federation presence/claim/done/reclaim — бачити `GSV_SOLO_SQUAD_JAIL.md`.

Один close = один локальний коміт у дереві продукту (push — лише закон продукту: GSV origin,
rusty-engine B11 тощо). Між вікнами нічого не «домовляється» поза дошкою: tickets.jsonl —
єдиний координатор.

## See also

- Ролі сесії: [`../GSV_ROLES.md`](../GSV_ROLES.md)
- Open ecosystem / super MCP hub / AGI path: [`GSV_AGI_PATH.md`](./GSV_AGI_PATH.md)
- Roadmap: [`GSV_TECH_ROADMAP.md`](./GSV_TECH_ROADMAP.md)
- PoolAI (продукт): `S:/rust/poolAI/docs/development/NEXT_SESSION_PROMPT.md`
