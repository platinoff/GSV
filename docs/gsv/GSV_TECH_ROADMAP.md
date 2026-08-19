# GSV TechPreroadMap — Galaxy StarWalker Vision

**TechPreroadMap**: логічний порядок реалізації проєкту GSV → future sprints.

Дата: 2026-08-05 · **Стан:** band 102 **реалізовано** + band 108 (roles/ratio canon) **✅** +
band 109 (Vision sync/migration) **✅** + band 110 (Vision map UI) **✅** + band 111 (Sprint map + doc-preview) **✅** +
band 112 (Vision auto-sync + sprint-queue planning) **✅** + band 113 (Node search + interactive map) **✅** +
band 114 (GSV Sprint-board + progress UI) **✅** · band 115 (GSV migration completion) **✅** ·
band 116 (GSV history charts — speed/rust analytics) **✅** · band 117 (GSV legacy vision deactivation) **✅** ·
band 118 (GSV sprint UI migration — theme + focus map) **✅** ·
band 119 (GSV Galaxy UI full parity — colors + box behaviors) **✅** · band 120 (GSV Ratio 96% stretch) **✅** ·
band 121 (GSV OmniRouter box parity) **✅** · band 125 (GSV Vision/UI polish — a11y/error/offline/stand contracts) **✅** ·
band 126 (GSV stand smoke + ops canon) **✅** ·
band 127 (GSV VDT kit — точка входу) **✅** ·
**band 128** (kit ops + grouped Galaxy UI) **✅** ·
**band 129** (canon port 9999 + dashboard card registry) **✅** ·
**band 130** (chrome shell: real wires + Rust RSS ticker) **✅** ·
**band 131** (Rust shell CSS + layout nav HTML) **✅** ·
**band 132** (Rust header chrome HTML + node-search fragment) **✅** ·
**band 133** (localhost security: bind + CSRF + terminal + data allowlist) **✅** ·
**band 134** (HTTP response hardening: CSP / nosniff / no-store + POST body cap) **✅** ·
**band 135** (gsv_mcp_openbot — MCP for OpenCode / Cursor / Grok Bot) **✅** ·
**band 136** (MCP Galaxy UI + remaining read tools) **✅** ·
**band 137** (MCP vision completeness) **✅** ·
**band 138** (MCP resources + prompts) **✅** ·
**band 139** (MCP logging + completions) **✅** ·
**band 140** (MCP resource subscribe + logging notifications) **✅** ·
**band 141** (MCP HTTP SSE / streamable notifications) **✅** ·
**band 142** (MCP HTTP sessions / `Mcp-Session-Id`) **✅** ·
**band 143** (Galaxy chrome + type/chart scale) **✅** ·
**band 144** (always-on live copy + apply) **✅** ·
**band 145** (VDT products picker + open folder + scan) **✅** ·
**band 146** (version bump + fingerprints) **✅** ·
**band 147** (README-level polish leftovers) **✅** ·
**band 148** (Service Worker shell cache) **✅** ·
**band 149** (omniroute PRODUCTS.md + semver minor = band) **✅** ·
**band 150** (live watchdog) **✅** ·
**band 151** (MCP catch-up) **✅** ·
**band 152** (MCP products select) **✅** ·
**band 153** (rust-first cargo xtask) **✅** ·
**band 154** (watchdog ops card + fingerprint model) **✅** ·
**band 155** (session token usage — MCP + OmniRoute + sync) **✅** ·
**band 156** (streaming usage + VDT git + owner tunnel) **✅** ·
**band 157** (OmniRouter catalog + quota timers) **✅** ·
**band 158** (live MCP stdio + sync check) **✅** ·
**band 159** (Cursor HTTP MCP + session SSE hold) **✅** ·
**band 160** (GSV sandbox MCP · no User leak into PoolAI) **✅** ·
**band 161** (vision lockstep PH-S2249 + disk MiB + `--clean` keep-live) **✅** ·
**band 162** (live crate/version lockstep) **✅** ·
**band 163** (vision queue lockstep + bump auto-advance) **✅** ·
**band 164** (Cursor 3.16.29 kit lockstep — rules / tools / MCP / sync) **✅** ·
**band 165** (watchdog live copy + lockstep observability) **✅** ·
**band 166** (GSV settings + Godfather secret store) **✅** ·
**band 167** (Telegram Godfather channel bind) **✅** ·
**band 168** (ticket board + MCP claim) **✅** ·
**band 169** (Telegram bus between MCP bots) **✅** ·
**Спринти:** `PH-S1659…S1668` (FM §5.12 §5.83 ✅) · `PH-S1719…S1728` (FM §5.12 §5.89 ✅) ·
`PH-S1729…S1738` (FM §5.12 §5.90 ✅) · `PH-S1739…S1748` (FM §5.12 §5.91 ✅) ·
`PH-S1749…S1758` (FM §5.12 §5.92 ✅) · `PH-S1759…S1768` (FM §5.12 §5.93 ✅) ·
`PH-S1769…S1778` (FM §5.12 §5.94 ✅) · `PH-S1789…S1798` (FM §5.12 §5.96 ✅) ·
`PH-S1799…S1808` (FM §5.12 §5.97 ✅) · `PH-S1809…S1818` (FM §5.12 §5.98 ✅) ·
`PH-S1819…S1828` (FM §5.12 §5.99 ✅) · `PH-S1829…S1838` (FM §5.12 §5.100 ✅) ·
`PH-S1839…S1848` (FM §5.12 §5.101 ✅) · `PH-S1849…S1855` (FM §5.12 §5.102 ✅) ·
`PH-S1889…S1898` (FM §5.12 §5.106 ✅) · `PH-S1899…S1908` (FM §5.12 §5.107 ✅).

## Логічний порядок (залежності)

```
docs/architecture (✅ ця сесія)
  → server scaffold (bin + static UI)
      → Tracker (джерела даних workflow)
      → SLI console (каталог команд зі скриптів)
      → Toolchain (інвентар тулів)
      → IDE (opencode + cursor сесії)
      → Update/offline/resync (ключова механіка)
      → Box preview (Rust-синтаксис-кольори)
      → SLI terminal (AI → команди)
      → Tests/bench hooks (без перекомпіляції)
  → band close (docs canon, parity, vision-sync, ratio hold)
  → [band 108] roles/ratio canon (GSV як poolAI-grade проєкт):
      GSV_ROLES → gsv-loc-audit → ratio contracts → Ratio box/UI
      → memory mark → HANDOFF/NEXT → FM §5.89 → poolAI parity → band close
```

## Спринти (band 102)

| Sprint | Фокус | Acceptance (ключ) |
|--------|-------|-------------------|
| **PH-S1659** | GSV docs/architecture + Cargo scaffold | `GSV/docs/gsv/` канон; `GSV/Cargo.toml`; empty server builds |
| **PH-S1660** | gsv-server bin scaffold | `gsv_server.rs`; `GET /` → UI; `GET /api/health` |
| **PH-S1661** | Tracker box | `tracker/`; `GET /api/tracker`; `gsv_tracker.json`; параметри останнього workflow |
| **PH-S1662** | SLI console box | `sli/`; `GET /api/sli`; каталог з `bin/`+`scripts/`+`src/bin/`; використані команди |
| **PH-S1663** | Toolchain box | `toolchain/`; `GET /api/toolchain`; інвентар (rustc 1.92, clippy, MSYS2, …) |
| **PH-S1664** | IDE box | `ide/`; `GET /api/ide/sessions`; `POST /api/ide/select`; opencode + cursor чати |
| **PH-S1665** | Update box | `update/`; `/api/update`; SSE `update_available`; «Update» замість reload |
| **PH-S1666** | Box preview + SLI terminal | `preview/` Rust-кольори; `POST /api/terminal` (whitelist SLI) |
| **PH-S1667** | Tests/bench hooks (без перекомпіляції) | `hooks/`; `/api/hooks/tests`; `/api/hooks/bench`; read `target/` без build |
| **PH-S1668** | Band close | offline-стійкість + metrics resync; Rust tests; docs canon; vision parity; ratio hold |

## Спринти (band 108) — roles/ratio canon (poolAI дисципліна)

| Sprint | Фокус | Acceptance (ключ) |
|--------|-------|-------------------|
| **PH-S1719** | GSV roles canon | `GSV/docs/GSV_ROLES.md`; README pointer |
| **PH-S1720** | `gsv-loc-audit` bin | `GSV/src/bin/gsv_loc_audit.rs`; `--min-ratio/--advisory`; `GSV/data/rust_ratio.json` |
| **PH-S1721** | Ratio contracts | `tests/gsv_ratio_contracts.rs` (7) |
| **PH-S1722** | Ratio box + wire | `boxes/ratio.rs`; `GET /api/ratio`; UI Ratio card |
| **PH-S1723** | GSV memory mark | `GSV/docs/MEMORY.md` + `GSV/docs/README.md` |
| **PH-S1724** | GSV HANDOFF/NEXT | `GSV/docs/HANDOFF_NEW_SESSION.md` + `NEXT_SESSION_PROMPT.md` |
| **PH-S1725** | FM band 108 + roadmap | FM §5.12 §5.89; цей файл |
| **PH-S1726** | poolAI docs parity | GSV rows у poolAI docs |
| **PH-S1727** | poolAI HANDOFF + NEXT | band 108 ✅ · horizon band 109 |
| **PH-S1728** | Band close | ratio hold (≥95%); fmt/clippy/test; docs canon; vision-sync rev 458 |

## Спринти (band 109) — Vision box (poolAI vision canon mirror)

| Sprint | Фокус | Acceptance (ключ) |
|--------|-------|-------------------|
| **PH-S1729** | Vision box scaffold | `GSV/src/boxes/vision.rs` (manifest/feed serde) + `Cargo.toml` bin |
| **PH-S1730** | Manifest wire | `gsv_manifest.json`; `GET /api/vision/manifest` |
| **PH-S1731** | Feed wire | `gsv_feed.json`; `GET /api/vision/feed` |
| **PH-S1732** | `gsv-vision-sync` bin | mirror + `--check` drift gate |
| **PH-S1733** | Vision UI card | summary + sprint ticker |
| **PH-S1734** | Vision contracts | `tests/gsv_vision_contracts.rs` (7) |
| **PH-S1735** | GSV vision docs | `VISION.md` + `GSV_MIGRATION.md` rows ✅ + MEMORY mark |
| **PH-S1736** | poolAI vision parity | `GSV/docs/vision/README.md` + cross-check |
| **PH-S1737** | Ratio hold advisory | `gsv-loc-audit --min-ratio 0.95 --advisory` |
| **PH-S1738** | Band close | ratio hold (≥95%); fmt/clippy/test; docs canon; vision-sync rev 459 |

## Спринти (band 110) — Vision map UI

| Sprint | Фокус | Acceptance (ключ) |
|--------|-------|-------------------|
| **PH-S1739** | Vision map wire | `map_report`/`wire_map`; `GET /api/vision/map` (layers L0..L5 z-sorted + edge kinds) |
| **PH-S1740** | vision.svg port | `GSV/ui/vision.svg` + `GET /assets/vision.svg` (audit Ignored, ratio-neutral) |
| **PH-S1741** | Vision Map UI card | layer chips + edge kinds + svg link у `ui/index.html` |
| **PH-S1742** | Vision map contracts | `tests/gsv_vision_contracts.rs` (10) |
| **PH-S1743** | Feed status filter | `GET /api/vision/feed?status=closed\|open\|all` |
| **PH-S1744** | GSV vision docs | `VISION.md` map/feed-filter/svg; `GSV_MIGRATION.md` rows ✅; MEMORY band 110 |
| **PH-S1745** | poolAI vision parity | `GSV/docs/vision/README.md` band 110; roadmap band 110 |
| **PH-S1746** | Ratio hold advisory | `gsv-loc-audit --min-ratio 0.95 --advisory` |
| **PH-S1747** | vision-sync close | `gsv-vision-sync` refresh + poolAI vision rev **461** |
| **PH-S1748** | Band close | ratio hold (≥95%); fmt/clippy/test; docs canon; push |

## Спринти (band 111) — Sprint map + doc-preview (Vision UI логіка)

| Sprint | Фокус | Acceptance (ключ) |
|--------|-------|-------------------|
| **PH-S1749** | Sprint-map wire | `sprint_map_report`/`wire_sprint_map`; `GET /api/vision/sprint-map` (sprint-scope/queue/session-tracks links + modules + kinds) |
| **PH-S1750** | Doc-preview wire | `doc_preview`/`wire_doc_preview`; `GET /api/vision/doc-preview?id=` (node + 1-hop neighbors) |
| **PH-S1751** | Sprint-map contracts | `tests/gsv_vision_contracts.rs` (12) |
| **PH-S1752** | Doc-preview contracts | `tests/gsv_vision_contracts.rs` (**14**) |
| **PH-S1753** | Sprint Map UI card | modules/kinds/links у `ui/index.html` |
| **PH-S1754** | Doc Preview UI card | node id input + out/in links + sections у `ui/index.html` |
| **PH-S1755** | GSV vision docs | `VISION.md` sprint-map/doc-preview; MEMORY band 111; HANDOFF/NEXT band 111 |
| **PH-S1756** | poolAI vision parity | `GSV_MIGRATION.md` row 21 ✅; `GSV/docs/vision/README.md`; roadmap band 111 |
| **PH-S1757** | Ratio hold advisory | `gsv-loc-audit --min-ratio 0.95 --advisory`; poolAI parity hold |
| **PH-S1758** | Band close | ratio hold (≥95%); fmt/clippy/test; docs canon; vision-sync; push |

## Спринти (band 112) — Vision auto-sync + sprint-queue planning

