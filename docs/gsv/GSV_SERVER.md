# GSV Server — exe/bin «Galaxy StarWalker Vision»

Специфікація бінарного сервера проєкту GSV. Окремий Rust bin `gsv-server`, що віддає UI та реалізує бокси.

## Призначення

- **Bin/exe** «Galaxy StarWalker Vision» — canon always-on: `cargo xtask live` (`target/live/gsv-server.exe`). `cargo run --bin gsv-server` still works but **locks** `target/debug/` on Windows.
- Віддає static UI (спадкоємець деактивованого legacy `GSV/docs/vision/index.html` — band 117) + REST API боксів + події (SSE).
- Працює як **self-contained server**: доки + метрики + бокs — все в одному Rust бінарнику.

## Endpoints

| Метод | Шлях | Опис |
|-------|------|------|
| GET | `/` | GSV UI (index.html) |
| GET | `/GSV/docs/gsv/…` | docs проєкту |
| GET | `/api/tracker` | параметри виконаного workflow (Tracker box) |
| GET | `/api/sli` | SLI-каталог (команди + функції) |
| GET | `/api/toolchain` | інвентар тулів |
| GET | `/api/ide/sessions` | список сесій (opencode/cursor) |
| POST | `/api/ide/select` | вибір сесії, з чим працювати |
| GET | `/api/products` | VDT environment projects (`ok`, `products`, `selected`) |
| POST | `/api/products/select` | `{id}` → process-local selection; unknown id → 404 `{ok:false}` |
| POST | `/api/products/open` | `{id}` → open folder (`cursor` if on PATH, else `explorer`); id must be discovered |
| GET | `/api/products/scan` | selected product: git HEAD/status, kind, registered, HANDOFF/NEXT (`AGENTS.md` / `docs/ROADMAP.md` fallback), `cargo_name` |
| GET | `/api/fingerprints` | drain fingerprints (`ok`, `path`, `count`, `fingerprints`; `?limit=` default 20, cap 100) |
| GET | `/sw.js` | Rust-rendered Service Worker (shell Cache API; `Service-Worker-Allowed: /`) |
| GET | `/api/watchdog` | live watchdog heartbeat (`ok`, `alive`, `path`, `last_action`, `debug_newer`, `bin_version`, `crate_version`, `version_lag`) |
| GET | `/api/xtask` | cargo xtask catalog (`?task=catalog\|products\|disk`; mutating names → 400) |
| GET | `/api/disk` | S0 disk guard (`ok`, `free_gb`, `target_gb`; `?enforce=true`) |
| GET | `/api/update` | статус оновлення (Update box; `live_copy`; `crate_version` / `version_lag`) |
| POST | `/api/update/notify` | виставити `update_available` + SSE |
| POST | `/api/update/apply` | SSE `offline` + `{ok,applying}`; process exit unless `GSV_UPDATE_APPLY_EXIT=0` |
| GET | `/api/preview` | превʼю з Rust-синтаксис-кольорами |
| POST | `/api/terminal` | SLI terminal — виконати команду (AI) |
| GET | `/api/hooks/tests` | результати тестів (read-only, без build) |
| GET | `/api/hooks/bench` | Criterion medians (read-only) |
| GET | `/api/omni` | OmniRouter overview (providers, models, clients, quotas, recommended, routing) |
| GET | `/api/omni/route` | Timer-aware next pick (`task=rust|web`, `prefer_free`) |
| GET | `/api/omni/config` | OmniRouter конфіг (redacted: лише `key_set`) |
| POST | `/api/omni/config` | тюнінг провайдерів (base_url/api_key/enabled/priority/routing) |
| GET | `/api/omni/v1/models` | OpenAI-сумісний список моделей |
| POST | `/api/omni/v1/chat/completions` | OpenAI-сумісний proxy (dry-run через `X-Omni-Dry-Run: 1`) |
| GET | `/api/omni/test` | connectivity check провайдера (`GET {base}/models`) |
| GET | `/api/usage` | per-session token totals (OmniRouter + MCP + OmniRoute pull; `data/gsv_usage.json`) |
| GET | `/api/settings` | Godfather settings (redacted: `token_set`, never `bot_token`; `data/gsv_settings.json`) |
| POST | `/api/settings` | owner write (loopback CSRF); stores posted token; response redacted |
| GET | `/api/telegram` | Godfather bind status (`ok`, `channel_id`, `token_set`, `bot_username`, `chat_title`, `last_probe`, `polling`, `last_bus_ts`, `last_bus_error`; never `bot_token`). `X-Telegram-Dry-Run: 1` forces the in-process stub. |
| GET | `/api/telegram/bus` | Poll bus envelopes (`messages`; requires `telegram-relay`; dry-run queue in tests). |
| POST | `/api/telegram/bus` | Send a bus envelope `{from,to?,ticket_id?,body}` (CSRF; body cap 2 KiB; never `bot_token`). |
| POST | `/api/telegram/ticket` | Ingest a Godfather message as a ticket (`{from,body,product?}`; `/ticket` or `{kind:ticket}`). Solo MCP auto-claims when online. Requires `telegram-relay` + `ticket-claim`. CSRF; never `bot_token`. |
| GET | `/api/tickets` | Ticket board (`ok`, `tickets[]`, `mode`, `lease_secs`, `online`, `scenarios`, `events`; missing JSONL empty-ok; expired WIP auto-reclaimed) |
| POST | `/api/tickets` | create `{title,body?,product?}` or `{scenario_id}` (loopback CSRF); registered product only; may auto-assign |
| POST | `/api/tickets/claim` | `{id}` → `open`→`in_progress` + `claimed_by` + append `docs/gsv/ticket_claims.jsonl`; unknown id → 404; `ticket-claim` off → 403 |
| POST | `/api/tickets/done` | `{id,note?}` → `in_progress`→`done` + event `kind:done` |
| POST | `/api/tickets/error` | `{id,note?}` → `in_progress`→`blocked` + event `kind:error` |
| POST | `/api/tickets/presence` | heartbeat `{actor?,ide?,model?,agent?}` → `{ok,online}` + renew this worker's WIP leases |
| POST | `/api/tickets/reclaim` | `{id?}` stale/explicit `in_progress` → `open` + event `kind:reclaimed`; empty id = all expired; CSRF; `ticket-claim` off → 403 |
| POST | `/api/tickets/walk` | Walk `{scenario_id?,create?,from?}`: claim/assign→done + Godfather session lines (`solo claimed` / `squad assigned` / `bench gsv_dev`). Live `sendMessage` 1/s; tests dry-run. Requires `ticket-claim` + `telegram-relay`. CSRF. |
| POST | `/api/tickets/hook` | Hook `{phrase?|source+id,walk?,from?}`: place ≤10 tickets from catalog / roadmap band / superpowers plan. Phrase `run mcp bot hook up scenario …`. CSRF. |
| GET | `/api/tickets/bench` | Last `abrakadabra-session` Instant timings (`docs/gsv/scenario_bench.json`; missing → `recorded:false` zeros) |
| POST | `/api/tickets/bench` | `{run:true}` times a throwaway session walk and persists JSON. CSRF. Default `{run:false}` is a read. |
| GET | `/api/mds` | Light memory / disk / speed report (`gsv-mds`) |
| GET | `/api/health` | health-чек (`crate_version`, `version_lag`, `update_available` matches Update box) |
| GET | `/mcp` | MCP discovery (`gsv_mcp_openbot` + `sandbox` GSV crate path + 52 tools + 11 resources + 3 prompts + `stdio`/`stdio_live`/`http`/`http_url`/`version`/`http_csrf`/`tool_count`/`resource_count`/`prompt_count`/`logging`/`completions`/`log_level`/`subscribe`/`subscription_count`/`sse`/`streamable`/`sessions`/`session_count`); sessionless `Accept: text/event-stream` flushes pending notifications as finite SSE; **GET with `Mcp-Session-Id` holds** the stream; unknown `Mcp-Session-Id` → 404 |
| POST | `/mcp` | MCP JSON-RPC (initialize / tools/* / resources/* including subscribe/unsubscribe / prompts/* / logging/setLevel / completion/complete); skips browser CSRF (bots); `initialize` issues `Mcp-Session-Id`; unknown id → 404; `Accept: text/event-stream` → SSE notifications then result; stdio twin is `target/live/gsv-mcp.exe` |
| DELETE | `/mcp` | End HTTP MCP session (`Mcp-Session-Id` required; missing → 400; unknown → 404) |
| GET | `/api/ui/layout` | grouped IA (ops/vision/sprint/studio) + `chrome` (8) + `html` (sidebar nav) + `header` (GPU/Auto/Power) |
| GET | `/api/ui/card/:name` | Rust-rendered card body HTML (`CARD_NAMES` 40, incl. `tickets` + `telegram` + `settings` + `usage` + `watchdog` + `sw` + `products` + `fingerprints`) |
| GET | `/api/ui/load-palette` | live Galaxy `:root` CSS (`GalaxyPalette::as_css_root`) |
| GET | `/api/ui/load-theme` | live sprint `:root` CSS (`SprintThemeReport::as_css_root`) |
| GET | `/data/{file}` | allowlisted JSON snapshot under `data/` (no `omni.toml`) |
| GET | `/events` | SSE: update · offline/online · metrics resync |

## Local bind + mutate (band 133)

- Default `--host` is `127.0.0.1`. Off-loopback bind requires `--allow-lan`.
- POST with `Sec-Fetch-Site: cross-site` or a non-loopback `Origin` → 403 `{ok:false,error}` (except `POST /mcp`, which skips browser CSRF so Cursor/Grok Bot can call JSON-RPC).
- `GET /data/{file}` is a basename allowlist (`gsv_*.json` / `rust_ratio.json`); `omni.toml` and `gsv_settings.json` are not served.
- SLI terminal: no `bash`/`node`/`npm`/`cat`; `cargo`/`git` subcommand allowlists.

## HTTP response hardening (band 134)

Every response (including 403/413) carries:

| Header | Value |
|--------|--------|
| `Content-Security-Policy` | `default-src 'self'` + inline script/style (embedded UI) + `worker-src 'self'` + `frame-ancestors 'none'` |
| `X-Content-Type-Options` | `nosniff` |
| `X-Frame-Options` | `DENY` |
| `Referrer-Policy` | `no-referrer` |
| `Permissions-Policy` | camera/microphone/geolocation `()` |
| `Cross-Origin-Opener-Policy` | `same-origin` |
| `Cross-Origin-Resource-Policy` | `same-origin` |
| `Cache-Control` | `no-store` |

POST bodies over **256 KiB** (`security::MAX_BODY_BYTES`) → 413 `{ok:false,error:"request body too large"}`. Axum `DefaultBodyLimit` matches that cap for chunked bodies without `Content-Length`.

## Stand smoke (`gsv-http-stand-smoke`, band 126)

Live HTTP smoke бінарника сервера — мірор poolAI `poolai-http-stand-smoke` (PH-S1900):

```bash
# канон-порт 9999:
cargo run --manifest-path GSV/Cargo.toml --bin gsv-http-stand-smoke
# JSON-репорт + кастомний base-url:
cargo run --manifest-path GSV/Cargo.toml --bin gsv-http-stand-smoke -- --base-url http://127.0.0.1:9999 --json
```

- Перевіряє core boxes (`/api/health`, `/api/tracker`, `/api/sli`, `/api/toolchain`, `/api/update`, `/api/ratio`, `/api/omni/status`), усі `/api/vision*` (ok-гейт), SVG-ассети та **усі 32 зареєстрованих карток** `/api/ui/card/:name` (non-empty `html`).
- Layout: `GET /api/ui/layout` — 4 групи (ops / vision / sprint / studio), default `sprint`, `chrome` (8 fragments: galaxy-backdrop / starfield / rss-ticker / gpu-mode / power-menu / panel-dock / fullscreen / node-search), `html` (sidebar nav inner HTML with `data-card-jump`), `header` (GPU / Auto / Resync / Power `data-action`).
- **Band 143 chrome:** header stacking `z-index ≥ 40`, `.power-menu` `z-index:80` (no `body>header,.workspace{z-index:2}`); collapse removes the card from the grid (dock chip restore); at most one `.fullscreen` card; Esc calls `exitFullscreen()` via `data-action='card-fs'`. Type scale `--fs-ui:13px` / `--fs-card:12px` / `--fs-meta:11px` / `--fs-chart:11px`; card body `max-height:420px`; speed/rust SVG canvas height 168, font-size 11, `ui-monospace` stack.
- Shell CSS: `GET /api/ui/load-palette` + `GET /api/ui/load-theme` — live `:root` stylesheets (inline `:root` in `ui/index.html` remains the offline fallback).
- `ok`-гейт лише там, де wire має поле `ok` (vision*/ratio/health/cards); struct-wire endpoints (tracker/sli/toolchain/update/omni) — лише 200 + JSON (empty-tolerant).
- Вихідний код: `GSV/src/bin/gsv_http_stand_smoke.rs`; контракти: `GSV/tests/gsv_stand_smoke_contracts.rs`.
- Репорт: `{base_url, ok, passed, failed, cases[], tool}`; exit code 1 при будь-якому FAIL.

