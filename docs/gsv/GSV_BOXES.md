# GSV Boxes — панелі/можливості «Galaxy StarWalker Vision»

Специфікація боксів сервера GSV. Кожен бокс — панель UI + Rust-модуль.

## 1. Tracker (технічні параметри workflow)

**Роль:** показує технічні параметри виконаного воркфлоу (що реально виконувалось).

Дані: спринти (PH-S*), команди, часові мітки, статуси, кількість файлів/LOC, wall-clock.

| Поле | Джерело |
|------|---------|
| Sprint id / band | FM §5.12 |
| Виконані команди | shell history / logs |
| Тривалість кроків | timestamps |
| LOC / files | `gsv-loc-audit` |
| Статус / ✅ | FM §5.12 |

Rust модуль: `tracker/` → `gsv_tracker.json`.

## 2. SLI console (команди + SLI-функції)

**Роль:** бачити, які команди використовуються, та **всі SLI-функції, які можна створити з наявних скриптів** (+ нові).

- Парсинг `src/bin/` + `cargo xtask` → каталог SLI-команд (назва, опис, входи). No product `.sh`.
- Виводить фактично використані команди (з Tracker/history).
- Пропонує **незадіяні скрипти** → потенційні нові SLI-функції.
- Відкритий реєстр для нових функцій.

Rust модуль: `sli/` → `gsv_sli.json`.

## 3. Toolchain (які тули використовуються)

**Роль:** інвентар тулів проєкту.

| Тул | Версія | Джерело |
|-----|--------|---------|
| rustc / cargo | 1.92.0 | `rust-toolchain.toml` |
| clippy / rustfmt | — | toolchain |
| MSYS2 bash | — | AGENTS.md |
| Node / Playwright | — | `e2e/` |
| Cursor / opencode | 3.13.21 | service |

Rust модуль: `toolchain/` → `gsv_toolchain.json`.

## 4. IDE (opencode + cursor чати; вибір, з чим працювати)

**Роль:** портувати opencode + cursor чати; можливість обирати, з чим працювати.

- Читання сесій/чатів opencode (`~/.local/share/opencode/`) та cursor (`.cursor/`).
- Список сесій у UI; вибір активної → **останні 8 повідомлень** (`preview_messages` jsonl).
- Вибір робочого фолдера/спринту.

Rust модуль: `ide/` (read-only).

## 5. Update (оновлення бінарника; offline-стійкість)

**Роль:** якщо оновлюємо/дебажимо vision Rust-кодбазу і запущена bin-версія — сервер приймає **повідомлення про апдейт**; вебсторінка не падає при офлайн.

Поведінка:
1. Перекомпіляція → новий бінарник (`target/debug/`).
2. Canon listener — **live copy** `cargo xtask live` → `target/live/gsv-server.exe`.
3. UI: **«Update»** → `POST /api/update/apply` (SSE `offline`, process exit).
4. Сторінка не падає — «offline» лише під час swap; SSE `onopen` → resync.

Деталі: [`GSV_SERVER.md`](./GSV_SERVER.md) (endpoints `/api/update`, `/api/update/apply`, `/events`, live copy).

## 6. Box preview (Rust-кольори відповідно до синтаксису)

**Роль:** превʼю файлів, де **Rust-кольори відповідають синтаксису** (висвітлення синтаксису Rust).

- `GET /api/preview?file=…` → HTML з токен-висвітленням (Rust-палітра).
- Підтримка `.rs`, `.toml`, `.md`, `.js`, `.css`.
- Шлях лише repo-relative: `ParentDir` / absolute → reject; canonicalize під `repo_root`.

## 7. SLI terminal (AI → команди)

**Роль:** щоб AI (ШІ) міг посилати команди на сервер.