| Sprint | Фокус | Acceptance (ключ) |
|--------|-------|-------------------|
| **PH-S1759** | Extensions mirror | `Extensions` struct + read/save/load/source; `gsv_extensions.json` snapshot; `wire_extensions` → `GET /api/vision/extensions`; `sync()`/`collect_drift`/bin include extensions |
| **PH-S1760** | Vision auto-sync wire | `wire_sync` → `GET /api/vision/sync` (re-mirror + drift gate) |
| **PH-S1761** | Sprint-queue planning wire | `SprintQueueReport`/`wire_sprint_queue` → `GET /api/vision/sprint-queue` (entries ∪ active) |
| **PH-S1762** | Extensions contracts | `tests/gsv_vision_contracts.rs` extensions (17) |
| **PH-S1763** | Sprint-queue contracts | sync + sprint-queue endpoints + real-workspace report (**19**) |
| **PH-S1764** | Vision Sync + Sprint Queue UI cards | Resync button + drift status; next/active/open + planned у `ui/index.html` |
| **PH-S1765** | GSV vision docs | `VISION.md` sync/extensions/sprint-queue; MEMORY band 112; HANDOFF/NEXT band 112 |
| **PH-S1766** | poolAI vision parity | `GSV_MIGRATION.md` rows ✅; `GSV/docs/vision/README.md`; roadmap band 112 |
| **PH-S1767** | Ratio hold advisory | `gsv-loc-audit --min-ratio 0.95 --advisory` (95.56%) |
| **PH-S1768** | Band close | ratio hold (≥95%); fmt/clippy/test (118); docs canon; vision-sync rev 463; push |

## Спринти (band 113) — Galaxy UI: node search + interactive map

| Sprint | Фокус | Acceptance (ключ) |
|--------|-------|-------------------|
| **PH-S1769** | Node search wire | `NodeSearchReport`/`node_search_report`/`wire_node_search` → `GET /api/vision/node-search?q=&layer=` (case-insensitive id/label/path/sections, top-N 25, layer-z-sorted, links_out/in tallies) |
| **PH-S1770** | Node search contracts | `tests/gsv_vision_contracts.rs` (real-workspace + layer filter + no-match empty/cap, **22**) |
| **PH-S1771** | Node-search endpoint contract | `tests/gsv_server_contracts.rs` (`/api/vision/node-search?q=` ok + results; empty q → ok true, **19**) |
| **PH-S1772** | Inline SVG map card | Vision Map card рендерить `assets/vision.svg` inline (`<img>`) + chips/kinds |
| **PH-S1773** | Layer filter + search UX | клікабельні layer chips (active filter) + node-search input + results → doc-preview deep-link у `ui/index.html` |
| **PH-S1774** | GSV vision docs | `VISION.md` node-search/map UX; MEMORY band 113; HANDOFF/NEXT band 113 |
| **PH-S1775** | poolAI vision parity | `GSV_MIGRATION.md` rows ✅; `GSV/docs/vision/README.md`; цей файл band 113 |
| **PH-S1776** | Ratio hold advisory | `gsv-loc-audit --min-ratio 0.95 --advisory` (≥95%) |
| **PH-S1777** | vision-sync close | `gsv-vision-sync` refresh + poolAI vision rev **465** |
| **PH-S1778** | Band close | ratio hold (≥95%); fmt/clippy/test (122); docs canon; vision-sync; push |

## Спринти (band 114) — GSV Sprint-board + progress UI

| Sprint | Фокус | Acceptance (ключ) |
|--------|-------|-------------------|
| **PH-S1779** | Sprint-board wire | `SprintBoardReport`/`SprintBoardColumn`/`sprint_board_report`/`wire_sprint_board` → `GET /api/vision/sprint-board` (open/closed/planned columns + counts + `progress_pct` = closed/total) |
| **PH-S1780** | Progress wire | `SprintProgressReport`/`SprintLayerProgress`/`sprint_progress_report`/`wire_sprint_progress` → `GET /api/vision/sprint-progress` (status counts + per-layer `node_count`/`linked_count`, z-ascending) |
| **PH-S1781** | Sprint-board contracts | `tests/gsv_vision_contracts.rs` (grouping, pct formula, column order, active-in-open, uniqueness, **30**) |
| **PH-S1782** | Progress contracts | sprint-progress contracts (layers match manifest, statuses sum, z-ordered, linked reflect queue, **38**) |
| **PH-S1783** | Endpoint contracts | `tests/gsv_server_contracts.rs` sprint-board + sprint-progress (ok + status sums + columns/layers shape, **21**) |
| **PH-S1784** | Sprint Board card | Sprint Board UI card у `ui/index.html`: progress bar + open/closed/planned колонки-details (`bar()` helper) |
| **PH-S1785** | Sprint Progress card | Sprint Progress UI card: progress bar + per-layer таблиця nodes/linked |
| **PH-S1786** | GSV vision docs | `VISION.md` sprint-board/sprint-progress API + band 114 section; MEMORY band 114; HANDOFF/NEXT band 114 |
| **PH-S1787** | poolAI vision parity | `GSV_MIGRATION.md` rows ✅; `GSV/docs/vision/README.md`; цей файл band 114; FM §5.95 |
| **PH-S1788** | Band close | ratio hold (**95.02%**); fmt/clippy/test (140); docs canon; vision-sync rev 467; push |

## Спринти (band 115) — GSV migration completion (legacy vision supersession)

| Sprint | Фокус | Acceptance (ключ) |
|--------|-------|-------------------|
| **PH-S1789** | Legacy parity audit | `GSV/docs/LEGACY_PARITY.md`: кожна legacy-панель (`GSV/docs/vision/index.html`) → GSV endpoint+card / superseded / out-of-scope |
| **PH-S1790** | Speeds wire | `SpeedIndexReport`/`SpeedIndexLatest`/`read_speed_index`/`save`/`load`/`source_speed_index`/`wire_speed_index` → `GET /api/vision/speeds` (empty-tolerant) |
| **PH-S1791** | Rust diagnostics wire | `RustDiagnosticsReport`/`RustDiagLatest`/`read_rust_diagnostics`/`save`/`load`/`wire_rust_diagnostics` → `GET /api/vision/rust-diagnostics` (empty-tolerant) |
| **PH-S1792** | Contracts | `gsv_vision_contracts.rs` (real-workspace speed_index/rust_diagnostics + wire shapes) + `gsv_server_contracts.rs` (`/speeds` + `/rust-diagnostics` 200/ok/shape) |
| **PH-S1793** | Speeds + Rust cards | Speed Index card + Rust Diagnostics card у `ui/index.html` (present/empty states, latest metrics, top clippy codes) |
| **PH-S1794** | GSV_MIGRATION rows + roadmap | `GSV_MIGRATION.md` rows ✅ (speed_index/rust_diagnostics/vision.js.css superseded); `GSV_TECH_ROADMAP.md` band 115 |
| **PH-S1795** | GSV vision docs canon | `VISION.md` +band 115 endpoints; MEMORY band 115; HANDOFF/NEXT band 115 |
| **PH-S1796** | poolAI vision parity | FM §5.12 §5.96; HANDOFF/NEXT band 115; `GSV/docs/vision/` canon |
| **PH-S1797** | Ratio hold advisory | `gsv-loc-audit` ≥95% (**95.04%**); legacy JS не переносимо (superseded) |
| **PH-S1798** | Band close | ratio hold; fmt/clippy/test (150); docs canon; vision-sync rev 468; push |

## Спринти (band 116) — GSV history charts (speed/rust analytics)

| Sprint | Фокус | Acceptance (ключ) |
|--------|-------|-------------------|
| **PH-S1799** | Scope + queue | FM §5.97 band 116 (PH-S1799…S1808) + manifest sync |
| **PH-S1800** | Speeds history wire | `SpeedTestCiRecord`/`SpeedBenchRecord` + `test_ci_history`/`bench_history` у `SpeedIndexReport`; `read_speed_index` carry arrays (source fallback unchanged) |
| **PH-S1801** | Rust diagnostics history wire | `RustDiagRecord` + `history` у `RustDiagnosticsReport`; `read_rust_diagnostics` carry history |
| **PH-S1802** | Contracts | vision tests 20 → **23**: typed parse, SVG bars + empty state (`data_dir_of` helper) |
| **PH-S1803** | Speed history chart UI | `speed_index_chart_svg` → `GET /api/vision/speeds.svg` (Rust-rendered SVG: test-CI wall bars green ok / red fail, ≤24 runs, footer latest bench) + `<img id="i-speed-chart">` |
| **PH-S1804** | Rust history chart UI | `rust_diagnostics_chart_svg` → `GET /api/vision/rust-diagnostics.svg` (warnings orange + errors red grouped bars, command footer) + `<img id="i-rust-chart">` |
| **PH-S1805** | Stand smoke + wasm defer | stand smoke: обидва SVG 200 `image/svg+xml`; `poolai-ui-wasm` defer row у `GSV_MIGRATION.md` + roadmap |
| **PH-S1806** | GSV vision docs canon | `VISION.md` +band 116 section/endpoints; MEMORY band 116; HANDOFF/NEXT band 116 |
| **PH-S1807** | poolAI vision parity | `GSV/docs/vision/README.md`; FM §5.12 §5.97; цей файл band 116; poolAI HANDOFF/NEXT band 116 |
| **PH-S1808** | Band close | ratio hold (**95.26%**); fmt/clippy/test (153); docs canon; vision-sync rev 469; push |

## Спринти (band 117) — GSV legacy vision deactivation

| Sprint | Фокус | Acceptance (ключ) |
|--------|-------|-------------------|
| **PH-S1809** | Scope + queue | FM §5.98 band 117 (PH-S1809…S1818) + manifest sync |
| **PH-S1810** | Legacy index deactivation | `GSV/docs/vision/index.html` → minimal GSV pointer page (no `vision.js`/`vision.css` refs) |
| **PH-S1811** | Legacy JS/CSS deactivation | DEACTIVATED banner у `vision.js`/`vision.css`; deactivation note у `GSV/docs/vision/README.md` |
| **PH-S1812** | Live link retarget | `poolai-vision-sync` feed links → `http://127.0.0.1:8891/#b-sprint-board`; GSV `vision.rs` sample links; RUN_LOCAL/GSV_SERVER/gsv README/SPEED_INDEX/RUST_DIAGNOSTICS → GSV |
| **PH-S1813** | Legacy test retirement | `poolai_vision_sync.rs` unit ×4 + `galaxy_horizon_s1011/s1019/s1039` → deactivated pointer state; e2e pointer assertions |
| **PH-S1814** | GSV parity docs | `LEGACY_PARITY.md` + `GSV_MIGRATION.md` band 117 |
| **PH-S1815** | GSV vision docs canon | `VISION.md`/`MEMORY.md`/HANDOFF/NEXT band 117 |
| **PH-S1816** | poolAI vision parity | `GSV/docs/vision/README.md`; FM §5.12 §5.98; цей файл band 117; poolAI HANDOFF/NEXT band 117 |
| **PH-S1817** | Ratio + rev prep | ratio hold advisory (**95.26%**); vision-sync rev 470 (poolai + gsv + --check) |
| **PH-S1818** | Band close | ratio hold; fmt/clippy/test (poolAI test-ci + GSV 153); docs canon; vision-sync rev 470; push |

## Спринти (band 118) — GSV sprint UI migration (theme + focus map)

| Sprint | Фокус | Acceptance (ключ) |
|--------|-------|-------------------|
| **PH-S1819** | Scope + queue | FM §5.99 band 118 (PH-S1819…S1828) + §5.12 header (master horizon) |
| **PH-S1820** | Sprint theme wire | `SprintThemeReport`/`SprintPillTheme`/`SprintChipTheme`/`SprintQueueStateTheme`/`SprintLayerColor`/`SprintEdgeKindColor` + `sprint_theme_report`/`wire_sprint_theme` → `GET /api/vision/sprint-theme` (sprint `#a78bfa`/next `#c4b5fd`, pill/chip/queue colors, layers L0–L5, edge kinds) |
| **PH-S1821** | Sprint focus SVG | `sprint_token_matches`/`path_matches_glob`/`nodes_for_sprint` + `sprint_focus_svg` → `GET /api/vision/sprint-focus.svg?sprint=` (sprint-dim: in-scope accent, out-of-scope opacity 0.22/text 0.28, edges tinted; default active sprint; empty-state) |
| **PH-S1822** | Contracts | `gsv_vision_contracts` (theme real-workspace + wire shapes + focus svg highlight/dim/empty) + `gsv_server_contracts` (theme + focus endpoints) |
| **PH-S1823** | UI sprint colors | `GSV/ui/index.html`: `--sprint*` CSS-змінні + sprint-pill/queue chips у Sprint Queue/Board cards; Sprint Focus card (`<img id="i-sprint-focus">`) + `loadSprintTheme`/`loadSprintFocus` |
| **PH-S1824** | GSV vision docs canon | `VISION.md` +band 118 (theme + focus endpoints/section); MEMORY band 118; GSV HANDOFF/NEXT band 118 |
| **PH-S1825** | poolAI vision parity | `GSV/docs/vision/README.md`; FM §5.12 §5.99; цей файл band 118 |
| **PH-S1826** | Ratio hold advisory | `gsv-loc-audit --min-ratio 0.95 --advisory` ≥95% (**95.35%**) + poolAI ratio96 advisory hold |
| **PH-S1827** | poolai-vision-sync close | `poolai-vision-sync` rev **471** (band 118); `--check` ok; sprint-queue/feed updated |
| **PH-S1828** | Band close | ratio hold; fmt/clippy/test (**163**); docs canon; vision-sync rev 471; push |

## Спринти (band 119) — Galaxy UI full parity: colors + box behaviors