## Update-повідомлення (Update box)

Ключова вимога: **якщо запущено bin-версію, сервер приймає повідомлення про апдейт.**

Сценарій (з ТЗ):
1. Йде перекомпіляція vision Rust-кодбази на **новий бінарник**.
2. Замість «reload» у UI з’являється **«Update»**.
3. Вебсторінка **не падає** при офлайн — переходить у стан «offline».
4. Після відновлення зв’язку **всі метрики синхронізуються** (resync).

**Always-on (band 144 + watchdog + band 153 rust live):** run a live copy (`target/live/gsv-server.exe` via `cargo xtask live`) so `cargo test`/`build` does not lock the listening process. `POST /api/update/apply` emits SSE `offline` and exits (gated by `GSV_UPDATE_APPLY_EXIT`); the supervisor recopies debug → live and rebinds `:9999`; the page stays **offline** until SSE `onopen` then resyncs. If the supervisor process dies (Cursor abort), `gsv-watchdog` probes `/api/health` and respawns the live copy. When debug is newer than a healthy live copy (or health `version_lag`), the watchdog POSTs `/api/update/apply` (lockstep) so Windows can recopy after exit. Failed apply is heartbeat `lockstep-fail` plus `last_apply_status` / `lockstep_note`. Cooldown while still needed is `lockstep-wait` (never silent `probe-ok`). Oneshot apply uses debug-newer **or** health lag, and only yields if the peer pid is still alive. A stale watchdog exe spawns a successor (debug → live hop). Heartbeat `bin_version` vs crate `version_lag` is on `GET /api/watchdog`. `cargo xtask watchdog` copies `gsv-watchdog` to `target/live/` so cargo can overwrite debug. Do **not** kill the live copy before `cargo test`. Spec: [`GSV_ALWAYS_ON_UI.md`](./GSV_ALWAYS_ON_UI.md).

