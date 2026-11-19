# Екосистема CI + реєстрація — схема згідно GSV environment security (2026-11-18)

**Owner ask (2026-11-18):** «CI напевно взагалі треба буде переналаштувати згідно станів проектів та екосистеми.
я би хотів щоб вся схема екосистеми була налаштована з проектами та привелешіями/регістрацією, то що згідно
GSV environment security: які проекти open які closed, які є плагіни, з доповненим ресерчем як правильно».

Цей документ — **канон CI для екосистеми**: що реєструється, які класи, які правила GitHub Actions,
що зроблено/що лишилось. Право GSV-матриця дозволів — **environment security first**
(`AGENTS.md § Formats + local rules`, `GSV_AGI_PATH.md`, `PRODUCTS.md`).

---

## 1. Інвентар (зведено з `cargo xtask products --state` + `--wiring`, 2026-11-18)

Легенда: **host** — GSV-kit; **plugin** — портативний плагін (власний репо/target/rules, власні CI);
**node** — не Rust; **agent** — Pi mesh-нода; **no-remote** — remote ще не створений owner'ом.

| id | Репо | Клас | GitHub remote | CI зараз | Приватність | Реєстрація |
|---|---|---|---|---|---|---|
| **gsv** | `S:/rust/GSV` | **host** | ✅ origin/main | — (локальні гейти) | public | host-евалют. |
| **poolai** | `S:/rust/poolAI` | plugin (RUST) | ✅ origin/main | `ci.yml` (14 jobs ubuntu+windows) | public | PRODUCTS.md |
| **telenetis** | `S:/rust/telenetis` | plugin (RUST) | ✅ origin/main | — | public (MIT?) | PRODUCTS.md |
| **llama-rs** | `S:/rust/llama-rs` | plugin (RUST) | ✅ origin/master | `ci.yml` (1 job win, GNU) | public (MIT?) | PRODUCTS.md |
| **rusty-engine** | `S:/rust/rusty-engine` | plugin (RUST) | ✅ origin/main | — | public (MIT) | PRODUCTS.md |
| **rebook** | `S:/rust/rebook` | plugin (RUST) | ✅ origin/master | — | — private — | PRODUCTS.md |
| **linfs** | `S:/rust/LinFS` | plugin (RUST) | ✅ origin/main | — | — private — | PRODUCTS.md |
| **orr_desktop** | `S:/rust/ORR_DESKTOP` | plugin (RUST) | ✅ origin/master | `ci.yml` (1 job win) | — private — | PRODUCTS.md |
| **sport-forcast** | `S:/rust/sport-forcast` | plugin (RUST) | ✅ origin/main | — | — private — | PRODUCTS.md |
| **coreboard_labs_shop** | `S:/rust/CoreBoard_Labs_shop` | plugin (RUST) | ✅ origin/main | — | — private — | PRODUCTS.md |
| **gsv-wgpu** | `S:/rust/GSV-WGPU` | plugin (RUST) | ❌ **немає** | — | public (MIT) | PRODUCTS.md |
| **mesh-node** | `S:/rust/mesh-node` | **agent** (Pi) | ❌ **немає** | — | public (—) | PRODUCTS.md |
| **omniroute** | `S:/rust/omniroute` | **node** (bun/npm) | ✅ release/v3.8.50 | — | — private — | PRODUCTS.md |
| **freebeat.ai** | `S:/rust/freebeat.ai` | node (sibling) | ✅ origin/main | — | — ? — | **НЕ зареєстровано** |
| **platinoff-profile** | `S:/rust/platinoff-profile` | node (sibling) | ✅ origin/main | — | — ? — | **НЕ зареєстровано** |

**Стан регістрації ≠ стан GitHub-наявності.** `wiring ok` справний у всіх зареєстрованих плагінів
(github remote ↔ registry row) — це вже дозволяє CI брати `PRODUCTS.md` як джерело правди.

---