| Sprint | Фокус | Acceptance (ключ) |
|--------|-------|-------------------|
| **PH-S1829** | Scope + queue | FM §5.12 band 119 (PH-S1829…S1838) + manifest sync; legacy parity scope: full `vision.css` `:root` palette + header chrome (ticker, GPU modes, power-menu buttons) + panel dock/collapse/fullscreen + starfield/galaxy backdrop |
| **PH-S1830** | `GalaxyPalette` wire | `GalaxyPalette` struct (bg-deep/bg/panel/panel-solid/border/border-bright/text/muted/accent/accent-2/glow/sidebar-w/L0–L5+dim/edge-*/ext-*/sprint/bg-tone/galaxy-bg-opacity) + `wire_palette` → `GET /api/vision/palette` (exact legacy `:root` values) |
| **PH-S1831** | Starfield SVG generator | `starfield_svg(mode)` in `boxes/vision.rs` → `GET /api/vision/starfield.svg?mode=eco\|fx\|ms` (Rust-rendered deterministic stars; eco sparse/static, fx dense+glow, ms medium; `image/svg+xml`) |
| **PH-S1832** | Galaxy backdrop SVG | `galaxy_svg()` → `GET /api/vision/galaxy.svg` (radial nebula gradient + galaxy arms using `--glow`/`--accent-2`/`--bg-tone`; `image/svg+xml`) |
| **PH-S1833** | Header chrome + ticker | `ui/index.html`: RSS ticker (label + viewport + track from `/api/vision/feed` items), GPU mode buttons (Eco/FX/Ms cycle via `btn-eco`), Auto toggle, Reload, Power menu buttons (shutdown/reboot/soft → `/api/vision/sync`), meta-rev + meta-trail (git HEAD + next sprint) |
| **PH-S1834** | Panel dock + Esc-fullscreen | collapse → panel dock row (restore on click), fullscreen via `⛶` + `Esc` exits fullscreen; `body.panel-fs-active` z-index layering; starfield/galaxy backdrop `<img>` (ratio-safe `.svg`) |
| **PH-S1835** | Contracts | `gsv_vision_contracts.rs`: palette values == legacy `:root`, starfield/galaxy svg shape (width/height, mode variance), starfield empty-state; `gsv_server_contracts.rs`: `/api/vision/palette` + `/api/vision/starfield.svg?mode=` + `/api/vision/galaxy.svg` 200 + `image/svg+xml` |
| **PH-S1836** | GSV docs canon | `VISION.md` +band 119 (palette/starfield/galaxy/header UI); MEMORY band 119; HANDOFF/NEXT band 119 |
| **PH-S1837** | Ratio hold + rev prep | `gsv-loc-audit` ≥95% (**95.18%** ✅; UI non-rust delta compensated by Rust tests + compact JS); vision-sync rev **472** (poolai + gsv + `--check`) |
| **PH-S1838** | Band close | ratio hold; fmt/clippy/test; docs canon; vision-sync rev 472; push |

## Спринти (band 120) — Ratio 96% stretch (server-rendered UI fragments + compact UI) ✅

| Sprint | Фокус | Acceptance (ключ) |
|--------|-------|-------------------|
| **PH-S1839** | Scope + queue | FM §5.101 band 120 (PH-S1839…S1848) + §5.12 header (master horizon); roadmap band 120 rows — **✅** |
| **PH-S1840** | `--stretch-96` advisory | `gsv-loc-audit --stretch-96` — 96% advisory threshold check (report + exit code); `boxes/ratio.rs` stretch support; unit tests — **✅** |
| **PH-S1841** | Ratio96 contracts | `gsv_ratio_contracts`: stretch-96 advisory + `rust_ratio.json` stretch fields + `/api/ratio` shape — **✅** |
| **PH-S1842** | UI fragment module | `GSV/src/boxes/ui.rs`: `esc`/`tab`/`bar` + per-card HTML renderers (vision cards) + unit tests — **✅** |
| **PH-S1843** | Render endpoint | `GET /api/ui/card/:name` — Rust-rendered card body HTML; dispatch у `server/mod.rs` — **✅** |
| **PH-S1844** | UI thin glue | `ui/index.html`: `render*` fns → `getText` injection; compact CSS/JS (thin glue, ratio-safe) — **✅** |
| **PH-S1845** | Contracts | `gsv_ui_contracts.rs` (render HTML markers) + `gsv_server_contracts.rs` (card endpoints) — **✅** |
| **PH-S1846** | Ratio 96% measurement | `gsv-loc-audit --stretch-96` green **≥96%** (**96.51%** ✅); compact/tests adjust to hold — **✅** |
| **PH-S1847** | GSV docs canon | MEMORY band 120; HANDOFF/NEXT band 120; VISION.md; `GSV_TECH_ROADMAP.md` band 120 — **✅** |
| **PH-S1848** | Band close | ratio **≥96%**; fmt/clippy/test; docs canon; vision-sync rev bump; push — **✅** |

## Спринти (band 121) — OmniRouter box parity ✅

| Sprint | Фокус | Acceptance (ключ) |
|--------|-------|-------------------|
| **PH-S1849** | Scope + queue | FM §5.102 band 121 (PH-S1849…S1855) + §5.12 header (master horizon); roadmap band 121 rows — **✅** |
| **PH-S1850** | `render_omni` | `boxes/ui.rs`: `render_omni` (summary/routing + recommended + providers table + models table + `format_number` grouping) + `render_card`/`CARD_NAMES` 13 + unit tests — **✅** |
| **PH-S1851** | Wire omni card | `server/mod.rs` `api_ui_card`: `"omni"` → `boxes::omni::wire`; UI thin glue: `renderOmni` JS removed, `rustCards` 13, `resync()` url drop — **✅** |
| **PH-S1852** | Contracts | `gsv_ui_contracts` (`ui_card_omni_renders_summary_providers_models`) + `gsv_server_contracts` (omni card endpoint 200) — **✅** |
| **PH-S1853** | Ratio hold + tests | `gsv-loc-audit --stretch-96` green **≥96%** (**96.73%** ✅); full GSV tests **207**; clippy 0 — **✅** |
| **PH-S1854** | GSV docs canon | MEMORY band 121; HANDOFF/NEXT band 121; VISION.md omni card section; `GSV_TECH_ROADMAP.md` band 121 — **✅** |
| **PH-S1855** | Band close | ratio **≥96%**; fmt/clippy/test; docs canon; vision-sync rev bump; push — **✅** |

## Спринти (band 125) — Vision/UI polish (a11y/error/offline/stand contracts) ✅

| Sprint | Фокус | Acceptance (ключ) |
|--------|-------|-------------------|
| **PH-S1889** | Scope + queue | FM §5.106 band 125 (PH-S1889…S1898) + §5.12 header (master horizon); roadmap band 125 rows — **✅** |
| **PH-S1890** | Card error/empty states | `boxes/ui.rs`: 13 renderers — consistent empty-state + error-state HTML markers, no panic — **✅** |
| **PH-S1891** | Card stand contracts | `gsv_ui_contracts`: stand contracts for all 13 renderers (empty + error markers) — **✅** |
| **PH-S1892** | Server JSON error shape | `server/mod.rs`: consistent 4xx + JSON error shape (`ok:false` + error) across endpoints — **✅** |
| **PH-S1893** | Server stand contracts | `gsv_server_contracts`: endpoint error/empty shape contracts — **✅** |
| **PH-S1894** | UI a11y markers | `ui/index.html`: aria/role/alt + focus-visible for all cards (axe markers) — **✅** |
| **PH-S1895** | Offline-stable cards | `ui/index.html`: last-good render kept on fetch fail, error badges, no blank cards — **✅** |
| **PH-S1896** | Vision wire polish | `boxes/vision.rs`: consistent `ok`/`error` + empty-tolerant wire shapes — **✅** |
| **PH-S1897** | Ratio hold + tests | `gsv-loc-audit --stretch-96` green **≥96%**; full GSV tests green; clippy 0 — **✅** |
| **PH-S1898** | GSV docs canon + band close | MEMORY/HANDOFF/NEXT/VISION/roadmap band 125; FM ✅; vision-sync rev bump; push — **✅** |

## Спринти (band 126) — GSV stand smoke + ops canon ✅

| Sprint | Фокус | Acceptance (ключ) |
|--------|-------|-------------------|
| **PH-S1899** | Scope + queue | FM §5.107 band 126 (PH-S1899…S1908) + §5.12 header (master horizon) — **✅** |
| **PH-S1900** | Stand smoke bin | `src/bin/gsv_http_stand_smoke.rs` (мірор poolAI `poolai-http-stand-smoke`) + `Cargo.toml` `[[bin]]` — **✅** |
| **PH-S1901** | Stand smoke checks | 48 live checks (core boxes + vision* ok-gate + SVG status + 20 ui cards non-empty html) — **✅** |
| **PH-S1902** | Stand smoke contracts | `gsv_stand_smoke_contracts.rs`: vision ok-gate + struct-wire JSON + status + cards + report shape — **✅** |
| **PH-S1903** | GSV docs canon | GSV_SERVER.md (stand smoke section), GSV_BOXES.md (row), README (tests/structure/endpoints/status), roadmap band 126 — **✅** |
| **PH-S1904** | Ratio hold + tests | `gsv-loc-audit --stretch-96` **≥96%** (96.87%); full GSV tests green (230); clippy 0 — **✅** |
| **PH-S1905** | GSV vision docs canon | VISION.md / MEMORY / GSV HANDOFF / NEXT_SESSION band 126 — **✅** |
| **PH-S1906** | poolAI vision parity | FM §5.12 §5.107 + GSV vision README + poolAI HANDOFF/NEXT — **✅** |
| **PH-S1907** | Vision-sync close | `poolai-vision-sync` rev bump band 126, `--check` ok — **✅** |
| **PH-S1908** | Band close | Speeds/Rust panel; один commit; `git push` + самарі; gsv-server restart — **✅** |

## Спринти (band 127) — GSV VDT kit (точка входу) ✅

Канон: [`GSV_VDT_KIT.md`](./GSV_VDT_KIT.md) (Status=Accepted). Owner 2026-08-17: GSV тримає спільні
rules/skills; продукти (PoolAI, GSV-server, далі — нові Rust-репо) лишають лише
продуктовий шар. **Відкривати Cursor на `S:\rust\GSV`** або `gsv.code-workspace`.
Реєстр: [`PRODUCTS.md`](./PRODUCTS.md).

| Sprint | Фокус | Acceptance (ключ) |
|--------|-------|-------------------|
| **PH-S1909** | Canon Accepted | `GSV_VDT_KIT.md` Status=Accepted; HANDOFF/NEXT/README pointers — **✅** |
| **PH-S1910** | Abracadabra host | `.agents/skills/abracadabra/` роутить **продукт** (poolai \| gsv); вікно ≠ продукт — **✅** |
| **PH-S1911** | Generic skills | copy marketplace skills з PoolAI `.agents/skills/` (без `poolai-documentation`) — **✅** |
| **PH-S1912** | Generic rules | `.cursor/rules/` VDT: session, roles, MSYS2, git, rust-generic, cursor baseline — **✅** |
| **PH-S1913** | Client mirrors | `.cursor/skills/` + `.opencode/skills/` identical (Windows copy) — **✅** |
| **PH-S1914** | Product registry | `docs/gsv/PRODUCTS.md` (root, handoff, test cmd, ratio per product) — **✅** |
| **PH-S1915** | Workspace | `gsv.code-workspace` (GSV перший + PoolAI) — **✅** |
| **PH-S1916** | PoolAI thin | PoolAI: product-only rules/skills; pointer «kit = GSV» — **✅** |
| **PH-S1917** | AGENTS / roles | `AGENTS.md` + `GSV_ROLES.md` = entry-point, не лише gsv-server — **✅** |
| **PH-S1918** | Band close | `cargo test` + loc-audit ≥96%; vision-sync; один commit + push — **✅** |

## Спринти (band 128) — kit ops + grouped Galaxy UI ✅

| Sprint | Фокус | Acceptance (ключ) |
|--------|-------|-------------------|
| **PH-S1919** | Kit scripts | `scripts/check_target_disk.sh` (`GSV_*` env) + `git-push-only.sh` + `scripts/README.md` — **✅** |
| **PH-S1920** | Speeds/Rust bins | `gsv-speed-index` + `gsv-rust-diagnostics` + `bin/record-test-speed.sh` + `bin/record-rust-diagnostics.sh` + `bin/gsv-vision-sync.sh` — **✅** |
| **PH-S1921** | OpenCode + commands | `AGENTS.md` OpenCode canon; `.opencode/package.json`; `.cursor/commands/git-push.md`; generic rules (`scripts`, toolchain, runtime-stack) — **✅** |
| **PH-S1922** | Layout wire | `UiLayout`/`UI_GROUPS` + `GET /api/ui/layout` + `render_nav`; 4 groups (ops/vision/sprint/studio) — **✅** |
| **PH-S1923** | Rust cards | `health`/`update`/`ide`/`vision`/`vision-map`/`vision-sync`/`doc-preview` renderers; `CARD_NAMES` 27 — **✅** |
| **PH-S1924** | Sidebar shell | `ui/index.html` `--sidebar-w` + group visibility; default `#sprint`; skip link + `:focus-visible` — **✅** |
| **PH-S1925** | IDE preview | `ide::preview_messages` last 8 jsonl lines; `render_ide` tool/session + preview — **✅** |
| **PH-S1926** | GitHub remote | `Cargo.toml` repository `platinoff/GSV`; `origin` + push at band close — **✅** |
| **PH-S1927** | Chrome/a11y | power-menu click-outside; `aria-expanded`; layout contracts; hint names `record-test-speed.sh` / `record-rust-diagnostics.sh` — **✅** |
| **PH-S1928** | Band close | fmt/clippy/test; loc-audit ≥96%; vision-sync; one commit + push — **✅** |

## Спринти (band 129) — canon port 9999 + dashboard card registry ✅

