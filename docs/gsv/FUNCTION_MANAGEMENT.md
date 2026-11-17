# FUNCTION_MANAGEMENT.md — GSV (мама, хаб)

**Status:** Canon, agent-drafted 2026-11-17 (owner ratified 2026-11-17) · **Owner:** platinoff
**Компаньйони:** [`GSV_FUNCTION_MANAGEMENT.md`](./GSV_FUNCTION_MANAGEMENT.md) (майстер-матриця) · [`GSV_ECOSYSTEM_SYMBIOSIS.md`](./GSV_ECOSYSTEM_SYMBIOSIS.md) (мама/тато) · [`PRODUCTS.md`](./PRODUCTS.md)
**Шар:** мама — операційний план екосистеми (хаб). poolAI (тато-план, FM) очолює управління; цей файл — функційний реєстр GSV.

## 0. Джерела правди (порядок звірки)

1. Код: `src/` (lib.rs, lib boxes, `crates/mesh-wasm`), bins у `src/bin/`.
2. Контрактні тести: `tests/*.rs` (33 файли).
3. Стани: `docs/gsv/GSV_ECOSYSTEM_SYMBIOSIS.md`, handoff, vision `docs/vision/manifest.json` (rev 516).
4. `data/` докази (fingerprints.jsonl, speed_index.json, rust_diagnostics.json).

Статуси: **І** Implemented (тест/гейт) · **P** Partial (є API, wire неповний) · **S** Declared (реалізовано, немає виділеного контрактного тесту) · **G** Concept-only.

## 1. Функції GSV (звірено з деревом @ 2026-11-17, HEAD `28753cf`)

| # | Функція | Модуль | Статус | Звірка |
|---|---------|--------|--------|--------|
| 1 | HTTP router / AppState / hardened headers (CSP, nosniff, 256 KiB body cap, LAN gate `--allow-lan`) | `server/mod.rs`, `server/serve.rs`, `lib.rs` | І | `tests/gsv_server_contracts.rs`, `tests/gsv_security_contracts.rs` |
| 2 | MCP server (stdio `gsv-mcp` + `/mcp` HTTP/SSE sessions), 60 tools, resources/prompts, `catalog_stale` lockstep | `mcp.rs` | І | `tests/gsv_mcp_contracts.rs` |
| 3 | Galaxy UI — SPA + Rust-rendered cards (`/api/ui/card/<n>`), theme vars, dock/fullscreen, ERROR ring (`ui_errors`) | `ui.rs`, `ui/index.html`, `boxes/ui_errors.rs` | І | `tests/gsv_ui_contracts.rs` |
| 4 | Tickets board (claim/done/error/reclaim, lease 300s, federated presence/claim/done/reclaim) | `boxes/tickets.rs` | І | `tests/gsv_tickets_contracts.rs` |
| 5 | Ranks / Godfather merit ladder (L0–L15, peak, decay, grace-held) | `boxes/ranks.rs` | І | `tests/gsv_ranks_contracts.rs` |
| 6 | Telegram bus + poll loop + Godfather sync (dry-run default) | `boxes/telegram.rs` | І | `tests/gsv_telegram_contracts.rs` |
| 7 | Keep-live aggregation (GSV/Telenetis/llama-rs heartbeat/OmniRoute), fail-open | `boxes/keep_live.rs` | І | `tests/gsv_keep_live_contracts.rs` |
| 8 | Watchdog (self-respawn `target/live`, `version_lag`, debug-newer oneshot apply, sys-history sampler) | `boxes/watchdog.rs`, bin `gsv-watchdog.exe` | І | `tests/gsv_watchdog_contracts.rs` |
| 9 | Vision docs/panel (map, sprint-board, progress, feed, `speed_*`, `rust_diagnostics*` SVG) | `boxes/vision.rs` | І | `tests/gsv_vision_contracts.rs` |
| 10 | OmniRouter (catalog + quota + proxy tap), `bunke-rock`/`rusty-05` providers, route pick | `boxes/omni/{catalog,quota,proxy}.rs` | І | `tests/gsv_omni_contracts.rs` |
| 11 | APK / apk-edge bridge лежки | `boxes/apk.rs` | І | `tests/gsv_apk_contracts.rs` |
| 12 | Edge proxy (VDC §6: per-call token, 20 rps, allowlist) | `boxes/edge.rs` | І | `tests/gsv_edge_contracts.rs` |
| 13 | Grid / discovery / virtual-nodes mirror | `boxes/grid.rs` | І | `tests/gsv_grid_contracts.rs` |
| 14 | mesh WGPU dispatch (`/api/wgpu/*`, ready-gate, shard dispatch) | `boxes/wgpu.rs`, `crates/mesh-wasm` | І | `tests/gsv_wgpu_contracts.rs` |
| 15 | Fingerprints (JSONL, version lockstep) | `boxes/fingerprint.rs` | І | `tests/gsv_fingerprint_contracts.rs` |
| 16 | Hooks (`run mcp bot hook up scenario`) | `boxes/hooks.rs` | І | `tests/gsv_hooks_contracts.rs` |
| 17 | IDE helpers (preview messages, prompt blobs) | `boxes/ide.rs` | І | `tests/gsv_ide_contracts.rs` |
| 18 | MDS report (memory/disk/speed) | `boxes/mds.rs` | І | `tests/gsv_mds_contracts.rs` |
| 19 | Products scan/select (`cargo xtask products`, wiring) | `boxes/products.rs` | І | `tests/gsv_products_contracts.rs`, `tests/gsv_kit_wiring_contracts.rs` |
| 20 | Preview (path confine, highlight_rs) | `boxes/preview.rs` | І | `tests/gsv_preview_contracts.rs` |
| 21 | Ratio box + `gsv-loc-audit --stretch-96` | `boxes/ratio.rs`, bin `gsv_loc_audit` | І | `tests/gsv_ratio_contracts.rs` |
| 22 | Rules-check (git/vision/ratio/registry/sli/bench gates) | `boxes/rules.rs` | S (gate `cargo xtask rules-check`) | inline unit tests у `rules.rs` |
| 23 | Rust-intel (OSV vulns, CVSS v3.1, cadence §C) | `boxes/rust_intel.rs` | S | inline unit tests |
| 24 | SLI catalog (used/unused tracking) | `boxes/sli.rs` | І | `tests/gsv_sli_contracts.rs` |
| 25 | Settings / Godfather token store (redacted) | `boxes/settings.rs` | І | `tests/gsv_settings_contracts.rs` |
| 26 | SW shell-cache (`/sw.js`) | `boxes/sw.rs` | І | `tests/gsv_sw_contracts.rs` |
| 27 | Terminal (HTTP SLI allowlist; `cargo`/`git` only) | `boxes/terminal.rs` | І | `tests/gsv_terminal_contracts.rs` |
| 28 | Telegram `bus`/`inbox` + `kind:presence/claim/done/reclaim` envelope | `boxes/telegram.rs` | І | `tests/gsv_telegram_contracts.rs` |
| 29 | Toolchain probe (`cargo`/`rustc`/MSRV pinned) | `boxes/toolchain.rs` | І | `tests/gsv_toolchain_contracts.rs` |
| 30 | Usage tap (SSE `include_usage` taps, `/api/usage`) | `boxes/usage.rs` | І | `tests/gsv_usage_contracts.rs` |
| 31 | Update (SSE apply→offline, `/api/update/apply`) | `boxes/update.rs` | S | inline (band 208 rewritten test) |
| 32 | sysmon (host sample + sys-history ring) | `boxes/sysmon.rs` | І | `cargo xtask record-speed` / `sys-history` xtask |
| 33 | weight_store (35B shard upload/manifest) | `boxes/weight_store.rs` | I (live use: blk.0..blk.4 g-73…g-136) | dispatch через `#6` (mesh) |
| 34 | adoption / products hygiene / deny (license gate) | `boxes/adopt.rs`, `boxes/hygiene.rs`, `boxes/deny.rs` | S | inline |
| 35 | github/gitkit (queue/issues, `GH#N` hook) | `boxes/github.rs`, `boxes/gitkit.rs` | S | inline |
| 36 | guide / UI card guide | `boxes/guide.rs` | І | `tests/gsv_readme_contracts.rs` |
| 37 | xtask (products/disk/sync/intel/rules-check/record-speed/record-rust/sys-history/live) | `boxes/xtask.rs`, bin `gsv-xtask` | І | `tests/gsv_xtask_contracts.rs` |
| 38 | Stand smoke (feature-gated) + `gsv-http-stand-smoke` | `tests/gsv_stand_smoke_contracts.rs` | І | `--features stand_smoke` → **9/9** |
| 39 | Bins-родини ops: `gsv-server`, `gsv-mcp`, `gsv-watchdog`, `gsv-loc-audit`, `gsv-http-stand-smoke`, `gsv-ui-probe`, + test bins (62+) | `src/bin/` | І | cargo + focused bins |
| 40 | README/docs canon (hero SVGs, install, cards list) | `docs/`, `ui/` | І | `tests/gsv_readme_contracts.rs` |
| 41 | mesh-wasm (hub-side mesh ops + grants) | `crates/mesh-wasm/` | І | `tests/gsv_wgpu_contracts.rs` (hub lane) |

