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
