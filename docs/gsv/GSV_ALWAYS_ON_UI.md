# GSV Always-on Galaxy UI — spec

**Status:** Bands **143–147 ✅** (always-on Galaxy) · **148 ✅** (Service Worker shell cache) · **149 ✅** (omniroute PRODUCTS.md + semver minor = band) · **150 ✅** (live watchdog)  
**Date:** 2026-08-18  
**Owner ask:** Galaxy UI always reachable; page goes **offline** only during binary swap; debug collapse / fullscreen / power menu; typography + chart scale; pick a VDT project and open its folder; auto-parse what the dashboard needs; every commit bumps the crate version; fingerprint who did the work (IDE / bot / model / agent / time). Match the polish of [`README.md`](../../README.md) presentations.

**Plan:** [`docs/superpowers/plans/2026-08-17-always-on-galaxy.md`](../superpowers/plans/2026-08-17-always-on-galaxy.md)  
**Roadmap:** bands **143–147** in [`GSV_TECH_ROADMAP.md`](./GSV_TECH_ROADMAP.md)

## Problem

`gsv-server` on `:9999` is the live product. Today the process **locks** `target/debug/gsv-server.exe`, so drain docs say “stop the server before `cargo test` / `build`”. The page then dies. `doUpdate()` only toasts “restart gsv-server” and forces offline with no automatic come-back.

Chrome bugs fight the README look:

- Power menu sits **under** cards (`body>header` and `.workspace` share `z-index: 2`; workspace paints later).
- Collapse only hides `.body`; the card still occupies the grid.
- Fullscreen is not exclusive (two `.fullscreen` cards + `panel-fs-active` overlap). Esc targets `.actions button:last-child`.
- Type and SVG chart sizes (10–12px mono, card `max-height: 340px`) are smaller and less balanced than the presentation shots.

There is an **IDE session** picker, not a **VDT product** picker. `Cargo.toml` is stuck at `0.1.0`. Commits have no durable “who / which client / which model” record.

## Goals

1. Process on `:9999` stays up across `cargo test` / `cargo build` (run a **live copy**, not the file cargo overwrites).
2. During swap the UI badge is **offline**; after the new process binds, SSE `onopen` → full resync → **online**.
3. Collapse, exclusive fullscreen, Esc, and the power menu behave correctly and stay above cards.
4. One type scale for chrome, cards, and Rust SVG charts.
5. Ops card: list environment projects (same merge as `cargo xtask products`), select one, open its folder (Windows Explorer, confined), auto-parse git/kind/HANDOFF/Cargo name.
6. Each product commit sets `CARGO_PKG_VERSION` minor to the band (`0.{band}.0`). Health / Update / header show it.
7. Append-only fingerprint (timestamp, actor, IDE, model, agent, version, git head, summary) on drain close; Galaxy card lists latest.

## Non-goals

- No LAN widen (`127.0.0.1` default; `--allow-lan` unchanged).
- No copy of `docs/vision/vision.js`.
- No Python. No `git add -A`. No staging `data/*`.
- No public telemetry; fingerprints stay in this repo.
- No auto `cargo test` when a product is selected (scan is metadata only).
- OmniRoute registration in `PRODUCTS.md` is owner-opt-in (**done band 149**).

## Current bugs (reproduce)

| ID | Where | What happens |
|----|--------|----------------|
| P1 | `ui/index.html` L11 vs L65 | `header{z-index:5}` then `body>header,.workspace{z-index:2}` — menu paints under cards — **fixed band 143** |
| P2 | `.card.collapsed .body{display:none}` | Card chrome still in the grid; dock chips exist but the hole remains — **fixed band 143** |
| P3 | fullscreen click | Second card can also get `.fullscreen`; both shown under `panel-fs-active` — **fixed band 143** |
| P4 | Esc | `.actions button:last-child` is brittle — **fixed band 143** |
| P5 | `doUpdate()` | Offline forever until manual restart; no apply/restart API — **fixed band 144** |
| P6 | `cargo test` | Locks running `target/debug/gsv-server.exe` (Windows) — **fixed band 144** (`gsv-live`; Rust bin as of band 153) |

## Requirements

### P0 — Must

**Always-on live binary**

- Supervisor copies `target/debug/gsv-server.exe` → `target/live/gsv-server.exe` and execs the copy on `127.0.0.1:9999`.
- `cargo build` / `cargo test` may overwrite `target/debug/` without killing the live process.
- `POST /api/update/apply` (CSRF-gated like other POSTs): emit SSE `offline`, exit so the supervisor can recopy + restart.
- UI: Update badge → apply → `setOffline(true)` → wait SSE `onopen` → `resync()` → online.
- Drain docs (AGENTS / HANDOFF / NEXT) stop saying “kill gsv-server before cargo test” once the live copy is the canon process.

**Chrome**

- Power menu stacking context above `.workspace` (header `z-index` ≥ 40; menu `z-index` ≥ 50; `overflow: visible`).
- Collapse: card `display:none` (or equivalent) + dock chip; restore from dock.
- Fullscreen: at most one card; previous FS cleared; Esc restores that card’s `□` via `data-action='card-fs'`.
- Collapse while fullscreen: exit FS first, then collapse.

**Offline during update**

- SSE `offline` / `online` (or reuse existing `offline` event + `onopen`).
- Keep last-good card HTML (already `getText` keep-last-good).