- `POST /api/terminal {command}` — виконати SLI-команду.
- Аудит у Tracker; результат — JSON/stdout.
- Обмеження: whitelist SLI-каталогу (без `bash`/`node`/`npm`/`cat`), cargo/git subcommand allowlist, sandbox (без `..` / shell metacharacters).
- Mutating POST з не-loopback `Origin` або `Sec-Fetch-Site: cross-site` → 403.
- POST body > 256 KiB → 413 `{ok:false}`. Responses include CSP / nosniff / `Cache-Control: no-store`.

## 8. Rust tests / benchmarks hook (без перекомпіляції)

**Роль:** запуск тестів/бенчмарків **без перекомпіляції** (read-only hook).

- `GET /api/hooks/tests` → статус + результати з `target/` (deps, `test-*` bins) без `cargo build`.
- `GET /api/hooks/bench` → Criterion medians (read `target/criterion/`).
- Дані не перебудовують проєкт — лише зчитують наявні артефакти.

## 9. OmniRouter (Rust AI-проксі/роутер)

**Роль:** Rust-роутер по AI-провайдерах (researched 2026-08-18) для **Rust + web** на OmniRouter, Cursor, OpenCode і Grok. Рекомендовані: Grok 4.6, GPT-5.2 Codex, Claude Sonnet 4.6, Gemini 3 Pro, Kimi K2.7 Code, GPT-5.3 Codex. Кожен провайдер має `quota.reset_secs` — MCP `gsv_omni_route` пропускає host у cooldown.

**Дані:**

| Поле | Джерело |
|------|---------|
| Каталог провайдерів | `catalog.rs` (19 providers, incl. xAI + Cursor) |
| Каталог моделей (ctx / rust / web / clients) | `catalog.rs` |
| Квоти / таймери | `quota.rs` → `data/omni_quota.json` (не git) |
| Конфіг / тюнінг | `GSV/data/omni.toml` + env `OMNI_<PROVIDER>_API_KEY` / `_BASE_URL` |
| Рекомендований список | rust+web research 2026-08-18 (6 моделей) |
| Canon notes | [`GSV_OMNI_CATALOG.md`](./GSV_OMNI_CATALOG.md) |

**Endpoints:**

- `GET /api/omni` — overview wire (providers, models, clients, quotas, recommended, routing).
- `GET /api/omni/route?task=rust|web&prefer_free=` — timer-aware next pick.
- `GET /api/omni/config` — конфіг **redacted** (лише `key_set`, без ключів).
- `POST /api/omni/config` — тюнінг (base_url / api_key / enabled / priority / routing).
- `GET /api/omni/v1/models` — OpenAI-сумісний список моделей.
- `POST /api/omni/v1/chat/completions` — OpenAI-сумісний proxy (empty `model` auto-picks; 429 starts cooldown; dry-run через `X-Omni-Dry-Run: 1`).
- `POST /api/omni/test {provider}` — connectivity check (`GET {base}/models`).

**Роутинг** (`proxy.rs::select_provider`): `X-Omni-Provider` header / `provider` у тілі → власник моделі з каталогу (skip cooling) → `free_fallback_order` → `routing.default_provider` → `routing.fallback_order` → найвищий пріоритет. `base_url` може вказувати на OmniRoute (`http://127.0.0.1:20128/v1`).

Rust модуль: `omni/` (catalog.rs, config.rs, proxy.rs, quota.rs) → `GSV/data/omni.toml`.

## Зведена таблиця

