# GSV Architecture — Galaxy StarWalker Vision

Архітектура окремого проєкту **GSV**. Міграція vision-системи (`GSV/docs/vision/`) у самостійний Rust-first проєкт `GSV/` з бінарним сервером та боксами.

## Принципи

- **Rust 95–100%** — runtime, API, ML, tools, бокси, сервер.
- **WebAssembly 0–5%** — лише горизонт (за потреби — маленькі wasm-модулі з `crates/poolai-ui-wasm`).
- **UI** — тонкий JS/DOM glue поверх Rust API; сторінка оновлюється через серверні події (SSE/WS), а не через перезавантаження.
- **Bind** — default `127.0.0.1:9999`; `--allow-lan` required to listen beyond loopback. Mutating POSTs from a non-local Origin are rejected. Responses carry CSP / `X-Content-Type-Options: nosniff` / `X-Frame-Options: DENY` / `Cache-Control: no-store`; POST bodies are capped at 256 KiB.
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

`src/bin/gsv_server.rs` → `cargo run --bin gsv-server`. Порт HTTP+WS, віддає UI + API боксів + події.

| Модуль | Роль |
|--------|------|
| `gsv_server` | точка входу, axum+tokio server |
| `server/` | HTTP/WS routing, static (GSV UI), SSE events |
| `boxes/` | Tracker, SLI console, Toolchain, IDE, Update, Box preview, SLI terminal, Tests/bench hooks |
| `sli/` | SLI-функції: парсинг `bin/`+`scripts/`+`src/bin/` → каталог команд |
| `toolchain/` | інвентар тулів (rustc/cargo/clippy/MSYS2/…) |
| `tracker/` | зберігання параметрів виконаного workflow (JSON store) |
| `ide/` | читання opencode/cursor чатів; вибір сесії |
| `update/` | перевірка оновлення бінарника; сигнал «Update»; offline resync |
| `mcp/` | `gsv_mcp_openbot` JSON-RPC (stdio `gsv-mcp` + `POST /mcp`) |

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

## Порядок реалізації (коротко)

Повний порядок зі спринтами — [`GSV_TECH_ROADMAP.md`](./GSV_TECH_ROADMAP.md). Логіка: **docs/architecture → server scaffold → SLI console + Tracker → Toolchain → IDE → Update/offline → Preview + SLI terminal → Tests/bench hooks → band close**. MCP: [`GSV_MCP_OPENBOT.md`](./GSV_MCP_OPENBOT.md) (band 135–138 ✅: stdio + `/mcp` + Galaxy card + 26 tools + 6 resources + 3 prompts).