## 2. GSV environment security — що дозволяє CI

Канон (`GSV_AGI_PATH.md` + `AGENTS.md § Formats + local rules` + `PRODUCTS.md`):

1. **Global scope = лише GSV kit**: MCP-тули, термінал, vision/docs. Плагін не відкриває плагін,
   не копіює `.agents/skills/` повз `cargo xtask mirrors`, не змінює глобальні правила.
2. **Плагін ⊆ власного репо**: власний `target/`, власні rules, власні HANDOFF. CI має право
   запускати **лише те, що плагін власноруч оголошує** (тест-команди зі свого PRODUCTS.md).
3. **Folder-MCP only; no User-scope MCP**; no tunnels/`:8091` off-box; `:9999` LAN-accepted.
   CI secrets: **жодних** окрім `GITHUB_TOKEN` (contents:read за замовчуванням).
4. **Unplug = fail-open**: плагін говорить до хаба (`/mcp`, `/api/edge`); хаб недоступний ⇒
   плагін деградує gracefully, CI не фейлить увесь репо (тільки advisory уронить).
5. **Cross-repo writes**: лише `cargo xtask` у GSV; жодних write із плагіна в GSV
   (документація через reference, не copy). CI плагіна не має власного доступу до GSV.
6. **Private vs public визначається в PRODUCTS.md** ("proprietary, private repo"):
   - public (rusty-engine MIT, GSV-WGPU MIT, mesh-node Pi) — нормально мати CI
   - private (CoreBoard, sport-forcast, LinFS, ORR) — CI продуктивний (не публічні артефакти)
   - omniroute — node, `npm test` відмінний сценарій, той самий порядок
   - **незареєстровані** (freebeat.ai, platinoff-profile) — CI **не створюємо** (власник не дав PI
     drain); те, що public, не привід для автоматичного CI без рядка в реєстрі.

---

## 3. Схема — що з чим поєднано (verified 2026-11-18)

```
S:/rust/GSV (host kit; folder-MCP; gsv_env власний; no CI — живий хаб)
  └─ hub: :9999 · watchdog · vision · tickets · ranks · MCP sandbox
       ├─ /mcp (folder MCP для poolAI/telenetis/llama-rs/…) — тільки read tools
       ├─ /api/edge (read-only proxy) — ніколи не пише в плагін
       └─ keep-live probes (heartbeat, fail-open)

Plugins (кожен = окремий GitHub репо):
  poolAI ── GitHub CI (stable, ubuntu+win) ── FM §5.12 → PLAN → CI required (test-ci)
  llama-rs ─ GitHub CI (win GNU; LLVM/bindgen) ── solo selftest (Pi4 agent debug окремо)
  ORR_DESKTOP ─ GitHub CI (win GNU) ── clippy+test
  telenetis ── краще: CI відсутня зараз; якщо буде — fmt+clippy+test лише
  rusty-engine / rebook / linfs / sport-forcast / CoreBoard ── CI відсутня зараз
  GSV-WGPU / mesh-node ── CI заблокується СТВОРЕННЯМ REMOTE owner'ом
```