## 2. Гейти (2026-11-17)

- `cargo test` — **exit 0**, ELAPSED=567s (усі suites ok).
- `cargo clippy --all-targets` — **0 warnings**.
- `cargo fmt --all -- --check` — ok.
- `gsv-loc-audit --stretch-96` → **99.61%** (rust 64215 / product 64465).
- `--features stand_smoke` — **9/9**.
- Stand smoke проти живого хаба (2026-11-17) — **87/87** exit=0 (див. poolAI FM evidence row).
- Rust ratio формально ≥ 95% ✅.
- Vision rev (canonical) — **516**; poolAI internal — **499** (mirror rule — owner Q, P6.1 phase 5).

## 3. Дублі / межі

- Ніколи: inference engine (інференс живе тільки в llama-rs / rusty-engine), FM-журнал (живе у poolAI), Telegram bot host (живе в telenetis).
- Свідомо розділено: D1/D2/D5 (див. майстер §3).
- 35B dispatch: GSV `weights_matvec` маршрутизація → engine виконує (rope recompute на практиці через hub probe).

## 4. Owner-decision queue (GSV-side)

1. **RAM/EDR** — причина вічних падінь хаба; потрібен Defender exclusion `target/live/gsv-server.exe` або розвантаження box.
2. **Vision rev mirror rule** (GSV 516 vs poolAI 499).
3. **P4 protocol federation** (FM↔tickets deep-link через hub).
4. Поверхня host-ага (куплена одиниця): раскачування backup/sync кодексу — не в GSV.
5. PoolAI CI runner — поза цим репо; коли з'явиться — GSV не блокується.

## 5. Як агенти користуються

- Будь-яка нова функція GSV → новий рядок §1 з посиланням на тест.
- `cargo xtask sync` — після зміни `docs/gsv/*` синхронізує vision board.
- FM-перелік — вхідна точка для аудиту перед PR; tests — джерело правди, README — реклама.