Реалізація (Rust):
- Сервер тримає `update_flag` (AtomicBool) + версію бінарника.
- Під час заміни бінарника (hot-swap файлу/процесу) сервер надсилає SSE подію `update_available`.
- UI показує кнопку/бейдж **Update** замість auto-reload; `doUpdate()` POSTs `/api/update/apply`.
- Клієнтський JS тримає стан offline; при SSE `onopen` робить full-resync (Tracker/SLI/toolchain/speed/rust diagnostics).

**Horizon:** band **174** solo Telegram tickets (`gsv_telegram_ticket` · `/ticket` ingest). Band **173** vision queue close-lockstep (`bump --band N` = last of N / first of N+1). Band **172** live crate lockstep. Settings/Telegram/tickets **166–171 ✅**. Next drain = **owner pick**. Spec: [`GSV_SETTINGS_TELEGRAM.md`](./GSV_SETTINGS_TELEGRAM.md) · [`GSV_OMNI_CATALOG.md`](./GSV_OMNI_CATALOG.md) · [`GSV_RUST_DEV.md`](./GSV_RUST_DEV.md).

## Live copy + apply (band 144)

Windows locks a running exe. The canon listener is a **copy**:

```bash
cargo build --bin gsv-server --bin gsv-watchdog --bin gsv-live
cargo xtask live                 # copies target/debug → target/live, loop restart
cargo xtask watchdog             # detached health probe + respawn if :9999 dies
cargo xtask watchdog-install     # ONLOGON scheduled task (survives Cursor)
```