| Sprint | Фокус | Acceptance (ключ) |
|--------|-------|-------------------|
| **PH-S1929** | Scope + queue | roadmap band 129; `extensions.json` `active_sprint` = `PH-S1929` (sync with `manifest.next_sprint`) — **✅** |
| **PH-S1930** | Live UI URL | `live_ui_url` + `DEFAULT_PORT` 9999; sample feed links; `docs/vision/feed.json` + pointer page — **✅** |
| **PH-S1931** | CARD_NAMES 30 | `preview` / `terminal` / `sprint-focus` renderers + registry (layout ⊆ CARD_NAMES) — **✅** |
| **PH-S1932** | card_wire | preview/terminal wires; sprint-focus uses theme `active_sprint` (not summary) — **✅** |
| **PH-S1933** | UI glue | `data-card` + `rustCards` 23; sidebar `data-card-jump` switches group + scroll — **✅** |
| **PH-S1934** | Contracts | layout reverse; ui/server/vision port+card contracts; stand smoke CARDS 30 — **✅** |
| **PH-S1935** | Docs canon | VISION/LEGACY_PARITY/SERVER/README/BOXES live port 9999; `/api/ui/layout` — **✅** |
| **PH-S1936** | Ratio hold | `gsv-loc-audit --stretch-96` ≥96%; fmt/clippy — **✅** |
| **PH-S1937** | vision-sync | `gsv-vision-sync` + `--check` — **✅** |
| **PH-S1938** | Band close | tests green; MEMORY/HANDOFF/NEXT; one commit + push — **✅** |

## Спринти (band 130) — chrome shell: real wires + Rust RSS ticker ✅

| Sprint | Фокус | Acceptance (ключ) |
|--------|-------|-------------------|
| **PH-S1939** | Scope + queue | roadmap band 130; `extensions.json` `active_sprint` = `PH-S1939` (sync with `manifest.next_sprint`) — **✅** |
| **PH-S1940** | Real chrome wires | rss-ticker reads `feed.feed.items`; starfield Eco/FX/Ms counts from `StarfieldMode::star_count`; galaxy src+opacity; gpu `fx`; power actions; empty dock — **✅** |
| **PH-S1941** | Chrome renderers | `err_html`/`empty_html`; rss ticker emits `<li class='rss-ticker-item'>` duplicated for marquee — **✅** |
| **PH-S1942** | Layout chrome | `GET /api/ui/layout` includes `chrome: CHROME_CARDS` (7) — **✅** |
| **PH-S1943** | UI glue | `loadRssTicker` → `/api/ui/card/rss-ticker`; `resync` includes ticker — **✅** |
| **PH-S1944** | Contracts | chrome unit tests + layout/rss/starfield server contracts + index `api/ui/card/rss-ticker` — **✅** |
| **PH-S1945** | Docs canon | VISION/SERVER/BOXES/ARCHITECTURE/ROLES + MEMORY/HANDOFF/NEXT band 130 — **✅** |
| **PH-S1946** | Ratio hold | `gsv-loc-audit --stretch-96` ≥96%; fmt/clippy — **✅** |
| **PH-S1947** | vision-sync | `gsv-vision-sync` + `--check` — **✅** |
| **PH-S1948** | Band close | tests green; one commit + push — **✅** |

## Спринти (band 131) — Rust shell: live palette/theme CSS + layout nav HTML ✅

| Sprint | Фокус | Acceptance (ключ) |
|--------|-------|-------------------|
| **PH-S1949** | Scope + queue | roadmap band 131; `extensions.json` `active_sprint` = `PH-S1949` (sync with `manifest.next_sprint`) — **✅** |
| **PH-S1950** | Layout nav HTML | `render_nav` inner HTML + `data-card-jump`; `GET /api/ui/layout` `html` — **✅** |
| **PH-S1951** | Palette CSS | `palette_stylesheet` / `GET /api/ui/load-palette` live `:root` (not stub) — **✅** |
| **PH-S1952** | Theme CSS | `SprintThemeReport::as_css_root` / `GET /api/ui/load-theme` `text/css` (not JS stub) — **✅** |
| **PH-S1953** | UI glue | `<link>` palette+theme; `loadLayout` uses `layout.html`; drop JS CSS-var mappers — **✅** |
| **PH-S1954** | Contracts | nav/html + CSS content-type unit/server/ui index contracts — **✅** |
| **PH-S1955** | Docs canon | VISION/SERVER/BOXES/ARCHITECTURE + MEMORY/HANDOFF/NEXT band 131 — **✅** |
| **PH-S1956** | Ratio hold | `gsv-loc-audit --stretch-96` ≥96%; fmt/clippy — **✅** |
| **PH-S1957** | vision-sync | `gsv-vision-sync` + `--check` — **✅** |
| **PH-S1958** | Band close | tests green; one commit + push — **✅** |

## Спринти (band 132) — Rust header chrome HTML + node-search fragment ✅

| Sprint | Фокус | Acceptance (ключ) |
|--------|-------|-------------------|
| **PH-S1959** | Scope + queue | roadmap band 132; `extensions.json` `active_sprint` = `PH-S1959` (sync with `manifest.next_sprint`) — **✅** |
| **PH-S1960** | Header HTML | `render_header` GPU/Auto/Resync/Power `data-action`; `GET /api/ui/layout` `header` — **✅** |
| **PH-S1961** | Node-search renderer | `render_node_search` table HTML from wire; `CARD_NAMES` 31 — **✅** |
| **PH-S1962** | Chrome registry | `CHROME_CARDS` 8 includes `node-search`; `card_wire` `q`/`layer` — **✅** |
| **PH-S1963** | UI glue | `loadLayout` injects `header`; search uses `/api/ui/card/node-search`; drop JS `tab` — **✅** |
| **PH-S1964** | Event delegation | header `data-action` (gpu/auto/resync/power) replaces onclick — **✅** |
| **PH-S1965** | Contracts | layout header + node-search html + CARD_NAMES 31 — **✅** |
| **PH-S1966** | Docs canon | VISION/SERVER/BOXES/ARCHITECTURE + MEMORY/HANDOFF/NEXT band 132 — **✅** |
| **PH-S1967** | Ratio hold | `gsv-loc-audit --stretch-96` ≥96%; fmt/clippy — **✅** |
| **PH-S1968** | Band close | tests green; one commit + push — **✅** |

## Спринти (band 133) — localhost security hardening ✅

Owner 2026-08-17: security check on `gsv-server` (loopback default, mutating POSTs, SLI terminal, `/data`).

| Sprint | Фокус | Acceptance (ключ) |
|--------|-------|-------------------|
| **PH-S1969** | Scope + queue | roadmap band 133; `extensions.json` `active_sprint` = `PH-S1969` — **✅** |
| **PH-S1970** | Loopback bind | `security::ensure_bind_host`; `--host` off-loopback requires `--allow-lan` — **✅** |
| **PH-S1971** | CSRF POST gate | `Sec-Fetch-Site: cross-site` / non-loopback `Origin` → 403 `{ok:false}` — **✅** |
| **PH-S1972** | Terminal whitelist | drop `bash`/`node`/`npm`/`cat`; cargo/git subcommand allowlists; `..` `\\` `~` forbidden — **✅** |
| **PH-S1973** | Data file allowlist | `GET /data/{file}` basenames only (`DATA_FILES`); `omni.toml` not served — **✅** |
| **PH-S1974** | Preview confine | `preview::resolve` rejects absolute/`ParentDir`; canonicalize under repo root — **✅** |
| **PH-S1975** | Omni GET | `/api/omni/config` has `key_set`, no `api_key` field — **✅** |
| **PH-S1976** | Contracts | `tests/gsv_security_contracts.rs` + terminal/preview unit tests — **✅** |
| **PH-S1977** | Docs canon | SERVER/BOXES/ARCHITECTURE/VISION + MEMORY/HANDOFF/NEXT band 133 — **✅** |
| **PH-S1978** | Band close | ratio ≥96%; fmt/clippy/test; vision-sync; one commit + push — **✅** |

## Спринти (band 134) — HTTP response hardening ✅

Owner 2026-08-17: after bind/CSRF, `gsv-server` still shipped HTML/JSON with no CSP, no
`nosniff`, and a 2 MiB default POST body. Band 134 closes that gap.

| Sprint | Фокус | Acceptance (ключ) |
|--------|-------|-------------------|
| **PH-S1979** | Scope + queue | roadmap band 134; `extensions.json` `active_sprint` = `PH-S1979` — **✅** |
| **PH-S1980** | Header constants | `security::SECURITY_HEADERS` + `CSP` (frame-ancestors none, default-src self) — **✅** |
| **PH-S1981** | Response middleware | `security_gate` inserts CSP / nosniff / DENY / no-referrer / COOP / CORP on every reply — **✅** |
| **PH-S1982** | Cache-Control | `Cache-Control: no-store` on all responses (live dashboard, no stale API) — **✅** |
| **PH-S1983** | Body limit | `MAX_BODY_BYTES` 256 KiB; `gate_content_length` + `DefaultBodyLimit` — **✅** |
| **PH-S1984** | 413 JSON | oversized POST → 413 `{ok:false,error}` (canonical shape) — **✅** |
| **PH-S1985** | Contracts | unit header/limit tests + `gsv_security_contracts` GET/403/413 — **✅** |
| **PH-S1986** | Docs canon | SERVER/BOXES/ARCHITECTURE/VISION + MEMORY/HANDOFF/NEXT band 134 — **✅** |
| **PH-S1987** | Ratio hold | `gsv-loc-audit --stretch-96` ≥96%; fmt/clippy — **✅** |
| **PH-S1988** | Band close | tests green; vision-sync; one commit + push — **✅** |

## Спринти (band 135) — `gsv_mcp_openbot` ✅

Owner 2026-08-17: research (Grok Bot + OpenCode + Cursor MCP) → GSV **owns** one MCP
server; those products stay **clients**. Canon: [`GSV_MCP_OPENBOT.md`](./GSV_MCP_OPENBOT.md).

Do **not** embed Grok Bot’s cloud computer or fork OpenCode into `gsv-server`.

| Sprint | Фокус | Acceptance (ключ) |
|--------|-------|-------------------|
| **PH-S1989** | Scope + queue | this band; `extensions.json` `active_sprint` = `PH-S1989`; MCP doc stays Planned — **✅** |
| **PH-S1990** | `gsv-mcp` bin stdio | `src/bin/gsv_mcp.rs`; MCP initialize + tools/list; shares `gsv` lib — **✅** |
| **PH-S1991** | Tool wrap (read) | `gsv_health` / `gsv_tracker` / `gsv_ratio` / `gsv_sli` / `gsv_toolchain` — **✅** |
| **PH-S1992** | Tool wrap (vision + omni) | vision manifest/feed/queue; `gsv_omni_chat` via OmniRouter; secrets redacted — **✅** |
| **PH-S1993** | Terminal + IDE tools | same SLI allowlist as HTTP; `gsv_ide_sessions` read-only — **✅** |
| **PH-S1994** | Auto-register | `.mcp.json` + `.cursor/mcp.json` + `opencode.json` `mcp.gsv_mcp_openbot` — **✅** |
| **PH-S1995** | Optional HTTP `/mcp` | Streamable HTTP on loopback only; `--allow-lan` required off-loopback — **✅** |
| **PH-S1996** | Contracts | MCP initialize/tools tests; no extra shell; loc-audit still ≥96% — **✅** |
| **PH-S1997** | Docs | SERVER/BOXES/ARCHITECTURE + README MCP row; Grok Bot = client (tunnel = owner opt-in) — **✅** |
| **PH-S1998** | Band close | fmt/clippy/test; vision-sync; one commit + push — **✅** |

## Спринти (band 136) — MCP Galaxy UI + remaining read tools ✅

Owner 2026-08-17: after band 135, `gsv_mcp_openbot` had no Galaxy card and only 11 tools.
Band 136 wraps the remaining read-only boxes and shows the server on the dashboard.

| Sprint | Фокус | Acceptance (ключ) |
|--------|-------|-------------------|
| **PH-S1999** | Scope + queue | this band; `extensions.json` `active_sprint` = `PH-S1999` — **✅** |
| **PH-S2000** | Extra read tools | `gsv_vision_{map,board,progress,speeds,rust}` + `gsv_hooks_{tests,bench}` + `gsv_update` (19 tools) — **✅** |
| **PH-S2001** | Discovery wire | `GET /mcp` `stdio` / `http` / `tool_count` (same payload as the card) — **✅** |
| **PH-S2002** | MCP card | `render_mcp` + `CARD_NAMES` 32 + ops group; `GET /api/ui/card/mcp` — **✅** |
| **PH-S2003** | UI glue | `data-card="mcp"` + `rustCards` 24 — **✅** |
| **PH-S2004** | Grok overlay | `.grok/config.toml` `[mcp_servers.gsv_mcp_openbot]` (project scope) — **✅** |
| **PH-S2005** | Contracts | mcp tools 19 + ui/server/stand-smoke card + grok overlay — **✅** |
| **PH-S2006** | Docs canon | MCP_OPENBOT / SERVER / BOXES / HANDOFF / NEXT / MEMORY / roadmap — **✅** |
| **PH-S2007** | Ratio hold | `gsv-loc-audit --stretch-96` ≥96%; fmt/clippy — **✅** |
| **PH-S2008** | Band close | tests green; vision-sync; one commit + push — **✅** |

## Спринти (band 137) — MCP vision completeness ✅

Owner 2026-08-17: after band 136, agents still lacked sprint-map / doc-preview /
node-search / sync / extensions / summary / preview. Band 137 wraps those boxes
(same confine as HTTP). Clippy 0 going in.

| Sprint | Фокус | Acceptance (ключ) |
|--------|-------|-------------------|
| **PH-S2009** | Scope + queue | this band; `extensions.json` `active_sprint` = `PH-S2009` — **✅** |
| **PH-S2010** | Sprint map + docs | `gsv_vision_sprint_map` + `gsv_vision_doc_preview` (`id`) + `gsv_vision_node_search` (`q`/`layer`) — **✅** |
| **PH-S2011** | Sync + extensions | `gsv_vision` summary + `gsv_vision_sync` + `gsv_vision_extensions` — **✅** |
| **PH-S2012** | Preview confine | `gsv_preview` (`file`); traversal / absolute rejected (same as `GET /api/preview`) — **✅** |
| **PH-S2013** | Schemas | parameterized inputSchema for id / q / layer / file — **✅** |
| **PH-S2014** | Discovery | `GET /mcp` `tool_count` 26; Galaxy card lists new tools — **✅** |
| **PH-S2015** | Contracts | mcp unit + `gsv_mcp_contracts` (26 tools, preview reject, doc-preview/search) — **✅** |
| **PH-S2016** | Docs canon | MCP_OPENBOT / SERVER / BOXES / ARCHITECTURE / HANDOFF / NEXT / MEMORY / roadmap — **✅** |
| **PH-S2017** | Ratio hold | `gsv-loc-audit --stretch-96` ≥96%; fmt/clippy — **✅** |
| **PH-S2018** | Band close | tests green; vision-sync; one commit + push — **✅** |