**Typography / charts**

- CSS variables: `--fs-ui` 13px, `--fs-card` 12px, `--fs-meta` 11px, `--fs-chart` 11px; card body `max-height` 420px for chart cards.
- Speed + rust SVG title 12→11, footer 10→11, plot height 140→168. Same family as UI (`ui-monospace`, Cascadia, Consolas).

**Products**

- `GET /api/products` — same rows as `cargo xtask products` (workspace folders ∪ sibling git ∪ kit).
- `POST /api/products/select` `{id}` — process-local selection on `AppState`.
- `POST /api/products/open` `{id}` — open folder; path must be a discovered root (no `..`).
- `GET /api/products/scan` — selected product: `git_head`, `git_status_short`, `kind`, `registered`, HANDOFF/NEXT path exists, `cargo_name` if `Cargo.toml`.
- Galaxy card `products` in **ops**.

**Version + fingerprint**

- Drain-close bump: `cargo xtask bump --band N` sets semver **minor = band** (`0.1.3` → `0.149.0`; same band → patch +1). Tests compare `env!("CARGO_PKG_VERSION")`, not a hardcoded version.
- `docs/gsv/fingerprints.jsonl` append-only; card `fingerprints` in ops.
- Commit trailers: `Gsv-Actor`, `Gsv-Ide`, `Gsv-Model` (no secrets).

### P1 — Should

- Open folder: prefer `cursor <path>` if on PATH, else `explorer.exe`.
- Header meta shows `v{version}` (already) plus last fingerprint actor/ide (short).
- Chart cards: `max-height` none when fullscreen — **done band 156** (`.card.fullscreen img`).

### P2 — Later

- Service Worker offline cache — **done band 148** (`GET /sw.js` Rust-rendered; precache `/` + live CSS + galaxy/vision svg; skip `/events` `/mcp`).
- Auto-register omniroute in PRODUCTS.md — **done band 149** (owner-opt-in).
- Semver minor = band number — **done band 149** (`cargo xtask bump --band N`).

## User stories

- As owner, I keep [http://127.0.0.1:9999/](http://127.0.0.1:9999/) open while the agent runs `cargo test`.
- As owner, I see **offline** only while the binary swaps, then cards resync.
- As owner, I collapse a box to the dock and restore it; one box fullscreen; Esc exits; Power menu is clickable.
- As owner, I pick GSV / poolAI / omniroute in the UI and open that folder.
- As owner, after a commit I see version `0.1.N` and a row “cursor · grok-4.6 · orchestrator · time”.

## Success metrics

- `cargo test` with live `target/live/gsv-server.exe` does not fail with os error 5 on `gsv-server.exe`.
- Stand-smoke `:9999` still 200 after a debug rebuild (before apply).
- UI contracts: power-menu z-index rule present; `data-action='card-fs'`; products card in `CARD_NAMES`.
- `gsv-loc-audit --stretch-96` ≥ 96%.
- Fingerprint file grows by one line per drain commit.

## Constraints

- MSYS2 bash; Rust 95–100% / wasm 0–5%; thin `ui/index.html` glue.
- CSRF + loopback POSTs (band 133); CSP / no-store (band 134).
- Open-folder spawn is allowlisted (`explorer` / `cursor`), not a generic shell.
- `target/live/` is gitignored (build artifact).

## Open questions (non-blocking)

- Fingerprint `model` string: Cursor session vs env. **Decision (band 154):** `GSV_MODEL` wins; else `CURSOR_MODEL` / `GSV_SESSION_FILE` JSON; else Cursor `renderer.log` `catalogModelId`; default `unknown` is valid.
- Supervisor: `cargo xtask live` / `gsv-live` bin. **Decision (band 153):** Rust supervisor; `gsv-watchdog` is the outer loop when that process dies (Cursor abort).

## Phasing (VDT bands)

| Band | PH-S* | Focus |
|------|-------|--------|
| **143** | S2069–S2078 | Chrome bugs + type/chart scale (debug first) ✅ |
| **144** | S2079–S2088 | Live copy + apply + offline-during-swap ✅ |
| **145** | S2089–S2098 | Products list / select / open / scan | ✅ |
| **146** | S2099–S2108 | Version bump + fingerprints | ✅ |
| **147** | S2109–S2118 | README-level UI polish leftovers + docs canon | ✅ |
| **148** | S2119–S2128 | Service Worker shell cache (`/sw.js` Rust-rendered) | ✅ |
| **149** | S2129–S2138 | OmniRoute PRODUCTS.md + semver minor = band | ✅ |
| **150** | S2139–S2148 | Live watchdog (`gsv-watchdog` probes `/api/health`, respawns live copy) | ✅ |

Always-on horizon **closed**. MCP catch-up **band 151–152 ✅**. Rust-first xtask **band 153 ✅**. Watchdog ops card + fingerprint model **band 154 ✅**. Session token usage **band 155 ✅**. Streaming usage + VDT git + owner tunnel **band 156 ✅**. Omni catalog **band 157 ✅**. Live MCP stdio + sync check **band 158 ✅**. Cursor HTTP MCP + session SSE hold **band 159 ✅**. GSV sandbox MCP / no User leak **band 160 ✅**. Vision lockstep + disk MiB / `--clean` keep-live **band 161 ✅**. Next `абракадабра` on **gsv**: scan first, then owner pick.
