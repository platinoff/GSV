# GSV Always-on Galaxy UI — spec

**Status:** Accepted (plan queued; implementation starts band 143)  
**Date:** 2026-08-17  
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
5. Ops card: list environment projects (same merge as `scripts/list-vdt-products.sh`), select one, open its folder (Windows Explorer, confined), auto-parse git/kind/HANDOFF/Cargo name.
6. Each product commit increments `CARGO_PKG_VERSION` (patch). Health / Update / header show it.
7. Append-only fingerprint (timestamp, actor, IDE, model, agent, version, git head, summary) on drain close; Galaxy card lists latest.

## Non-goals

- No LAN widen (`127.0.0.1` default; `--allow-lan` unchanged).
- No copy of `docs/vision/vision.js`.
- No Python. No `git add -A`. No staging `data/*`.
- No public telemetry; fingerprints stay in this repo.
- No auto `cargo test` when a product is selected (scan is metadata only).
- OmniRoute registration in `PRODUCTS.md` stays owner-opt-in.

## Current bugs (reproduce)

| ID | Where | What happens |
|----|--------|----------------|
| P1 | `ui/index.html` L11 vs L65 | `header{z-index:5}` then `body>header,.workspace{z-index:2}` — menu paints under cards |
| P2 | `.card.collapsed .body{display:none}` | Card chrome still in the grid; dock chips exist but the hole remains |
| P3 | fullscreen click | Second card can also get `.fullscreen`; both shown under `panel-fs-active` |
| P4 | Esc | `.actions button:last-child` is brittle |
| P5 | `doUpdate()` | Offline forever until manual restart; no apply/restart API |
| P6 | `cargo test` | Locks running `target/debug/gsv-server.exe` (Windows) |

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

- `GET /api/products` — same rows as `list-vdt-products.sh` (workspace folders ∪ sibling git ∪ kit).
- `POST /api/products/select` `{id}` — process-local selection on `AppState`.
- `POST /api/products/open` `{id}` — open folder; path must be a discovered root (no `..`).
- `GET /api/products/scan` — selected product: `git_head`, `git_status_short`, `kind`, `registered`, HANDOFF/NEXT path exists, `cargo_name` if `Cargo.toml`.
- Galaxy card `products` in **ops**.

**Version + fingerprint**

- Drain-close bump: patch +1 in `Cargo.toml` (`0.1.0` → `0.1.1` …). Tests compare `env!("CARGO_PKG_VERSION")`, not a hardcoded `"0.1.0"`.
- `docs/gsv/fingerprints.jsonl` append-only; card `fingerprints` in ops.
- Commit trailers: `Gsv-Actor`, `Gsv-Ide`, `Gsv-Model` (no secrets).

### P1 — Should

- Open folder: prefer `cursor <path>` if on PATH, else `explorer.exe`.
- Header meta shows `v{version}` (already) plus last fingerprint actor/ide (short).
- Chart cards: `max-height` none when fullscreen.

### P2 — Later

- Service Worker offline cache (mentioned in GSV_SERVER.md, not required for 143–147).
- Auto-register omniroute in PRODUCTS.md.
- Semver minor = band number.

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

- Fingerprint `model` string: Cursor session vs env. Default `unknown` if missing.
- Supervisor: `scripts/gsv-live.sh` vs a `gsv-live` bin. **Decision:** bash supervisor first (Windows-friendly copy+exec); bin only if bash restart is flaky.

## Phasing (VDT bands)

| Band | PH-S* | Focus |
|------|-------|--------|
| **143** | S2069–S2078 | Chrome bugs + type/chart scale (debug first) |
| **144** | S2079–S2088 | Live copy + apply + offline-during-swap |
| **145** | S2089–S2098 | Products list / select / open / scan |
| **146** | S2099–S2108 | Version bump + fingerprints |
| **147** | S2109–S2118 | README-level UI polish leftovers + docs canon |

Next `абракадабра` on **gsv** drains **band 143** only (≤10 open PH-S*).