## Спринти (band 138) — MCP resources + prompts ✅

Owner 2026-08-17: after band 137, agents had 26 tools but no MCP `resources/*`
or `prompts/*`. Band 138 advertises both, allowlists `gsv://` URIs (same path
confine as preview), and ships three drain prompts. Kit trigger alias
`abrakadabra` lands with the docs canon.

| Sprint | Фокус | Acceptance (ключ) |
|--------|-------|-------------------|
| **PH-S2019** | Scope + queue | this band; `extensions.json` `active_sprint` = `PH-S2019`; `abrakadabra` alias — **✅** |
| **PH-S2020** | Capabilities | `initialize` advertises `resources` + `prompts` (`listChanged: false`) — **✅** |
| **PH-S2021** | resources/list | 6 `gsv://` URIs (vision manifest/feed/extensions + docs mcp/handoff/next) — **✅** |
| **PH-S2022** | resources/read | allowlist + `preview::resolve`; traversal / `file://` / unknown → `-32602` — **✅** |
| **PH-S2023** | prompts | `prompts/list` + `prompts/get` (`gsv_status` / `gsv_vision_brief` / `gsv_drain`) — **✅** |
| **PH-S2024** | Discovery + card | `GET /mcp` `resource_count`/`prompt_count`; Galaxy card lists both — **✅** |
| **PH-S2025** | Contracts | mcp unit + `gsv_mcp_contracts` + ui card resources/prompts — **✅** |
| **PH-S2026** | Docs canon | MCP_OPENBOT / SERVER / BOXES / ARCHITECTURE / HANDOFF / NEXT / MEMORY / roadmap — **✅** |
| **PH-S2027** | Ratio hold | `gsv-loc-audit --stretch-96` ≥96%; fmt/clippy — **✅** |
| **PH-S2028** | Band close | tests green; vision-sync; one commit + push — **✅** |

## Спринти (band 139) — MCP logging + completions ✅

Owner 2026-08-17: after band 138, agents could list/read `gsv://` resources and
named prompts, but Cursor/OpenCode still lacked MCP `completion/complete`
(URI/name autocomplete) and `logging/setLevel`. Band 139 advertises both,
allowlists completion prefixes (same `..` / `file:` reject as resources/read),
and surfaces the process log level on `GET /mcp` + the Galaxy card.

| Sprint | Фокус | Acceptance (ключ) |
|--------|-------|-------------------|
| **PH-S2029** | Scope + queue | this band; `extensions.json` `active_sprint` = `PH-S2029` — **✅** |
| **PH-S2030** | Capabilities | `initialize` advertises `logging` + `completions` — **✅** |
| **PH-S2031** | logging/setLevel | RFC 5424 levels; invalid → `-32602`; process-local on `AppState` — **✅** |
| **PH-S2032** | completion resources | `ref/resource` prefix-match allowlisted `gsv://` URIs; `..` / `file:` → `-32602` — **✅** |
| **PH-S2033** | completion prompts | `ref/prompt` prefix-match prompt names; unknown ref type → `-32602` — **✅** |
| **PH-S2034** | Discovery + card | `GET /mcp` `logging`/`completions`/`log_level`; Galaxy card lists both — **✅** |
| **PH-S2035** | Contracts | mcp unit + `gsv_mcp_contracts` + ui/server card logging/completions — **✅** |
| **PH-S2036** | Docs canon | MCP_OPENBOT / SERVER / BOXES / ARCHITECTURE / HANDOFF / NEXT / MEMORY / roadmap — **✅** |
| **PH-S2037** | Ratio hold | `gsv-loc-audit --stretch-96` ≥96%; fmt/clippy — **✅** |
| **PH-S2038** | Band close | tests green; vision-sync; one commit + push — **✅** |

## Спринти (band 140) — MCP resource subscribe + logging notifications ✅

Owner 2026-08-17: after band 139, `logging/setLevel` stored a process-local
level but the server never emitted `notifications/message`, and
`resources.subscribe` stayed `false`. Band 140 advertises subscribe, allowlists
`resources/subscribe`+`unsubscribe` (same `gsv://` confine as read), flushes
`notifications/message` on stdio (filtered by log level), and emits
`notifications/resources/updated` for subscribed `gsv://vision/*` URIs after
`gsv_vision_sync`.

| Sprint | Фокус | Acceptance (ключ) |
|--------|-------|-------------------|
| **PH-S2039** | Scope + queue | this band; `extensions.json` `active_sprint` = `PH-S2039` — **✅** |
| **PH-S2040** | Capabilities | `initialize` advertises `resources.subscribe: true` — **✅** |
| **PH-S2041** | subscribe/unsubscribe | allowlisted `gsv://` only; `file:` / `..` / unknown → `-32602` — **✅** |
| **PH-S2042** | resource updated | `gsv_vision_sync` → `notifications/resources/updated` for subscribed vision URIs — **✅** |
| **PH-S2043** | logging notifications | `notifications/message` filtered by `mcp_log_level` (idx < min skipped) — **✅** |
| **PH-S2044** | Discovery + card | `GET /mcp` `subscribe` / `subscription_count` / `subscriptions`; Galaxy card lists count — **✅** |
| **PH-S2045** | Contracts | mcp unit + `gsv_mcp_contracts` + ui/server card subscribe — **✅** |
| **PH-S2046** | Docs canon | MCP_OPENBOT / SERVER / BOXES / ARCHITECTURE / HANDOFF / NEXT / MEMORY / roadmap — **✅** |
| **PH-S2047** | Ratio hold | `gsv-loc-audit --stretch-96` ≥96%; fmt/clippy — **✅** |
| **PH-S2048** | Band close | tests green; vision-sync; one commit + push — **✅** |

## Спринти (band 141) — MCP HTTP SSE / streamable notifications ✅

Owner 2026-08-17: after band 140, stdio flushed `notifications/message` and
`notifications/resources/updated`, but HTTP `POST /mcp` discarded the queue
and `GET /mcp` advertised `transport: streamable-http` without SSE. Band 141
honors `Accept: text/event-stream` on GET/POST (finite SSE: pending notes,
then the JSON-RPC result on POST). JSON discovery stays the default for
Galaxy / stand-smoke. Same `gsv://` confine; no LAN widen.

| Sprint | Фокус | Acceptance (ключ) |
|--------|-------|-------------------|
| **PH-S2049** | Scope + queue | this band; `extensions.json` `active_sprint` = `PH-S2049` — **✅** |
| **PH-S2050** | SSE helpers | `wants_sse` / `format_sse_message` / `sse_body` (`event: message`) — **✅** |
| **PH-S2051** | POST SSE | `Accept: text/event-stream` → notifications then JSON-RPC result — **✅** |
| **PH-S2052** | GET SSE | `Accept: text/event-stream` flushes pending; JSON discovery unchanged — **✅** |
| **PH-S2053** | Discovery + card | `GET /mcp` `sse` / `streamable`; Galaxy card lists sse — **✅** |
| **PH-S2054** | JSON drain | POST without SSE still JSON; queue drained; loopback unless `--allow-lan` — **✅** |
| **PH-S2055** | Contracts | mcp unit + `gsv_mcp_contracts` POST/GET SSE + ui/server card sse — **✅** |
| **PH-S2056** | Docs canon | MCP_OPENBOT / SERVER / BOXES / ARCHITECTURE / HANDOFF / NEXT / MEMORY / roadmap — **✅** |
| **PH-S2057** | Ratio hold | `gsv-loc-audit --stretch-96` ≥96%; fmt/clippy — **✅** |
| **PH-S2058** | Band close | tests green; vision-sync; one commit + push — **✅** |

## Спринти (band 142) — MCP HTTP sessions (`Mcp-Session-Id`) ✅

Owner 2026-08-17: after band 141, HTTP SSE flushed notifications as a finite
body but there was no `Mcp-Session-Id`, GET/POST treated every caller as the
same process queue, and `DELETE /mcp` did not exist. Band 142 issues a
process-local session on HTTP `initialize`, 404s unknown ids, and ends the
session on `DELETE /mcp`. JSON discovery stays sessionless for Galaxy /
stand-smoke. Same `gsv://` confine; no LAN widen; stdio does not issue HTTP
sessions.

| Sprint | Фокус | Acceptance (ключ) |
|--------|-------|-------------------|
| **PH-S2059** | Scope + queue | this band; `extensions.json` `active_sprint` = `PH-S2059` — **✅** |
| **PH-S2060** | Session store | process-local map on `AppState` (cap 32, oldest dropped) — **✅** |
| **PH-S2061** | Initialize | HTTP `initialize` issues `Mcp-Session-Id` (alphanumeric + hyphen) — **✅** |
| **PH-S2062** | Unknown id | POST/GET with unknown `Mcp-Session-Id` → 404 `{ok:false}`; missing header still allowed — **✅** |
| **PH-S2063** | DELETE | `DELETE /mcp` requires id (400 if missing); ends session; reuse → 404 — **✅** |
| **PH-S2064** | Discovery + card | `GET /mcp` `sessions` / `session_count`; Galaxy card lists sessions — **✅** |
| **PH-S2065** | Contracts | mcp unit + `gsv_mcp_contracts` init/404/DELETE + ui/server card sessions — **✅** |
| **PH-S2066** | Docs canon | MCP_OPENBOT / SERVER / BOXES / ARCHITECTURE / HANDOFF / NEXT / MEMORY / roadmap — **✅** |
| **PH-S2067** | Ratio hold | `gsv-loc-audit --stretch-96` ≥96%; fmt/clippy — **✅** |
| **PH-S2068** | Band close | tests green; vision-sync; one commit + push — **✅** |

## Спринти (band 143) — Galaxy chrome + type/chart scale

Owner 2026-08-17: server must stay the live product; first drain is **debug chrome** (power menu under cards, collapse/fullscreen), then type/chart balance. Spec: [`GSV_ALWAYS_ON_UI.md`](./GSV_ALWAYS_ON_UI.md). Plan: [`docs/superpowers/plans/2026-08-17-always-on-galaxy.md`](../superpowers/plans/2026-08-17-always-on-galaxy.md). **✅** this band. Next `абракадабра` gsv drain = **band 144**.

| Sprint | Фокус | Acceptance (ключ) |
|--------|-------|-------------------|
| **PH-S2069** | Scope + queue | this band; spec Accepted; `extensions.json` `active_sprint` = `PH-S2069` — **✅** |
| **PH-S2070** | Power menu stack | header z-index ≥ 40; drop `body>header,.workspace{z-index:2}`; menu z-index 80 — **✅** |
| **PH-S2071** | Exclusive fullscreen | one `.fullscreen`; `data-action='card-fs'`; `exitFullscreen()`; Esc not `:last-child` — **✅** |
| **PH-S2072** | Collapse → dock | `.card.collapsed{display:none}`; restore from dock — **✅** |
| **PH-S2073** | Type scale | `--fs-ui/card/meta/chart`; card body max-height 420px — **✅** |
| **PH-S2074** | Chart SVG | speed/rust height 168; font-size 11; ui-monospace stack — **✅** |
| **PH-S2075** | Contracts | `gsv_ui_contracts` stack/collapse/fs/type markers — **✅** |
| **PH-S2076** | Docs canon | ALWAYS_ON_UI / BOXES / HANDOFF / NEXT / MEMORY / roadmap — **✅** |
| **PH-S2077** | Ratio hold | `gsv-loc-audit --stretch-96` ≥96%; fmt/clippy — **✅** |
| **PH-S2078** | Band close | tests green; vision-sync; one commit + push — **✅** |

## Спринти (band 144) — Always-on live binary + offline during apply ✅

Windows locks the running exe. Canon process is a **copy** (`target/live/gsv-server.exe`) so `cargo test`/`build` may overwrite `target/debug/`. UI goes **offline** on apply, SSE `onopen` resyncs.

| Sprint | Фокус | Acceptance (ключ) |
|--------|-------|-------------------|
| **PH-S2079** | Scope + queue | this band; `active_sprint` = `PH-S2079` — **✅** |
| **PH-S2080** | `scripts/gsv-live.sh` | copy debug → live; loop restart; gitignore `target/live/` — **✅** |
| **PH-S2081** | Apply API | `POST /api/update/apply` → SSE offline; exit gated by `GSV_UPDATE_APPLY_EXIT` — **✅** |
| **PH-S2082** | UI apply | `doUpdate()` POST apply; stay offline until SSE `onopen` — **✅** |
| **PH-S2083** | Drain docs | AGENTS/HANDOFF/NEXT: do **not** kill live copy before cargo test — **✅** |
| **PH-S2084** | Contracts | update-flow apply + server POST 200 `{ok,applying}` — **✅** |
| **PH-S2085** | GSV_SERVER | live-copy + apply scenario — **✅** |
| **PH-S2086** | Docs canon | HANDOFF / NEXT / MEMORY / roadmap — **✅** |
| **PH-S2087** | Ratio hold | `--stretch-96` ≥96%; fmt/clippy — **✅** |
| **PH-S2088** | Band close | tests green; vision-sync; one commit + push — **✅** |

## Спринти (band 145) — VDT products picker + open folder + scan ✅

Same discovery merge as `scripts/list-vdt-products.sh`, in Rust. Open path confined to discovered roots.

