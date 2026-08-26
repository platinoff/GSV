# GSV docs — Galaxy StarWalker Vision

Канон окремого Rust-first репо **`S:\rust\GSV`** ([github.com/platinoff/GSV](https://github.com/platinoff/GSV)). Sibling of PoolAI — **not** a subfolder. Live UI: [http://127.0.0.1:9999/](http://127.0.0.1:9999/).

**Rust 95–100% · WebAssembly 0–5% (horizon) · без Python/Java.** Product tests/benches/scripts = `.rs` (`cargo xtask`).

Install + what to do: root [`README.md`](../../README.md). SMIL tiles: [`../assets/presentations/README.md`](../assets/presentations/README.md). Session memory: [`../MEMORY.md`](../MEMORY.md).

## Документи

| Файл | Призначення |
|------|-------------|
| [`GSV_ARCHITECTURE.md`](./GSV_ARCHITECTURE.md) | Архітектура сервера + боксів; Rust/wasm split; шари L0–L5 |
| [`GSV_SERVER.md`](./GSV_SERVER.md) | `gsv-server`: endpoints, update, offline, metrics resync |
| [`GSV_BOXES.md`](./GSV_BOXES.md) | Специфікація боксів |
| [`GSV_TECH_ROADMAP.md`](./GSV_TECH_ROADMAP.md) | TechPreroadMap (bands 102–**208**; Telenetis 208 ✅) |
| [`../telenetis/README.md`](../telenetis/README.md) | Telenetis Mini App + Bot (band 208, Axum 9800) |
| [`GSV_ALWAYS_ON_UI.md`](./GSV_ALWAYS_ON_UI.md) | Always-on live binary, chrome, products, fingerprints (**143–150 ✅**) |
| [`GSV_POST_ALWAYS_ON.md`](./GSV_POST_ALWAYS_ON.md) | After always-on: MCP catch-up (**151–165 ✅**) |
| [`GSV_VDT_KIT.md`](./GSV_VDT_KIT.md) | Shared rules/skills vs product canon (Accepted, band 127) |
| [`GSV_RUST_DEV.md`](./GSV_RUST_DEV.md) | Rust-first tests/benches/scripts (`cargo xtask`; band **153 ✅**) |
| [`GSV_OMNI_CATALOG.md`](./GSV_OMNI_CATALOG.md) | OmniRouter / Cursor / OpenCode / Grok models (band **157**) |
| [`PRODUCTS.md`](./PRODUCTS.md) | Enrichment-реєстр; discover = `cargo xtask products` |
| [`GSV_MCP_OPENBOT.md`](./GSV_MCP_OPENBOT.md) | MCP `gsv_mcp_openbot` (band 135–**187 ✅**) |
| [`GSV_SETTINGS_TELEGRAM.md`](./GSV_SETTINGS_TELEGRAM.md) | Settings + Godfather + tickets (**166–187 ✅**) |
| [`GSV_SOLO_SQUAD_JAIL.md`](./GSV_SOLO_SQUAD_JAIL.md) | Solo vs squad vs federated jail (band **186 ✅**) |
| [`GSV_MIGRATION.md`](./GSV_MIGRATION.md) | Історична міграція з `docs/vision/` (закрито) |
| [`../GSV_ROLES.md`](../GSV_ROLES.md) | Ролі VDT + канон сесії + ratio gate |
| [`../HANDOFF_NEW_SESSION.md`](../HANDOFF_NEW_SESSION.md) | Операційний зріз |
| [`../NEXT_SESSION_PROMPT.md`](../NEXT_SESSION_PROMPT.md) | Промпт наступної сесії |

PoolAI FM / concept живуть у **`S:/rust/poolAI`**. Цей репо не тримає `docs/catalog/FUNCTION_MANAGEMENT.md`.

## Правила (коротко)

1. **Rust-only** для runtime/API/tools; bins — лише `src/bin/`.
2. Python заборонено (0× `.py`). Java немає.
3. UI — vanilla HTML+CSS+JS; WASM — горизонт (charts уже Rust SVG).
4. Бокси — панелі сервера (`GSV_BOXES.md`).
5. Тести — Rust (`tests/`), не нові Playwright API-специ. Kit scripts — `cargo xtask`, не `.sh`.

## Статус

Лічильники (tests, ratio, vision rev) — у [`MEMORY.md`](../MEMORY.md), не дублювати тут. **Next drain = owner pick** after a warnings-first scan.
