# GSV Architecture — Galaxy StarWalker Vision

Архітектура окремого проєкту **GSV**. Міграція vision-системи (`GSV/docs/vision/`) у самостійний Rust-first проєкт `GSV/` з бінарним сервером та боксами.

## Принципи

- **Rust 95–100%** — runtime, API, ML, tools, бокси, сервер.
- **WebAssembly 0–5%** — лише горизонт (за потреби — маленькі wasm-модулі з `crates/poolai-ui-wasm`).
- **UI** — тонкий JS/DOM glue поверх Rust API; сторінка оновлюється через серверні події (SSE/WS), а не через перезавантаження.
- **Bind** — default `127.0.0.1:9999`; `--allow-lan` required to listen beyond loopback. Mutating POSTs from a non-local Origin are rejected. Responses carry CSP (`worker-src 'self'`) / `X-Content-Type-Options: nosniff` / `X-Frame-Options: DENY` / `Cache-Control: no-store`; POST bodies are capped at 256 KiB.
- **Без Python/Java.** Bins — лише `src/bin/`.

## Шари (L0–L5)

| ID | Шар | GSV-зміст |
|----|-----|-----------|
| L0 | Concept | концепт GSV (цей docs-каталог), `POOLAI_GALAXY_GRID` |
| L1 | Operations | workflow/спринти (FM §5.12 band 102), HANDOFF/NEXT |
| L2 | Catalog | FM, DIGEST — джерела для Tracker/SLI console |
| L3 | Code | `src/` сервера GSV (бокси, sli, toolchain, tracker, ide, update) |
| L4 | Lib roots | `src/lib.rs`, `crates/poolai-ui-core`, `crates/poolai-ui-wasm` |
| L5 | Workspace | `GSV/Cargo.toml`, `Cargo.toml` (workspace) |

## Компоненти (Rust)

### gsv-server (bin)

`src/bin/gsv_server.rs`. Canon run is `cargo xtask live` (`gsv-live`): copies `target/debug/gsv-server.exe` → `target/live/gsv-server.exe` (and `gsv-mcp.exe` / `gsv-watchdog.exe` when built) and execs the server copy on `127.0.0.1:9999`. `cargo test` / `cargo build` may overwrite `target/debug/` without killing the listener. `POST /api/update/apply` exits so the supervisor recopies. `cargo run --bin gsv-server` still works but locks `target/debug/` on Windows. Spec: [`GSV_ALWAYS_ON_UI.md`](./GSV_ALWAYS_ON_UI.md).

| Модуль | Роль |
|--------|------|
| `gsv_server` | точка входу, axum+tokio server |
| `server/` | HTTP/WS routing, static (GSV UI), SSE events |
| `boxes/` | Tracker, SLI console, Toolchain, IDE, Update, Box preview, SLI terminal, Tests/bench hooks |
| `sli/` | SLI-функції: парсинг `bin/`+`scripts/`+`src/bin/` → каталог команд |
| `toolchain/` | інвентар тулів (rustc/cargo/clippy/MSYS2/…) |
| `tracker/` | зберігання параметрів виконаного workflow (JSON store) |
| `ide/` | читання opencode/cursor чатів; вибір сесії |
| `products/` | VDT environment picker (discover / select / confined open / scan) |
| `sw/` | Service Worker shell cache (`GET /sw.js` Rust-rendered; `GET /api/sw`) |
| `watchdog/` | live watchdog heartbeat (`GET /api/watchdog`) + bin `gsv-watchdog` + Galaxy ops card `watchdog` (band **154 ✅** · **180 ✅** process lockstep) |
| `usage/` | session token usage (`GET /api/usage`) — OmniRouter + MCP bot + OmniRoute; Galaxy studio card `usage` (band **155 ✅**) |
| `settings/` | Godfather settings (`GET`/`POST /api/settings`) — redacted token; Galaxy ops card `settings` (band **166 ✅**) |
| `telegram/` | Godfather bind (`GET /api/telegram`) — dry-run stub in tests; Galaxy ops card `telegram` (band **167 ✅**); inbound poller band **179 ✅** |
| `tickets/` | Ticket board (`GET`/`POST /api/tickets` · claim/done/error/presence/reclaim/walk) — scenarios + solo/squad + lease + MDS band walk; Galaxy ops card `tickets` (band **168 ✅** · **170 ✅** · **171 ✅** · **175 ✅**) |
| `mds/` | Light memory / disk / speed probe (`GET /api/mds` · `gsv-mds`) — band **175 ✅** |
| `update/` | перевірка оновлення бінарника; сигнал «Update»; offline resync |
| `mcp/` | `gsv_mcp_openbot` JSON-RPC (stdio `target/live/gsv-mcp.exe` + Cursor HTTP `http://127.0.0.1:9999/mcp`); **53** tools + **11** `gsv://` (band **179 ✅** `gsv_telegram_poll` · **178 ✅** `gsv_tickets_bench` · **177 ✅** `gsv_tickets_hook` · **175 ✅** `gsv_tickets_walk` + `gsv_mds` · **174 ✅** `gsv_telegram_ticket` · **171 ✅** ticket reclaim · **170 ✅** ticket create/done/error/presence · **169 ✅** `gsv_telegram_bus_*`; **168 ✅** `gsv_tickets` + `gsv_tickets_claim`; **167 ✅** `gsv_telegram`; **166 ✅** `gsv_settings`; **164 ✅** Cursor 3.16.29 kit lockstep; **159 ✅** Cursor HTTP + session SSE hold; **158 ✅** live copy + sync `--check`; **157 ✅** omni route) |