| Sprint | Фокус | Acceptance (ключ) |
|--------|-------|-------------------|
| **PH-S2089** | Scope + queue | this band; `active_sprint` = `PH-S2089` — **✅** |
| **PH-S2090** | `boxes/products.rs` | `discover(kit_root)` includes gsv rust registered — **✅** |
| **PH-S2091** | HTTP list/select | `GET /api/products`; `POST /api/products/select`; unknown id → 404 — **✅** |
| **PH-S2092** | Open folder | `POST /api/products/open`; explorer/cursor; id allowlist — **✅** |
| **PH-S2093** | Auto-parse scan | `GET /api/products/scan` git/kind/HANDOFF/cargo_name (no cargo test) — **✅** |
| **PH-S2094** | Galaxy card | `render_products`; `CARD_NAMES` + ops group — **✅** |
| **PH-S2095** | Contracts | `gsv_products_contracts` + ui/server — **✅** |
| **PH-S2096** | Docs canon | BOXES / SERVER / HANDOFF / NEXT / MEMORY — **✅** |
| **PH-S2097** | Ratio hold | `--stretch-96` ≥96%; fmt/clippy — **✅** |
| **PH-S2098** | Band close | tests green; vision-sync; one commit + push — **✅** |

## Спринти (band 146) — Version bump + fingerprints

Each drain commit increments `CARGO_PKG_VERSION` patch. Fingerprint JSONL: actor, IDE, model, agent, time.

| Sprint | Фокус | Acceptance (ключ) |
|--------|-------|-------------------|
| **PH-S2099** | Scope + queue | this band; `active_sprint` = `PH-S2099` — **✅** |
| **PH-S2100** | Version tests | `assert_eq!(wire.version, env!("CARGO_PKG_VERSION"))` (no hardcoded `0.1.0`) — **✅** |
| **PH-S2101** | `gsv-bump-version.sh` | patch +1 in `Cargo.toml` `[package]` — **✅** |
| **PH-S2102** | Fingerprint module | `docs/gsv/fingerprints.jsonl`; `append` / `latest` — **✅** |
| **PH-S2103** | HTTP + card | `GET /api/fingerprints`; `render_fingerprints`; ops group — **✅** |
| **PH-S2104** | Drain scripts | `gsv-fingerprint.sh` + commit trailers `Gsv-Actor/Ide/Model` — **✅** |
| **PH-S2105** | Contracts | `gsv_fingerprint_contracts` + ui/server — **✅** |
| **PH-S2106** | Docs canon | HANDOFF close step = bump + fingerprint; MEMORY — **✅** |
| **PH-S2107** | Ratio hold | `--stretch-96` ≥96%; fmt/clippy — **✅** |
| **PH-S2108** | Band close | bump in the same commit; tests green; vision-sync; push — **✅** |

## Спринти (band 147) — README-level Galaxy polish leftovers ✅

Visual pass vs `docs/assets/presentations/`; stand-smoke new cards; README Quick start → `gsv-live.sh`.

| Sprint | Фокус | Acceptance (ключ) |
|--------|-------|-------------------|
| **PH-S2109** | Scope + queue | this band; `active_sprint` = `PH-S2109` — **✅** |
| **PH-S2110** | Header/card density | padding/gap vs presentation shots (not pixel-perfect) — **✅** |
| **PH-S2111** | Stand-smoke | `products` + `fingerprints` in `CARDS` — **✅** |
| **PH-S2112** | README Quick start | `bash scripts/gsv-live.sh` as canon run — **✅** |
| **PH-S2113** | Architecture note | live-copy in `GSV_ARCHITECTURE.md` — **✅** |
| **PH-S2114** | Docs index | ALWAYS_ON_UI row in `docs/gsv/README.md` — **✅** |
| **PH-S2115** | Contracts | stand-smoke + ui leftover markers — **✅** |
| **PH-S2116** | Docs canon | HANDOFF / NEXT / MEMORY / roadmap — **✅** |
| **PH-S2117** | Ratio hold | `--stretch-96` ≥96%; fmt/clippy — **✅** |
| **PH-S2118** | Band close | tests green; vision-sync; one commit + push — **✅** |

## Спринти (band 148) — Service Worker shell cache ✅

Owner-picked P2 leftover from [`GSV_ALWAYS_ON_UI.md`](./GSV_ALWAYS_ON_UI.md): static UI opens offline. SW script is Rust-rendered (`GET /sw.js`); `/events` / `/mcp` / non-GET stay network-only.

| Sprint | Фокус | Acceptance (ключ) |
|--------|-------|-------------------|
| **PH-S2119** | Scope + queue | this band; `active_sprint` = `PH-S2119` — **✅** |
| **PH-S2120** | `boxes/sw.rs` | CACHE_NAME + PRECACHE (`/` + palette/theme + galaxy/vision svg); no `/mcp` `/events` — **✅** |
| **PH-S2121** | `GET /sw.js` | `text/javascript` + `Service-Worker-Allowed: /`; Cache API install/activate — **✅** |
| **PH-S2122** | `GET /api/sw` | `{ok, cache, script, urls}` — **✅** |
| **PH-S2123** | CSP + register | `worker-src 'self'`; thin `serviceWorker.register("/sw.js")` — **✅** |
| **PH-S2124** | Ops card | `render_sw`; `CARD_NAMES` 35; ops group — **✅** |
| **PH-S2125** | Contracts | `tests/gsv_sw_contracts.rs` + lib unit — **✅** |
| **PH-S2126** | Docs canon | SERVER / BOXES / ALWAYS_ON_UI P2 / HANDOFF / NEXT / MEMORY — **✅** |
| **PH-S2127** | Ratio hold | `--stretch-96` ≥96%; fmt/clippy — **✅** |
| **PH-S2128** | Band close | tests green; vision-sync; one commit + push — **✅** |

## Спринти (band 149) — OmniRoute PRODUCTS.md + semver minor = band ✅

Owner-picked remaining P2 from [`GSV_ALWAYS_ON_UI.md`](./GSV_ALWAYS_ON_UI.md): register omniroute (node) and set crate semver minor to the VDT band.

| Sprint | Фокус | Acceptance (ключ) |
|--------|-------|-------------------|
| **PH-S2129** | Scope + queue | this band; `active_sprint` = `PH-S2129` — **✅** |
| **PH-S2130** | PRODUCTS.md | `\| **omniroute**` row (AGENTS.md / ROADMAP.md / `npm test` / ratio n/a) — **✅** |
| **PH-S2131** | Discover contracts | sibling omniroute `registered` + `kind=node` — **✅** |
| **PH-S2132** | Scan fallback | `AGENTS.md` counts as handoff; `docs/ROADMAP.md` as next — **✅** |
| **PH-S2133** | Abracadabra flow | registered node: S0 + git; **no** PH-S* invent — **✅** |
| **PH-S2134** | `gsv-bump-version.sh` | `--band N` / `GSV_BAND` → `0.N.0`; same band patch +1 — **✅** |
| **PH-S2135** | Bump contracts | 0.1.3→0.149.0; 0.149.0→0.149.1; missing band fails — **✅** |
| **PH-S2136** | Docs canon | ALWAYS_ON_UI P2 / HANDOFF / NEXT / MEMORY / BOXES / PRODUCTS — **✅** |
| **PH-S2137** | Ratio hold | `--stretch-96` ≥96%; fmt/clippy — **✅** |
| **PH-S2138** | Band close | tests green; vision-sync; `--band 149` + fingerprint; one commit + push — **✅** |

## Спринти (band 150) — live watchdog ✅

Owner ask: keep `:9999` up when `gsv-live.sh` (Cursor terminal) dies. Outer loop probes health and respawns the live copy.

| Sprint | Фокус | Acceptance (ключ) |
|--------|-------|-------------------|
| **PH-S2139** | Scope + queue | this band; `active_sprint` = `PH-S2139` — **✅** |
| **PH-S2140** | `boxes/watchdog.rs` | tick / cooldown / heartbeat / copy debug→live — **✅** |
| **PH-S2141** | `gsv-watchdog` bin | probe `/api/health`; spawn detached; `--once` — **✅** |
| **PH-S2142** | HTTP | `GET /api/watchdog`; health `watchdog_alive` — **✅** |
| **PH-S2143** | Scripts | `gsv-watchdog.sh` + `gsv-watchdog-install.sh` (schtasks / HKCU Run) — **✅** |
| **PH-S2144** | Health card | `watchdog` row in `render_health` — **✅** |
| **PH-S2145** | Contracts | `tests/gsv_watchdog_contracts.rs` — **✅** |
| **PH-S2146** | Docs canon | SERVER / BOXES / ALWAYS_ON_UI / HANDOFF / NEXT / MEMORY — **✅** |
| **PH-S2147** | Ratio hold | `--stretch-96` ≥96%; fmt/clippy — **✅** |
| **PH-S2148** | Band close | tests green; vision-sync; `--band 150` + fingerprint; one commit + push — **✅** |

## Спринти (band 151) — MCP always-on catch-up ✅

Owner 2026-08-18: wrap products / scan / watchdog / sw / fingerprints on `gsv_mcp_openbot`. Spec: [`GSV_POST_ALWAYS_ON.md`](./GSV_POST_ALWAYS_ON.md). Plan: [`docs/superpowers/plans/2026-08-18-mcp-always-on-catchup.md`](../superpowers/plans/2026-08-18-mcp-always-on-catchup.md).

| Sprint | Фокус | Acceptance (ключ) |
|--------|-------|-------------------|
| **PH-S2149** | Scope + queue | this band; `active_sprint` = `PH-S2149` — **✅** |
| **PH-S2150** | `gsv_products` + scan | wrap `products::wire` / `scan`; `id` required; unknown id → tool error — **✅** |
| **PH-S2151** | `gsv_watchdog` + `gsv_sw` | wrap `watchdog::wire` / `sw::wire` — **✅** |
| **PH-S2152** | `gsv_fingerprints` | wrap `fingerprint::wire` + `clamp_limit` — **✅** |
| **PH-S2153** | Resources | `gsv://docs/fingerprints` + `gsv://docs/post-always-on`; `..` / `file:` → `-32602` — **✅** |
| **PH-S2154** | `gsv_drain` prompt | text names products / scan / watchdog + `gsv://docs/next` — **✅** |
| **PH-S2155** | Discovery | `TOOL_NAMES` 31; `RESOURCE_URIS` 8; Galaxy card count — **✅** |
| **PH-S2156** | Contracts | `gsv_mcp_contracts` + mcp unit (no hardcoded 26) — **✅** |
| **PH-S2157** | Docs canon | MCP_OPENBOT / SERVER / BOXES / ARCHITECTURE / HANDOFF / NEXT / MEMORY — **✅** |
| **PH-S2158** | Band close | tests green; `--stretch-96` ≥96%; `--band 151` + fingerprint; one commit + push — **✅** |

## Спринти (band 152) — MCP products select ✅

`gsv_products_select` `{id}` (same allowlist as HTTP); `gsv_products_scan` may omit `id` when selected. Still **no** `gsv_products_open`, **no** `update/apply`.

| Sprint | Фокус | Acceptance (ключ) |
|--------|-------|-------------------|
| **PH-S2159** | Scope + queue | this band; `active_sprint` = `PH-S2159` — **✅** |
| **PH-S2160** | `gsv_products_select` | wrap select allowlist; unknown id → tool error — **✅** |
| **PH-S2161** | Scan without id | omit `id` when `AppState` has a selection — **✅** |
| **PH-S2162** | `gsv_drain` prompt | uses selected id — **✅** |
| **PH-S2163** | Contracts | mcp unit + `gsv_mcp_contracts` — **✅** |
| **PH-S2164** | Docs | MCP_OPENBOT / HANDOFF / NEXT — **✅** |
| **PH-S2165** | Ratio hold | `--stretch-96` **96.61%** (19921/20619); fmt/clippy 0 — **✅** |
| **PH-S2166** | (reserve) | — **✅** |
| **PH-S2167** | (reserve) | — **✅** |
| **PH-S2168** | Band close | tests **399** green; `--stretch-96` 96.61%; `--band 152` → `0.152.0` + fingerprint; one commit + push — **✅** |

Band **154** (watchdog ops card + fingerprint model) closed this session. Next gsv drain: scan / owner pick (Grok Bot tunnel stays opt-in).

## Спринти (band 153) — rust-first tests/benches/scripts ✅

Owner ask: product tests, benches, and kit scripts in `.rs` (`cargo xtask`), not `.sh` / `.ps1` / JSON harnesses. MCP `gsv_xtask` + `gsv_disk` + `gsv://docs/rust-dev`. Compared 10 Rust projects (cargo-xtask / rust-analyzer / cargo / helix / clap / tokio / ripgrep / wgpu / zellij / rustc bootstrap).

| Sprint | Фокус | Acceptance (ключ) |
|--------|-------|-------------------|
| **PH-S2169** | Canon | `GSV_RUST_DEV.md` + `gsv-rust-dev.mdc` + `cargo xtask` alias — **✅** |
| **PH-S2170** | Box + bins | `boxes/xtask.rs` · `gsv-xtask` · `gsv-live` — **✅** |
| **PH-S2171** | Port scripts | products/disk/live/watchdog/push/mirrors/bump/fingerprint/record/sync — **✅** |
| **PH-S2172** | MCP | `gsv_xtask` + `gsv_disk` + resource rust-dev (34 tools / 9 URIs) — **✅** |
| **PH-S2173** | SLI / terminal / ratio | catalog `src/bin` + xtask; `cargo xtask` allowlist; ops_shell note — **✅** |
| **PH-S2174** | Delete `.sh` | product `scripts/*.sh` / `bin/*.sh` gone — **✅** |
| **PH-S2175** | Benches | `benches/gsv_dev.rs` (`cargo bench --bench gsv_dev`) — **✅** |
| **PH-S2176** | Contracts | `gsv_xtask_contracts` + fingerprint/watchdog/update tests — **✅** |
| **PH-S2177** | Docs / skills | AGENTS / HANDOFF / NEXT / abracadabra / MCP_OPENBOT — **✅** |
| **PH-S2178** | Band close | tests green; `--band 153` + fingerprint; one commit + push — **✅** |

## Спринти (band 154) — watchdog ops card + fingerprint model ✅

Owner pick. Dedicated Galaxy ops card `watchdog`; fingerprint `model` from Cursor session vs env. Still **no** MCP `products/open`, **no** `update/apply`, **no** Grok Bot tunnel.