**Реєстраційна лінійка (owner decision, дотримано):**
- `freebeat.ai`, `platinoff-profile` — **без CI** (не зареєстровані; «один проект один кіт» правило).
- Комбінований колонка `audit` у PRODUCTS.md — не редагуємо (канон-файл owner'а).

---

## 4. Схема по класах + що робити CI

| Клас | Приклади | CI — minimum-required | CI — advisory-only | secrets |
|---|---|---|---|---|
| Host (GSV) | GSV | — | — | none |
| Rust-plugin, remote ✅, gates known | poolAI, llama-rs, ORR | fmt + check/clippy + ratio | full test (job-matrix), audit | GITHUB_TOKEN (contents:read) |
| Rust-plugin, remote ✅, gates unknown | telenetis, rusty-engine, rebook, LinFS, sport-forcast, CoreBoard | fmt + check | — | same |
| Node-plugin (npm) | omniroute | — | `npm ci && npm test` (якщо реєструють) | same |
| Rust-plugin, no remote yet | GSV-WGPU, mesh-node | **не створювати** до owner-gated remotes | — | — |
| Unregistered node siblings | freebeat.ai, platinoff-profile | **не створювати** | — | — |

Параметри, однакові для всіх workflows (poolAI вже має; інші запрошено):

```yaml
concurrency:
  group: ci-${{ github.ref }}
  cancel-in-progress: true
permissions:
  contents: read
  security-events: none
```

**Платформа тесту + `stable` = 1.100.0** (як у crate stable; перевірено — усі 11 `cargo check` зелені).
Windows jobs використовують `stable-x86_64-pc-windows-gnu` (елементи MinGW на GitHub runners є).

---

## 5. Що зроблено 2026-11-18 (обов'язковий scope "лише poolAI")

Файл: `/s/rust/poolAI/.github/workflows/ci.yml`

1. Тригери: `stage3/*` прибрано → **main + PRs до main**; додано `concurrency.cancel-in-progress: true`.
2. Кеш-ключі: від **PH-S47** (параметр 2026-03) → `ci-${{ runner.os }}-${{ hashFiles('**/Cargo.lock') }}`
   з restore-keys префіксом (per-OS keying, без глобальної «lost-update» лінії).
3. Прибрано `--verbose` у тестів/білдів (на GitHub annotation-шму ↑); поведінка деплой-матриці та сама.
4. `RUSTSEC` `audit-check` **залишено як є** (owner не підтримав жодну з опцій), задокументовано
   як головне джерело анотation (див. §6).
5. `actions/cache@v5`, `checkout@v6`, `setup-node@v4` тощо — сучасні (було OK); не чіпали.
6. `if-no-files-found: warn` для rust-diagnostics artifact — лишили (потрібне для кор.panel).
7. Повний `cargo check --features cloud,cloud-sdk` на poolAI **зелений (10 хв 42 s)** — 1.100.0
   сумісний; k8s-відкриття не блокер.

**llama-rs та ORR_DESKTOP — не чіпали** (owner: «Лише poolAI»). Перевірено локально, що їхні
CI вже self-consistent: llama-rs (`windows-latest`, dtolnay `stable-x86_64-pc-windows-gnu` explicitly — no host/pin mismatch), ORR_DESKTOP (`windows-latest`, `rustup default stable-x86_64-pc-windows-gnu` — no mismatch). Linux-target роботу не ведуть — збігу нема взагалі.
**GSV CI — відсутній** (локальний dev-box; хаб — жива інфра, не PR-полігон).

### 5.1 Run 1704 (`a6d7e8fbc`) — виміряний outcome

| Job | Result |
|---|---|
| Test Suite (ubuntu-latest) | ❌ **failure** — "Run tests (default) - Required" (`cargo test -j 1 --lib --tests --features ml,enterprise,cloud,test-utils,job-store-sqlite,prometheus`), 24m35s; fmt/clippy/build усі green — **перший реальний прогін тестів в історії цього репо**; failing test(невідомі без raw log; owner: "Skip for now") |
| Test Suite (windows-latest) | ⊘ cancelled — matrix `fail-fast: true` default; **виправлено** `6f0c92d85` (`fail-fast: false`) |
| OpenAPI gap audit | ✅ **green** — fix `5653fd4d6` (`/virtual-nodes/{peer_id}/tasks/result` + schemas) підтверджено |
| Vision manifest FM drift / LOC ratio / Rust diagnostics / Detect UI / Pa11y script contract / всі feature-checks | ✅ green |
| Pa11y WCAG 2.2 | ⊘ skipped — `Detect UI` paths filter: UI не змінилась ⇒ job не стартує (expected) |
| Playwright admin E2E | ❌ **failure** — "Build poolai and run smoke tests" після 344s; причина невідома (build/stand/visual-snapshot — без raw log не відрізнити); owner: "Skip for now" |
| Documentation | ⊘ run 1414 failed (pre-fix docs.yml, той самий toolchain-хост); fix `c5746bee1` (`RUSTUP_TOOLCHAIN: stable` у docs.yml) **не перевірено** — docs.yml тригериться лише на src/docs/Cargo.toml changes |

**Підсумок run 1704:** вся **CI-інфра** (toolchain pin, normalization, triggers, permissions, cache keys, `fail-fast: false`) зелена; залишаються два **реальні продуктові/тестові** червоні — обидва deferred owner'ом 2026-11-18.

---

**Test Suite (1705, заміряно нативно — обидва OS):**
- ubuntu: ❌ `Run tests (default) - Required` — **детермінований failing test** `horizon_s960_band_docs_legacy_close_ph_s969`
  (`tests/galaxy_horizon_s960_integration.rs:39`, `missing flat legacy doc docs/EXECUTE_NOW.md`)
- windows: ❌ **той самий тест** (1h04m, у логу видно той самий файл/рядок)
- Причина: `cargo test-ci` (ml,enterprise,cloud,test-utils,job-store-sqlite,prometheus) **найперший раз у історії репо виконав весь прогін**
  (імфра-піни виправлені) — і вперше виявив, що `FLAT_LEGACY_DOC_SAMPLES` вказує на файли, перенесені архів-смугою 2026-11 у `docs/archive/2026-11/`.
- **Fix `dafaff76e`** (sample list → існуючі `archive/2026-11/*.md` + `QUICK_START.md`; банери перевірено ×12).

**Pa11y + Playwright (1705):** build зелені (~2–6 хв), але `/api/v1/health` ніколи не повертає 200 за 90 спроб × 2s —
poolai **стартує, але або падає в init (мовчання RUST_LOG=warn), або висне**; причина невідома.
**Fix `4e682b111`** — діагностичний: обидва скрипти (bin/pa11y-ci.sh wait_health; poolai_e2e_stand.rs start/restart)
тепер друкують `tail -n 200 $STAND_ROOT/poolai.log` у лог кроку. Happy path не змінено.

**Run 1706 (`dafaff76e`, тестовий фікс)** — відомий кандидатом на green, але його скасує **1707 (`4e682b111`)** (cancel-in-progress за задумом).
**Run 1707 (`4e682b111`)** — **failure** (завершений 20:28Z):
- ✅ windows-latest Test Suite — **усі кроки зелені** (default tests: PASS; тестовий фікс підтверджено на Windows)
- ❌ ubuntu-latest Test Suite — fmt/clippy/builds/tests-default/tests-no-features ✅; job conclusion failure, але **steps «cloud-sdk feature»/«Security audit» у API-відповіді показані як in_progress/pending — логи 404 (job завершився без логу → імовірно runner-side kill/OOM)**; logs для цього job недоступні навіть з токеном
- ✅ OpenAPI · Check Features ×5 · FM drift · LOC ratio · Rust diagnostics · Detect UI · Pa11y contract — green
- ❌ Pa11y + Playwright — **root cause знайдено диким логом** (завдяки `4e682b111`): poolai падає в init з
  `ConfigError("Failed to create library directory. … Path: '/var/lib/poolai/libs', Error: Permission denied (os error 13)")`
  —_hardcoded Linux default; `POOLAI_DATA_PATH` (який скрипти ставлять у writable stand_root) не враховувався.
- **Fix `9f330d3aa`** — `LibraryManager::get_default_library_path()` тепер з `POOLAI_DATA_PATH` → `$POOLAI_DATA_PATH/libs`
  (production `/var/lib/poolai` семантика не змінилася; docs RUN_PARAMETERS.md оновлено). Очікуємо green Pa11y/Playwright у 1708.
- **Run 1708 (`9f330d3aa`) — фактичні результати** (перевірено jobs API):
  - ✅ Documentation run **1416** — green (той самий sha, docs.yml спрацював)
  - ✅ OpenAPI · Check Features ×5 · FM drift · LOC ratio · Rust diagnostics · Detect UI · Pa11y contract — green
  - ❌ **Test Suite (ubuntu) — знову обрив на кроці `Run tests (cloud-sdk feature)`** (default tests ✅ — `dafaff76e` підтверджено на ubuntu; logs 404 — runner-side kill, імовірно OOM на найважчій feature-збірці; той самий симптом, що й 1707)
  - ⏳ **Test Suite (windows) — усе ще in_progress на момент перевірки** (очікуваний ~50 хв прогін; на 1707 був повністю зелений)
  - ⊘ **Pa11y + Playwright — SKIPPED за paths-filter** (`ui`: src/ui/**, bin/pa11y-ci.sh, .github/workflows/{a11y,ci}.yml, tests/pa11y_ci_script.rs; `e2e`: e2e/**, bin/e2e-playwright.sh, ...). `9f330d3aa` (src/libs/manager.rs + docs/RUN_PARAMETERS.md) **не підпадає під жоден фільтр** → runtime-перевірка libs-фиксу в CI не відбулась. Фікс імплементаційно правильний (скрипти вже ставили POOLAI_DATA_PATH на writable stand_root), підтвердження — наступний push, що чіпає `bin/pa11y-ci.sh` / `.github/workflows/ci.yml` / UI, або ручний dispatch `a11y.yml` (owner decision).

- **Run 1710 (`a4f975825`) — результат:** pa11y — `/ui` **зелений** (фікс #home-power-reboot підтверджено), але `/ui/admin/monitoring` **вдруге червоний із тим самим 3.14:1**: у репо **друга копія** `showNotification()` у `src/ui/admin_common.js` з тими ж `#ff5555` + white. **Фікс `b870bae38`** → run **1711** (в дорозі на час запису).
- **Run 1709** — скасовано через `cancel-in-progress: true` при пуші 1710 (за задумом).

## 6. Джерела анотation (чому «багато анотацій»)

Порядок за імовірністю:
1. **`rustsec/audit-check@v2.0.0`** — кожна advisory = 1 annotation (з `continue-on-error: true`).
   У workspace із ~1000+ deps — десятки анотation на кожному ubuntu+windows прогоні.
   Owner-рішення pending (skill-флоу питання без відповіді); варіанти: (a) `cargo audit --json` + exit 0 у лог;
   (b) прибрати з ci.yml взагалі; (c) лишити як є.
   **Не змінював без рішення.**
2. **⚠ Windows fmt коштував тижні червоного — root cause знайдено 2026-11-18**: `rust-toolchain.toml`
   пінує `stable-x86_64-pc-windows-gnu`, але `dtolnay/rust-toolchain@stable` default installs host default
   (ubuntu: stable=linux-gnu — працює, як і треба; windows: stable-msvc — **НЕ stable-k8s-brew, тобто
   не той хост, ще й без x86_64-pc-windows-gnu**). `RUSTUP_TOOLCHAIN` тоді вказує на інший toolchain,
   який не встановлено → `cargo fmt/build/clippy` Exit 1 за мілісекунди.
   **Fix**: dtolnay ставить той самий toolchain (`${{ matrix.os == 'windows-latest' && 'stable-x86_64-pc-windows-gnu' || 'stable' }}`) — за пушено (`be76670a8`).
3. ~14 job-ів × ~1-2 анотації: `upload-artifact if-no-files-found: warn` (warning) — артефакт-ім'я враховує k8s,
   але у `cargo test` тепер `--verbose` прибрано, тож анотation-типів стало значно менше.
4. `dorny/paths-filter` — у нормі не анотation.
5. `CARGO_TERM_COLOR: always` + `--verbose` давав ↑ volume (сьогодні прибрано).
6. Ubuntu fmt was fine all along; Windows fmt always was the red (MinGW/MSVC mismatch).

---

## 6.1 Стан 2026-11-19 (run 1711 — a11y-фікс `#de3434` верифікація)

Run **1711** (`id 44436252477`, push `b870bae385ca5b61224a5cf60e820b257cf1a63a`, `2026-11-19T01:21→03:10Z`) —
висновок overall **failure**, але підсумки по кроках:

| Крок | Висновок |
|---|---|
| **Pa11y WCAG 2.2** (login + 21 auth URLs, `/ui/admin/monitoring`) | **✅ success** — фікс `#de3434` (4.59:1) верифіковано живцем |
| Documentation 1418 | ✅ success |
| OpenAPI gap audit · ratio hold · FM drift · vision manifest | ✅ success |
| Test Suite (ubuntu) | ✅ success (cloud-sdk tests skipped — owner: non-blocking) |
| Test Suite (windows) | ✅ success — cloud-sdk tests **running and passing** |
| Security audit | ✅ success |
| **Playwright admin E2E / Playwright admin suite** | ❌ **failure** — крок «Build poolai and run smoke tests» (~18 хв, падіння на 01:40Z) |

**Playwright-логи 403 для anonymous** (потрібен токен) — причина падіння не визначена.
Кандидат №1: знайомий клас «poolai не відповідає 200 за 180 s» (init/health), той самий корінь,
що фіксив `4e682b111` (лог-tail у `bin/pa11y-ci.sh` + `e2e_stand`). **Дія власника:** перевірити
крок у браузері (`https://github.com/platinoff/poolAI/actions/runs/44436252477/job/135265930583`)
або authorized re-run із лог-tail діагностикою. Local cargo test не запускався (RAM/EDR) — CI авторитет.

## 7. Що лишилось власницьки (не робити без інструкцій)

| # | Дія | Власник | Коментар |
|---|---|---|---|
| 1 | GitHub repo для `GSV-WGPU` | owner | `github.com/platinoff/GSV-WGPU.git`; після — CI у plugin (той самий шаблон) |
| 2 | GitHub repo для `mesh-node` | owner | `github.com/platinoff/mesh-node`; Pi-only agent; CI = fmt+check+selftest, не testy на PC |
| 3 | `freebeat.ai`, `platinoff-profile` — у PRODUCTS.md? | owner | зараз НЕ реєструємо; CI не створюємо |
| 4 | poolAI rustsec шлях (a / b / c) | owner | див. §6.1 |
| 5 | CI для інших плагінів (telenetis, rusty-engine, …) | owner | чекає власного drain-циклу; не автоматично |
| 6 | llama-rs / ORR_DESKTOP workflows v4→v6 | owner | зараз не в scope («Лише poolAI») |

**Правило без винятків:** жоден плагін не отримує CI без рядка в `PRODUCTS.md` (вимога
runner-узгодження з реєстром) і без `permissions: contents: read` + no extra secrets.

## 8. Схема потокологіки

```
owner → GSV (host kit)
         ├─ docs/PRODUCTS.md (регістр drain-кандидатів)
         ├─ cargo xtask products (discover+wiring)
         ├─ HANDOFF/NEXT (потоковий стан)
         ├─ $plugin (drain: HANDOFF → tests → FM/vision → один commit+push)
         └─ CI (GitHub Actions per плагін)
                ├─ stable = 1.100.0 (не наявний нижче 1.100)
                ├─ ubuntu-latest + windows-latest
                ├─ permissions: contents:read
                ├─ concurrency: cancel-in-progress: true
                └─ (gates plugin: fmt+clippy+check → ratio → test-ci)
```

**DoD для реєстрації нового плагіна:** рядок у PRODUCTS.md → `gsv.code-workspace` folder →
`cargo xtask products` бачить → вибір owner'ом → CI фітиль тільки після створення remote owner'ом.