| Step | What happens |
|------|----------------|
| 1 | Supervisor copies `target/debug/gsv-server.exe` → `target/live/gsv-server.exe` and execs `--host 127.0.0.1 --port 9999` |
| 2 | `cargo test` / `cargo build` may overwrite `target/debug/` — no os error 5 on the listener |
| 3 | UI **Update** → `POST /api/update/apply` → SSE `event: offline` + `{ok:true, applying:true}` |
| 4 | Process exits unless `GSV_UPDATE_APPLY_EXIT=0` (tests / harness under `target/debug/deps/` skip exit) |
| 5 | Supervisor recopies debug → live, rebinds `:9999`; page SSE `onopen` → `resync()` → online |

Do **not** kill `target/live/gsv-server.exe` before `cargo test`. Only stop `target/debug/gsv-server.exe` if *that* file is still the listener.

`target/live/` is gitignored. Spec: [`GSV_ALWAYS_ON_UI.md`](./GSV_ALWAYS_ON_UI.md).

## Offline-стійкість

- Static assets (UI) кешуються у Service Worker (`GET /sw.js`, Cache Storage `gsv-shell-v1`) → сторінка `/` відкривається офлайн. `/events`, `/mcp`, non-GET — network-only.
- Жодних повних reload-ів без потреби: зміна даних → SSE подія → частковий re-render.
- Якщо сервер недоступний: UI показує статус «offline», дані лишаються на екрані, при реконекті — resync.

## Залежності (план)

`tokio`, `axum` (або `rocket`), `serde`, `serde_json`, `tracing`, `tower-http` (static). Все — Rust, у `GSV/Cargo.toml`.

## Тести (Rust)

- `tests/gsv_server_contracts.rs` — API-контракти (HTTP/4xx/JSON), Rust-інтеграційні тести.
- `tests/gsv_omni_contracts.rs` — контракти OmniRouter (catalog, redacted config, dry-run proxy, v1/models).
- `tests/gsv_update_flow.rs` — сценарій update/offline/resync (state machine + SSE).
- Playwright — лише для браузерного UI (DOM), не для API-дублювання.

## Хук оновлення (без перекомпіляції — Tests/bench box)

Окремо в [`GSV_BOXES.md`](./GSV_BOXES.md): сервер читає `target/…/deps` результати тестів/бенчмарків **без перекомпіляції** (read-only запуск `cargo test` / `criterion` через `/api/hooks`).