| Sprint | Фокус | Acceptance (ключ) |
|--------|-------|-------------------|
| **PH-S2179** | Scope + queue | this band — **✅** |
| **PH-S2180** | Watchdog ops card | `render_watchdog` + `CARD_NAMES` 36 + Galaxy ops — **✅** |
| **PH-S2181** | Fingerprint model | `GSV_MODEL` else Cursor session (`CURSOR_MODEL` / `GSV_SESSION_FILE`); default `unknown` stays valid — **✅** |
| **PH-S2182** | (reserve) | — **✅** |
| **PH-S2183** | (reserve) | — **✅** |
| **PH-S2184** | (reserve) | — **✅** |
| **PH-S2185** | (reserve) | — **✅** |
| **PH-S2186** | Contracts | card + health row + `resolve_model_from` — **✅** |
| **PH-S2187** | Docs | HANDOFF / NEXT / BOXES — **✅** |
| **PH-S2188** | Band close | tests **428** green; `--stretch-96` **99.14%**; `--band 154` + fingerprint; one commit + push — **✅** |

## Спринти (band 155) — session token usage (MCP + OmniRoute + sync)

Owner ask: automatic per-session token spend, aligned with `gsv_mcp_openbot`, OmniRoute, and vision-sync.

| Sprint | Фокус | Acceptance (ключ) |
|--------|-------|-------------------|
| **PH-S2189** | Usage box | `boxes/usage.rs` parse OpenAI/OmniRoute/Gemini usage; session keys (`mcp:` / `stdio` / `process`); persist `data/gsv_usage.json` — **✅** |
| **PH-S2190** | Auto-count | OmniRouter `json_response` records live completions; dry-run stays 0 — **✅** |
| **PH-S2191** | MCP bot | `gsv_omni_chat` tags `x-gsv-source=mcp` + HTTP/`stdio` session; tool `gsv_usage` (**35** tools) — **✅** |
| **PH-S2192** | OmniRoute | fail-open `GET {base}/api/usage/history` (default `127.0.0.1:20128`); skip under cargo-test — **✅** |
| **PH-S2193** | Sync | vision-sync writes usage snapshot (`usage_target`); `/api/vision/sync` + `gsv_vision_sync` refresh OmniRoute pull — **✅** |
| **PH-S2194** | Galaxy | `GET /api/usage` + studio card `usage`; `CARD_NAMES` **37** — **✅** |
| **PH-S2195** | Contracts | `tests/gsv_usage_contracts.rs` — **✅** |
| **PH-S2196** | Stand-smoke / DATA | `/api/usage` + `gsv_usage.json` allowlist — **✅** |
| **PH-S2197** | Docs | BOXES / SERVER / MCP_OPENBOT / HANDOFF / NEXT / MEMORY — **✅** |
| **PH-S2198** | Band close | tests **445** green; `--stretch-96` **99.14%**; `--band 155` + fingerprint; one commit + push — **✅** |

## Спринти (band 156) — streaming usage + VDT git + owner tunnel

Owner pick: record `stream:true` tokens; fullscreen chart imgs; Grok Bot `cargo xtask tunnel`; universal `cargo xtask git` replacing `comitmsg/*.sh` (messages `.md`, logs `.log`).

| Sprint | Фокус | Acceptance (ключ) |
|--------|-------|-------------------|
| **PH-S2199** | Scope + queue | this band — **✅** |
| **PH-S2200** | Streaming usage | SSE tap + `stream_options.include_usage`; record on stream end — **✅** |
| **PH-S2201** | Chart fullscreen | `.card.fullscreen img{max-height:none` — **✅** |
| **PH-S2202** | Tunnel | `cargo xtask tunnel` → cloudflared loopback; MCP never starts it — **✅** |
| **PH-S2203** | `cargo xtask git` | status/log/fetch/commit `--file comitmsg/*.md`/push; no add -A — **✅** |
| **PH-S2204** | comitmsg | `.sh`/`.txt` retired; `.md` + `.log`; gitignore except README — **✅** |
| **PH-S2205** | Contracts | usage SSE + ui fullscreen + gitkit + xtask catalog — **✅** |
| **PH-S2206** | Docs / kit | RUST_DEV / MCP_OPENBOT / git-workflow / abracadabra never-stage — **✅** |
| **PH-S2207** | Ratio hold | `--stretch-96` ≥96%; fmt/clippy — **✅** |
| **PH-S2208** | Band close | tests **457** green; `--stretch-96` **99.17%**; `--band 156` + fingerprint; one commit + push — **✅** |

## Спринти (band 157) — OmniRouter shared catalog + quota timers

Owner pick (`абракадабра` gsv / omnirouter): research Cursor / OpenCode / Grok / Omni models for Rust+web; shared catalog with free notes and `reset_secs` so MCP auto-switches on cooldown.

| Sprint | Фокус | Acceptance (ключ) |
|--------|-------|-------------------|
| **PH-S2209** | Catalog | `catalog.rs` xAI/Cursor/Grok 4.6, rust+web+clients, quota windows — **✅** |
| **PH-S2210** | Quota store | `quota.rs` `data/omni_quota.json`; 429 / RPM → cooldown — **✅** |
| **PH-S2211** | Route | `select_provider` skips cooling; `GET /api/omni/route`; empty-model chat auto-pick — **✅** |
| **PH-S2212** | MCP | `gsv_omni_route` + resource `gsv://docs/omni-catalog`; tools **36** / resources **10** — **✅** |
| **PH-S2213** | Galaxy | `render_omni` free/timer columns; wire `clients` + `researched_at` — **✅** |
| **PH-S2214** | Contracts | omni route + catalog recommended rust+web + mcp tool — **✅** |
| **PH-S2215** | Docs | `GSV_OMNI_CATALOG.md` + BOXES / SERVER / MCP_OPENBOT — **✅** |
| **PH-S2216** | (reserve) | — **✅** |
| **PH-S2217** | Ratio hold | `--stretch-96` **99.21%**; fmt/clippy — **✅** |
| **PH-S2218** | Band close | tests **466** green; `--band 157` + fingerprint; one commit + push — **✅** |

## Спринти (band 158) — live MCP stdio + sync check ✅

Owner ask (`абракадабра` gsv): perfect working bot with MCP and synchronizations. Scan: client JSON still `cargo run --bin gsv-mcp` (cargo lock + slow); POST `/mcp` CSRF blocked Cursor/Grok origins; `gsv_xtask` had no drift check; `gsv_vision_sync` only notified `gsv://vision/*`.

| Sprint | Фокус | Acceptance (ключ) |
|--------|-------|-------------------|
| **PH-S2219** | Scope + queue | this band; `active_sprint` = `PH-S2219` — **✅** |
| **PH-S2220** | Live `gsv-mcp` | `copy_debug_to_live` copies `gsv-mcp` best-effort next to `gsv-server` — **✅** |
| **PH-S2221** | Client configs | `.mcp.json` / `.cursor/mcp.json` / `opencode.json` / `.grok/config.toml` spawn `target/live/gsv-mcp.exe` — **✅** |
| **PH-S2222** | HTTP CSRF | POST `/mcp` skips Origin / `Sec-Fetch-Site` (bots); body cap stays; other POSTs gated — **✅** |
| **PH-S2223** | `gsv_xtask` sync | MCP `task=sync` is `--check` only; remirror stays `gsv_vision_sync` — **✅** |
| **PH-S2224** | Resource notify | `gsv_vision_sync` notifies **every** subscribed `gsv://` URI — **✅** |
| **PH-S2225** | Discovery + card | `stdio_live` + `http_csrf` on `GET /mcp`; Galaxy card lists live path — **✅** |
| **PH-S2226** | Contracts | mcp/xtask/watchdog/security/ui client-config tests — **✅** |
| **PH-S2227** | Docs | MCP_OPENBOT / RUST_DEV / SERVER / BOXES / HANDOFF / NEXT / MEMORY — **✅** |
| **PH-S2228** | Band close | tests green; `--stretch-96` ≥96%; `--band 158` + fingerprint; one commit + push — **✅** |

## Спринти (band 159) — Cursor HTTP MCP + session SSE hold ✅

Owner ask (`абракадабра` gsv / mvp with mcp): bot still not in the Cursor agent toolkit. Scan: live `gsv-server` was **0.152** (32 tools) while crate **0.158** (36 tools); `.cursor/mcp.json` spawned stdio `gsv-mcp` (second AppState, Cursor catalog empty); GET `/mcp` with `Accept: text/event-stream` was a finite flush even with a session (Cursor Streamable HTTP drops).

| Sprint | Фокус | Acceptance (ключ) |
|--------|-------|-------------------|
| **PH-S2229** | Scope + queue | this band; `active_sprint` = `PH-S2229` — **✅** |
| **PH-S2230** | Cursor HTTP | `.cursor/mcp.json` `url` = `http://127.0.0.1:9999/mcp`; stdio stays `.mcp.json` / OpenCode / Grok — **✅** |
| **PH-S2231** | Discovery | `GET /mcp` `version` + `http_url`; Galaxy card lists both — **✅** |
| **PH-S2232** | Session SSE | GET SSE **with** `Mcp-Session-Id` holds the stream; sessionless GET stays finite flush — **✅** |
| **PH-S2233** | Instructions | `initialize` + `gsv_drain` name the HTTP URL and stale-live check — **✅** |
| **PH-S2234** | (reserve) | — **✅** |
| **PH-S2235** | Contracts | mcp/ui client-config + hold-stream tests — **✅** |
| **PH-S2236** | Docs | MCP_OPENBOT / SERVER / HANDOFF / NEXT / MEMORY — **✅** |
| **PH-S2237** | Live lockstep | recopy `gsv-server` + `gsv-mcp` after tests (do not kill live before `cargo test`) — **✅** |
| **PH-S2238** | Band close | tests **473** green; `--stretch-96` **99.23%**; `--band 159` + fingerprint; one commit + push — **✅** |

## Спринти (band 160) — GSV sandbox MCP, no User leak ✅

Owner: User MCP overlay made `gsv_mcp_openbot` appear in PoolAI windows. GSV sandbox is `S:/rust/GSV`; VDT products stay on `gsv_products_*` allowlist; mutating open/apply/tunnel stay off MCP.

| Sprint | Фокус | Acceptance (ключ) |
|--------|-------|-------------------|
| **PH-S2239** | Scope | this band; drop `%USERPROFILE%/.cursor/mcp.json` overlay — **✅** |
| **PH-S2240** | Discovery | `GET /mcp` `sandbox` = GSV crate path — **✅** |
| **PH-S2241** | Instructions | `gsv_drain` names GSV sandbox + no User MCP — **✅** |
| **PH-S2242** | Confine | preview `../poolAI/…` is tool error; no `gsv_products_open` / tunnel / apply tools — **✅** |
| **PH-S2243** | Cursor | project `.cursor/mcp.json` `type=http` loopback only (folder GSV) — **✅** |
| **PH-S2244** | Galaxy card | sandbox kbd — **✅** |
| **PH-S2245** | (reserve) | — **✅** |
| **PH-S2246** | Contracts | mcp/ui sandbox + omit-mutating — **✅** |
| **PH-S2247** | Docs | MCP_OPENBOT / HANDOFF / NEXT / MEMORY — **✅** |
| **PH-S2248** | Band close | tests **475** green; `--stretch-96` **99.23%**; `--band 160` + fingerprint; one commit + push — **✅** |

## Спринти (band 161) — vision lockstep + S0 disk ✅

Owner pick (`абракадабра` gsv / lockstep): Galaxy queue stuck on PH-S2229 after bands 159–160; `gsv_disk` showed `0 GiB` when 503 MiB remained; no keep-live `cargo xtask disk --clean`.

| Sprint | Фокус | Acceptance (ключ) |
|--------|-------|-------------------|
| **PH-S2249** | Scope + queue | this band; `active_sprint` / `next_sprint` = `PH-S2249` — **✅** |
| **PH-S2250** | Disk MiB | `free_mb` / `target_mb`; sub-GiB notes say MiB not `0 GiB` — **✅** |
| **PH-S2251** | `disk --clean` | drop `debug/deps`+incremental; **never** `target/live` — **✅** |
| **PH-S2252** | Vision lockstep | `last_sprint_closed` = `PH-S2248` — **✅** |
| **PH-S2253** | Contracts | xtask unit + `/api/disk` `free_mb` + vision PH-S2249 — **✅** |
| **PH-S2254** | MCP | `gsv_disk` stays read-only; `clean` is not an MCP task — **✅** |
| **PH-S2255** | Docs | RUST_DEV / HANDOFF / NEXT / MEMORY / MCP_OPENBOT — **✅** |
| **PH-S2256** | (reserve) | — **✅** |
| **PH-S2257** | Ratio hold | `--stretch-96` ≥96%; fmt/clippy — **✅** |
| **PH-S2258** | Band close | tests **481** green; `--stretch-96` **99.25%**; `--band 161` + fingerprint; one commit + push — **✅** |

## Спринти (band 162) — live crate/version lockstep ✅

Owner pick (`абракадабра` gsv / live-lockstep): live `gsv-server` was **0.160.0** while crate **0.161.0**; `gsv_update.update_available` true (src mtime) but `gsv_health.update_available` false (notify flag only); MCP `gsv_disk` missed `free_mb` because live MCP lagged.