| Box | Rust module | Endpoint | Джерело даних |
|-----|-------------|----------|---------------|
| Tracker | `tracker/` | `/api/tracker` | FM §5.12, logs, loc-audit |
| SLI console | `sli/` | `/api/sli` | `bin/`, `scripts/`, `src/bin/` |
| Toolchain | `toolchain/` | `/api/toolchain` | toolchain, env |
| IDE | `ide/` | `/api/ide/…` | opencode/cursor сесії |
| Products | `products/` (`boxes/products.rs`) | `/api/products` · `/api/products/select` · `/api/products/open` · `/api/products/scan` | workspace ∪ sibling git ∪ kit; `registered` from PRODUCTS.md (`omniroute` band 149); scan HANDOFF fallback `AGENTS.md` / `docs/ROADMAP.md` |
| Fingerprints | `fingerprint/` (`boxes/fingerprint.rs`) | `/api/fingerprints` | append-only `docs/gsv/fingerprints.jsonl` (actor / IDE / model / agent / time); ops card `fingerprints`; **band 154:** `model` from `GSV_MODEL` else Cursor session (`CURSOR_MODEL` / `GSV_SESSION_FILE`); else latest Cursor `renderer.log` `catalogModelId`; default `unknown` |
| Service Worker | `sw/` (`boxes/sw.rs`) | `/sw.js` · `/api/sw` | Rust-rendered SW; Cache Storage `gsv-shell-v1`; precache `/` + live CSS + galaxy/vision svg; skip `/events` `/mcp`; ops card `sw` |
| Watchdog | `watchdog/` (`boxes/watchdog.rs`) | `/api/watchdog` · bin `gsv-watchdog` | probe `/api/health`; after 2 misses copy debug→live and spawn detached; heartbeat `target/live/watchdog.json`; health row `watchdog_alive`; **band 154:** ops card `watchdog`; **band 162:** `debug_newer` + POST apply when debug is newer than live |
| Usage | `usage/` (`boxes/usage.rs`) | `/api/usage` | per-session token counts from OmniRouter completions (**band 156:** includes `stream:true` SSE) + MCP bot (`gsv_omni_chat` / `Mcp-Session-Id`) + fail-open OmniRoute `/api/usage/history`; persist `data/gsv_usage.json`; vision-sync snapshot; Galaxy studio card `usage` (**band 155**) |
| Update | `update/` | `/api/update` · `/api/update/apply` · `/events` | live copy + версія; **band 162:** `crate_version` / `version_lag`; health uses the same `update_available` |
| Box preview | `preview/` | `/api/preview` | файли |
| SLI terminal | `terminal/` | `/api/terminal` | SLI-каталог |
| Tests/bench hooks | `hooks/` | `/api/hooks/…` | `target/` артефакти |
| OmniRouter | `omni/` | `/api/omni/…` | shared catalog + `omni.toml` + quota timers |
| Vision | `vision/` (`boxes/vision.rs`) | `/api/vision*` · `/assets/vision.svg` | `GSV/docs/vision/{manifest,feed,extensions}.json` → `GSV/data/gsv_*.json` |
| UI fragments | `ui/` (`boxes/ui.rs`) | `/api/ui/layout` · `/api/ui/card/:name` · `/api/ui/load-palette` · `/api/ui/load-theme` | dashboard `CARD_NAMES` 37 + chrome 8 + layout `html`/`header` + live `:root` CSS; **band 156:** `.card.fullscreen img{max-height:none`; **band 155:** studio card `usage`; **band 154:** ops card `watchdog`; **band 148:** ops card `sw`; **band 146:** ops card `fingerprints`; **band 145:** ops card `products` (list/select/open/scan); **band 143:** power menu `z-index:80` above workspace, exclusive fullscreen (`data-action='card-fs'`), collapsed cards `display:none` (dock restore), `--fs-*` type scale, speed/rust SVG height 168 |
| Stand smoke | `src/bin/gsv_http_stand_smoke.rs` | live HTTP перевірка | всі boxes + `/api/vision*` + SVG + `/api/ui/card/:name` |
| **gsv_mcp_openbot** | `mcp.rs` + `gsv-mcp` bin | stdio live copy + `GET`/`POST`/`DELETE /mcp` + Galaxy card `/api/ui/card/mcp` | 36 box tools + 10 `gsv://` resources (band **160** GSV sandbox + no User MCP · **159** Cursor HTTP url + session SSE hold · **158** live stdio + sync check · **157** omni route) — [`GSV_OMNI_CATALOG.md`](./GSV_OMNI_CATALOG.md) · [`GSV_MCP_OPENBOT.md`](./GSV_MCP_OPENBOT.md) |
