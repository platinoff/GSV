# GSV Server — exe/bin «Galaxy StarWalker Vision»

Специфікація бінарного сервера проєкту GSV. Окремий Rust bin `gsv-server`, що віддає UI та реалізує бокси.

## Призначення

- **Bin/exe** «Galaxy StarWalker Vision» — `cargo run --bin gsv-server` (або зібраний `.exe` на Windows).
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
| GET | `/api/update` | статус оновлення (Update box) |
| GET | `/api/preview` | превʼю з Rust-синтаксис-кольорами |
| POST | `/api/terminal` | SLI terminal — виконати команду (AI) |
| GET | `/api/hooks/tests` | результати тестів (read-only, без build) |
| GET | `/api/hooks/bench` | Criterion medians (read-only) |
| GET | `/api/omni` | OmniRouter overview (providers, models, recommended, routing) |
| GET | `/api/omni/config` | OmniRouter конфіг (redacted: лише `key_set`) |
| POST | `/api/omni/config` | тюнінг провайдерів (base_url/api_key/enabled/priority/routing) |
| GET | `/api/omni/v1/models` | OpenAI-сумісний список моделей |
| POST | `/api/omni/v1/chat/completions` | OpenAI-сумісний proxy (dry-run через `X-Omni-Dry-Run: 1`) |
| POST | `/api/omni/test` | connectivity check провайдера (`GET {base}/models`) |
| GET | `/api/health` | health-чек |
| GET | `/mcp` | MCP discovery (`gsv_mcp_openbot` name + 26 tools + 6 resources + 3 prompts + `stdio`/`http`/`tool_count`/`resource_count`/`prompt_count`/`logging`/`completions`/`log_level`) |
| POST | `/mcp` | MCP JSON-RPC (initialize / tools/* / resources/* / prompts/* / logging/setLevel / completion/complete); stdio twin is `gsv-mcp` |
| GET | `/api/ui/layout` | grouped IA (ops/vision/sprint/studio) + `chrome` (8) + `html` (sidebar nav) + `header` (GPU/Auto/Power) |
| GET | `/api/ui/card/:name` | Rust-rendered card body HTML (`CARD_NAMES`) |
| GET | `/api/ui/load-palette` | live Galaxy `:root` CSS (`GalaxyPalette::as_css_root`) |
| GET | `/api/ui/load-theme` | live sprint `:root` CSS (`SprintThemeReport::as_css_root`) |
| GET | `/data/{file}` | allowlisted JSON snapshot under `data/` (no `omni.toml`) |
| GET | `/events` | SSE: update · offline/online · metrics resync |

## Local bind + mutate (band 133)

- Default `--host` is `127.0.0.1`. Off-loopback bind requires `--allow-lan`.
- POST with `Sec-Fetch-Site: cross-site` or a non-loopback `Origin` → 403 `{ok:false,error}`.
- `GET /data/{file}` is a basename allowlist (`gsv_*.json` / `rust_ratio.json`); `omni.toml` is not served.
- SLI terminal: no `bash`/`node`/`npm`/`cat`; `cargo`/`git` subcommand allowlists.

## HTTP response hardening (band 134)

Every response (including 403/413) carries:

| Header | Value |
|--------|--------|
| `Content-Security-Policy` | `default-src 'self'` + inline script/style (embedded UI) + `frame-ancestors 'none'` |
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

Реалізація (Rust):
- Сервер тримає `update_flag` (AtomicBool) + версію бінарника.
- Під час заміни бінарника (hot-swap файлу/процесу) сервер надсилає SSE подію `update_available`.
- UI показує кнопку/бейдж **Update** замість auto-reload.
- Клієнтський JS тримає стан offline в `navigator.onLine` / heartbeat SSE; при реконекті робить `GET /api/...` full-resync та оновлює метрики (Tracker/SLI/toolchain/speed/rust diagnostics).

## Offline-стійкість

- Static assets (UI) кешуються у Service Worker / localStorage → сторінка відкривається офлайн.
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