| Sprint | Фокус | Acceptance (ключ) |
|--------|-------|-------------------|
| **PH-S2259** | Scope + queue | this band; `active_sprint` / `next_sprint` = `PH-S2259`; `last_sprint_closed` = `PH-S2258` — **✅** |
| **PH-S2260** | Update wire | `crate_version` / `version_lag`; lag or src mtime → `update_available` — **✅** |
| **PH-S2261** | Health wire | same `effective_available` as Update; expose crate vs running — **✅** |
| **PH-S2262** | Watchdog lockstep | `debug_newer_than_live`; POST `/api/update/apply` when healthy + debug newer (cooldown); miss path still recopies — **✅** |
| **PH-S2263** | MCP | `GET /mcp` + `gsv_health` `crate_version` / `version_lag`; drain prompt names lag — **✅** |
| **PH-S2264** | Galaxy cards | health / update / watchdog / mcp show crate + lag / `debug_newer` — **✅** |
| **PH-S2265** | Contracts | update/health/watchdog/mcp/vision PH-S2259 — **✅** |
| **PH-S2266** | Docs | SERVER / BOXES / RUST_DEV / HANDOFF / NEXT / MEMORY / MCP_OPENBOT — **✅** |
| **PH-S2267** | Ratio hold | `--stretch-96` ≥96%; fmt/clippy — **✅** |
| **PH-S2268** | Band close | tests **485** green; `--stretch-96` **99.25%**; `--band 162` + fingerprint; recopy live; one commit + push — **✅** |

## Спринти (band 163) — vision queue lockstep + bump auto-advance ✅

Owner pick (`абракадабра` gsv / vision-lockstep): Galaxy queue stayed on `PH-S2259` / last `PH-S2258` after band **162** closed `PH-S2268`. `cargo xtask bump --band N` now locksteps source vision JSON so the next drain does not stick.

| Sprint | Фокус | Acceptance (ключ) |
|--------|-------|-------------------|
| **PH-S2269** | Scope + queue | this band; `active_sprint` / `next_sprint` = `PH-S2269`; `last_sprint_closed` = `PH-S2268` — **✅** |
| **PH-S2270** | Vision lockstep files | `docs/vision/manifest.json` + `extensions.json` — **✅** |
| **PH-S2271** | Band math | `band_first_sprint` / `band_last_sprint` (origin band 102 = PH-S1659) — **✅** |
| **PH-S2272** | JSON patch | `replace_json_string_field` keeps surrounding text (no pretty rewrite) — **✅** |
| **PH-S2273** | `lockstep_queue_for_band` | patches last / next / active for band N — **✅** |
| **PH-S2274** | `cargo xtask bump` | after semver, lockstep vision queue; catalog help names lockstep — **✅** |
| **PH-S2275** | Contracts | vision PH-S2269 + xtask bump help + fingerprint queue ids — **✅** |
| **PH-S2276** | Docs | RUST_DEV / BOXES / SERVER / HANDOFF / NEXT / MEMORY / roadmap — **✅** |
| **PH-S2277** | Ratio hold | `--stretch-96` ≥96%; fmt/clippy — **✅** |
| **PH-S2278** | Band close | tests **489** green; `--stretch-96` **99.26%**; `--band 163` + fingerprint; one commit + push — **✅** |

## Спринти (band 164) — Cursor 3.16.29 kit lockstep ✅

Owner pick (`абракадабра` gsv / Cursor version update): desktop jumped **3.13.21 → 3.16.29**. Scan: folder MCP `type:http` + live `:9999/mcp` still works (36 tools); User MCP still absent; vision drift ok; kit pin and toolchain inventory lagged.

| Sprint | Фокус | Acceptance (ключ) |
|--------|-------|-------------------|
| **PH-S2279** | Scope | this band; `active_sprint` / `next_sprint` = `PH-S2279`; `last_sprint_closed` = `PH-S2278` — **✅** |
| **PH-S2280** | Probe | Cursor `package.json` version **3.16.29**; toolchain `cursor` entry — **✅** |
| **PH-S2281** | Rules | `.cursor/rules/cursor-environment-baseline.mdc` pin + MCP/sync notes — **✅** |
| **PH-S2282** | MCP | keep folder `type:http` loopback; drain prompt names 3.16 / type=http; never User; no Origin-host — **✅** |
| **PH-S2283** | Tools | `gsv_health` / `gsv_xtask` / `gsv_watchdog` / `gsv_products` still match Cursor 3.16 Streamable HTTP — **✅** |
| **PH-S2284** | Sync | `gsv_xtask` `{task:sync}` `--check`; remirror `gsv_vision_sync`; `cargo xtask mirrors` — **✅** |
| **PH-S2285** | Contracts | baseline 3.16.29 · MCP folder HTTP · toolchain cursor · queue PH-S2279 — **✅** |
| **PH-S2286** | Docs | BOXES / ARCHITECTURE / MCP_OPENBOT / HANDOFF / NEXT / MEMORY / roadmap — **✅** |
| **PH-S2287** | Ratio hold | `--stretch-96` ≥96%; fmt/clippy — **✅** |
| **PH-S2288** | Band close | tests **496** green; `--stretch-96` **99.26%**; `--band 164` + fingerprint; one commit + push — **✅** |

## Спринти (band 165) — watchdog live copy + lockstep observability ✅

Owner pick (`абракадабра` gsv / scan): live crate **0.164.0** vs running **0.163.0**, `debug_newer=true`, but heartbeat stayed `probe-ok`. Watchdog spawned from `target/debug` (locks cargo) and swallowed non-`applying` apply responses. `--once` skipped lockstep.

| Sprint | Фокус | Acceptance (ключ) |
|--------|-------|-------------------|
| **PH-S2289** | Scope | this band; `active_sprint` / `next_sprint` = `PH-S2289`; `last_sprint_closed` = `PH-S2288` — **✅** |
| **PH-S2290** | Heartbeat | `last_apply_status` + `lockstep_note`; old JSON still deserializes — **✅** |
| **PH-S2291** | Apply visibility | `lockstep-fail` never silent `probe-ok`; `--once` locksteps; peer oneshot apply — **✅** |
| **PH-S2292** | Probe | health `version_lag` also locksteps; POST `Origin` loopback — **✅** |
| **PH-S2293** | Live copy | `copy_debug_to_live` copies `gsv-watchdog`; `cargo xtask watchdog` / install spawn live — **✅** |
| **PH-S2294** | Galaxy card | watchdog rows `last_apply_status` / `lockstep_note`; warn on fail — **✅** |
| **PH-S2295** | Contracts | heartbeat compat · oneshot · spawn-exe · render 403 note — **✅** |
| **PH-S2296** | Docs | BOXES / SERVER / RUST_DEV / MCP / HANDOFF / NEXT / MEMORY / roadmap — **✅** |
| **PH-S2297** | Ratio hold | `--stretch-96` ≥96%; fmt/clippy — **✅** |
| **PH-S2298** | Band close | tests **503** green; `--stretch-96` **99.27%**; `--band 165` + fingerprint; one commit + push — **✅** |

## Спринти (band 166) — GSV settings + Godfather store ✅

Owner pick 2026-08-19: Settings card; Godfather channel + token store; co-workflows; later Telegram + ticket board + MCP bus. Spec: [`GSV_SETTINGS_TELEGRAM.md`](./GSV_SETTINGS_TELEGRAM.md). Plan: [`docs/superpowers/plans/2026-08-19-gsv-settings-telegram-tickets.md`](../superpowers/plans/2026-08-19-gsv-settings-telegram-tickets.md). **This spec is complete (166–169 ✅).** Next drain = owner pick. Do not invent 170.

| Sprint | Фокус | Acceptance (ключ) |
|--------|-------|-------------------|
| **PH-S2299** | Scope | this band; `active_sprint` / `next_sprint` = `PH-S2299`; `last_sprint_closed` = `PH-S2298` — **✅** |
| **PH-S2300** | Schema + disk | `data/gsv_settings.json`; env `GSV_TELEGRAM_BOT_TOKEN` wins; missing file empty-ok — **✅** |
| **PH-S2301** | HTTP | `GET`/`POST /api/settings`; wire never contains `bot_token`; CSRF loopback — **✅** |
| **PH-S2302** | Galaxy card | `settings` in `CARD_NAMES`; empty/error HTML — **✅** |
| **PH-S2303** | MCP | `gsv_settings` read; `gsv://docs/settings-telegram`; drain prompt names 166 — **✅** |
| **PH-S2304** | Contracts | redaction · env override · mcp unit + `gsv_mcp_contracts` — **✅** |
| **PH-S2305** | Docs | BOXES / SERVER / MCP / HANDOFF / NEXT / MEMORY / spec Landed 166 — **✅** |
| **PH-S2306** | Ratio hold | `--stretch-96` ≥96%; fmt/clippy — **✅** |
| **PH-S2307** | Tests | `cargo test` green; live copy stays — **✅** |
| **PH-S2308** | Band close | bump `--band 166` + fingerprint; one commit + push; next = **167** — **✅** |

## Спринти (band 167) — Telegram Godfather channel bind ✅

Owner 2026-08-19: remaining plan **168–169 fully specified**. Next `абракадабра` gsv = **168 only**. Plan: [`docs/superpowers/plans/2026-08-19-gsv-settings-telegram-tickets.md`](../superpowers/plans/2026-08-19-gsv-settings-telegram-tickets.md). Dry-run / `X-Telegram-Dry-Run`; poller default off; no bus.

| Sprint | Фокус | Acceptance (ключ) |
|--------|-------|-------------------|
| **PH-S2309** | Scope | bind only; no `gsv_telegram_bus_*`; no `tickets.jsonl` — **✅** |
| **PH-S2310** | Probe | `boxes/telegram.rs` getMe+getChat; cargo-test stub; token stripped from errors — **✅** |
| **PH-S2311** | HTTP | `GET /api/telegram` redacted status; `X-Telegram-Dry-Run: 1` — **✅** |
| **PH-S2312** | Poller | default off; `telegram-relay` or `godfather.poll` only — **✅** |
| **PH-S2313** | Galaxy | card `telegram`; `CARD_NAMES` 39; empty/error HTML — **✅** |
| **PH-S2314** | MCP | `gsv_telegram` read; drain prompt names 167 landed / next 168 — **✅** |
| **PH-S2315** | Contracts | no sockets in tests; mcp/ui lockstep — **✅** |
| **PH-S2316** | Docs | BOXES / SERVER / MCP / HANDOFF / NEXT / spec Landed 167 — **✅** |
| **PH-S2317** | Ratio + tests | fmt/clippy/`cargo test`/`--stretch-96`; keep live — **✅** |
| **PH-S2318** | Band close | `--band 167` + fingerprint; next = **168** — **✅** |

## Спринти (band 168) — ticket board + MCP claim ✅

| Sprint | Фокус | Acceptance (ключ) |
|--------|-------|-------------------|
| **PH-S2319** | Scope | sibling `ticket_claims.jsonl`; no Telegram create-ticket — **✅** |
| **PH-S2320** | JSONL | `docs/gsv/tickets.jsonl` + claims; missing file empty-ok — **✅** |
| **PH-S2321** | HTTP | GET list; POST create; POST claim; CSRF; workflow `ticket-claim` — **✅** |
| **PH-S2322** | Galaxy | card `tickets`; `CARD_NAMES` 40 — **✅** |
| **PH-S2323** | MCP | `gsv_tickets` + `gsv_tickets_claim`; unknown id → tool error — **✅** |
| **PH-S2324** | Claim row | append claims JSONL; `open`→`in_progress` + `claimed_by` — **✅** |
| **PH-S2325** | Contracts | claim round-trip; CSRF; no secrets — **✅** |
| **PH-S2326** | Docs | BOXES / spec Landed 168; next = **169** — **✅** |
| **PH-S2327** | Ratio + tests | fmt/clippy/`cargo test`/`--stretch-96` — **✅** |
| **PH-S2328** | Band close | `--band 168` + fingerprint + push — **✅** |

## Спринти (band 169) — Telegram bus between MCP bots · ✅

| Sprint | Фокус | Acceptance (ключ) |
|--------|-------|-------------------|
| **PH-S2329** | Scope | no webhook; no Cloudflare; no invent 170 — **✅** |
| **PH-S2330** | Envelope | `{v:1,kind:bus,…}`; dry-run in-memory queue — **✅** |
| **PH-S2331** | Gates | `telegram-relay`; allowlist; body cap; rate-limit — **✅** |
| **PH-S2332** | MCP | `gsv_telegram_bus_send` / `gsv_telegram_bus_poll` — **✅** |
| **PH-S2333** | HTTP | `/api/telegram/bus`; CSRF on POST — **✅** |
| **PH-S2334** | Card + tests | two dry-run messages; token redact — **✅** |
| **PH-S2335** | Docs | spec P2 Landed; this plan complete — **✅** |
| **PH-S2336** | Ratio | `--stretch-96` ≥96%; fmt/clippy — **✅** |
| **PH-S2337** | Tests | `cargo test`; keep live — **✅** |
| **PH-S2338** | Band close | `--band 169` + fingerprint; NEXT = owner pick — **✅** |

## Ключові UX-вимоги (узагальнення ТЗ)

1. Оновлюємо/дебажимо vision Rust-кодбазу, запущена **bin-версія** → сервер приймає **повідомлення про апдейт**.
2. Перекомпіляція на новий бінарник → у UI **«Update» замість reload**.
3. Вебсторінка **не падає** при офлайн — просто переходить в offline.
4. Після реконекту — **всі метрики синхронізуються** (resync).
5. Tracker показує технічні параметри воркфлоу, що виконувалось.
6. SLI console показує команди + усі SLI-функції з наявних скриптів (+ нові).
7. Toolchain показує, які тули використовуються.
8. IDE — портовані opencode + cursor чати; вибір, з чим працювати.
9. Box preview — Rust-кольори відповідно до синтаксису.
10. SLI terminal — щоб AI міг посилати команди.
11. Rust tests/benchmarks — хук **без перекомпіляції**.

## Посилання

- Бокси: [`GSV_BOXES.md`](./GSV_BOXES.md)
- Сервер: [`GSV_SERVER.md`](./GSV_SERVER.md)
- Архітектура: [`GSV_ARCHITECTURE.md`](./GSV_ARCHITECTURE.md)
- Міграція: [`GSV_MIGRATION.md`](./GSV_MIGRATION.md)
- MCP horizon: [`GSV_MCP_OPENBOT.md`](./GSV_MCP_OPENBOT.md)
- FM §5.12 band 102: [`../../docs/catalog/FUNCTION_MANAGEMENT.md`](../../docs/catalog/FUNCTION_MANAGEMENT.md)
