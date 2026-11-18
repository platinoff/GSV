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

**llama-rs та ORR_DESKTOP — не чіпали** (owner: «Лише poolAI»).
**GSV CI — відсутній** (локальний dev-box; хаб — жива інфра, не PR-полігон).

---

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