### UI (тонкий JS glue)

`ui/` — vanilla HTML+CSS+JS (thin glue). Панелі: map, sprint-queue, doc-preview + **бокси GSV**; chrome (RSS ticker, GPU, power, node-search) з `/api/ui/card/:name`; sidebar nav HTML + header actions з `/api/ui/layout` `html`/`header`; live `:root` з `/api/ui/load-palette` + `/api/ui/load-theme`. Дані — з Rust API (fetch/SSE). Сторінка переживає офлайн (див. `GSV_SERVER.md`).

## Rust / WebAssembly split

| Область | Rust | Wasm |
|---------|------|------|
| Сервер, API, бокси | ✅ 100% | — |
| SLI-парсинг, toolchain-інвентар | ✅ 100% | — |
| Tracker-зберігання | ✅ 100% | — |
| Update/offline-механіка | ✅ 100% | — |
| Форматування чисел/дати в UI | — | ⏳ 0–5% (horizon, `poolai-ui-wasm`) |
| AI-чати (IDE box) | ✅ (читання) | — |

## Дані / зберігання

| Дані | Формат | Місце |
|------|--------|-------|
| Workflow params (Tracker) | `gsv_tracker.json` | `GSV/data/` |
| SLI-каталог | `gsv_sli.json` (згенерований) | `GSV/data/` |
| Toolchain-інвентар | `gsv_toolchain.json` | `GSV/data/` |
| Сесії чатів (opencode/cursor) | read-only | `~/.local/share/opencode/`, `.cursor/` |
| Метрики (speed, rust diagnostics) | `speed_index.json`, `rust_diagnostics.json` | `docs/vision/` (сирці) → `GSV/data/gsv_*.json` |
| Session token usage | `gsv_usage.json` | `GSV/data/` (OmniRouter + MCP + OmniRoute pull) |
| Settings / Godfather token | `gsv_settings.json` | `GSV/data/` (gitignored; **band 166 ✅**; API/MCP redact `bot_token`; env `GSV_TELEGRAM_BOT_TOKEN` wins) |
| Tickets / claims | `tickets.jsonl` / `ticket_claims.jsonl` | `docs/gsv/` git-tracked (**band 168 ✅**; no secrets) |

## Порядок реалізації (коротко)

Повний порядок зі спринтами — [`GSV_TECH_ROADMAP.md`](./GSV_TECH_ROADMAP.md). Логіка: **docs/architecture → server scaffold → SLI console + Tracker → Toolchain → IDE → Update/offline → Preview + SLI terminal → Tests/bench hooks → band close**. MCP: [`GSV_MCP_OPENBOT.md`](./GSV_MCP_OPENBOT.md) (band 135–**180 ✅**: watchdog process lockstep · inbound poller · scenario benchmark · roadmap/plan hook-up · MDS scenario walk · solo Telegram tickets · vision queue close-lockstep · live crate lockstep · ticket lease/reclaim · ticket solo/squad · Telegram bus · ticket board + MCP claim · Godfather bind + settings store · watchdog live copy + lockstep observability · vision queue lockstep + bump auto-advance · GSV sandbox `S:/rust/GSV` + folder MCP only + live stdio `gsv-mcp` + `/mcp` CSRF skip + `gsv_xtask` `sync` `--check` + notify all subscribed `gsv://` + Galaxy card + **53** tools + **11** resources). Next gsv drain: **owner pick** after a warnings-first scan.
