# GSV after always-on — MCP catch-up (conception)

**Status:** Landed (band **156** ✅ streaming usage + VDT git + owner tunnel · band **155** ✅ session token usage · band **154** ✅ watchdog ops card + fingerprint model)  
**Date:** 2026-08-18  
**Owner ask:** update needed docs/plans; put what the next `абракадабра` / `abrakadabra` on **gsv** must drain.  
**Scan (same day):** clippy 0 · roadmap 102–156 all ✅ · no `TODO` in `src/` · always-on P2 leftovers closed. Streaming token record **done band 156**. Grok Bot tunnel is `cargo xtask tunnel` (owner opt-in).

**Plan:** [`docs/superpowers/plans/2026-08-18-mcp-always-on-catchup.md`](../superpowers/plans/2026-08-18-mcp-always-on-catchup.md)  
**Roadmap:** band **152** (then 153) in [`GSV_TECH_ROADMAP.md`](./GSV_TECH_ROADMAP.md)  
**Closed horizon:** [`GSV_ALWAYS_ON_UI.md`](./GSV_ALWAYS_ON_UI.md) bands **143–150 ✅**

## Problem statement

Always-on Galaxy is the live product: `:9999` stays up across `cargo test` (live copy + watchdog), the page goes offline only during binary swap, and ops cards exist for **products**, **fingerprints**, **sw**, and **watchdog** (health row).

`gsv_mcp_openbot` wraps those boxes as of band **155**: **35 tools**, **9 `gsv://` resources**, **3 prompts**. Session token usage is `gsv_usage` / `GET /api/usage` (OmniRouter + MCP + OmniRoute, mirrored on vision-sync). Product tests/benches/scripts are `cargo xtask` — [`GSV_RUST_DEV.md`](./GSV_RUST_DEV.md).

Cost of leaving it: the next drain session invents work, or the agent repeats a warnings-first scan with no queued band. This spec is the queued band.

## Goals

1. Next `абракадабра` on **gsv** drains **band 152** (MCP `products_select`) after S0 + warnings-first scan.
2. MCP tools wrap the always-on HTTP wires that already exist (`products::wire` / `scan`, `watchdog::wire`, `sw::wire`, `fingerprint::wire`).
3. Discovery (`GET /mcp`, Galaxy MCP card, `tools/list`) reports the new `tool_count` from `TOOL_NAMES` (no hardcoded 26).
4. `gsv_drain` prompt text tells the agent to call the new tools before proposing PH-S*.
5. Ratio stays `gsv-loc-audit --stretch-96` ≥ 96%. No Python. No LAN widen.

## Non-goals (band 151)

- **No** MCP `products/open` (spawning Explorer/Cursor from an agent is surprising; HTTP POST stays).
- **No** MCP `products/select` (process-local mutation) — band **152**.
- **No** MCP `update/apply` (kills the live process).
- **No** Grok Bot public tunnel (still owner opt-in).
- **No** wasm / `poolai-ui-wasm`.
- **No** dedicated watchdog ops card (health row is enough until band **153**).
- **No** inventing product boxes that do not already have HTTP wires.

## User stories

- As owner, I type `abrakadabra`, pick **GSV**, and the agent drains **band 152** from this spec after clippy is clean.
- As an OpenCode/Cursor agent, I call `gsv_products` and `gsv_products_scan` with `id=gsv` and see the same rows as `GET /api/products` / `scan`.
- As an agent, I call `gsv_watchdog` and know whether `target/live/watchdog.json` is fresh before I assume `:9999` is supervised.
- As an agent, I call `gsv_fingerprints` and `gsv_sw` instead of guessing from README.
- As owner, I still use Galaxy cards for select/open; MCP does not open folders.

## Requirements

### P0 — Must (band 151, `PH-S2149…S2158`)

**MCP tools (26 → 31)** — wrap existing boxes; same JSON as HTTP; secrets stay redacted.

| Tool | Wraps | Args | Error |
|------|-------|------|--------|
| `gsv_products` | `products::wire(repo, selected)` | none | none (empty list is ok) |
| `gsv_products_scan` | `products::scan(repo, id)` | **required** `id` | unknown id → tool error (same idea as HTTP 404) |
| `gsv_watchdog` | `watchdog::wire(repo)` | none | none (`alive: false` if no heartbeat) |
| `gsv_sw` | `sw::wire()` | none | none |
| `gsv_fingerprints` | `fingerprint::wire(repo, limit)` | optional `limit` (default 20, clamp 1–100 via `clamp_limit`) | none |

**MCP resources (6 → 8)** — same `preview::resolve` confine (`..` / `file:` / unknown → `-32602`).

| URI | File |
|-----|------|
| `gsv://docs/fingerprints` | `docs/gsv/fingerprints.jsonl` |
| `gsv://docs/post-always-on` | `docs/gsv/GSV_POST_ALWAYS_ON.md` |

**Prompt:** `gsv_drain` text must name `gsv_products`, `gsv_products_scan`, `gsv_watchdog`, and `gsv://docs/next` (still no mid-drain push; MSYS2 bash).

**Discovery:** `TOOL_NAMES.len() == 31`; `RESOURCE_URIS.len() == 8`; Galaxy `render_mcp` already uses `tool_count` from the wire — do not hardcode 26 in `src/mcp.rs` unit tests or `tests/gsv_mcp_contracts.rs`.

**Docs drift from the 2026-08-18 scan** (land in band 151 docs task, or already in this queue commit):

- `GSV_TECH_ROADMAP.md` header lists bands **149** and **150**.
- `GSV_ARCHITECTURE.md` has a `watchdog/` row next to `sw/`.
- `.cursor/commands/git-push.md` is tracked (PH-S1921 leftover).

### P1 — Should (band 152) ✅

- MCP `gsv_products_select` `{id}` — same allowlist as `POST /api/products/select`; unknown id → tool error.
- `gsv_products_scan` may omit `id` when a product is already selected on `AppState`.
- `gsv_drain` prompt: call scan on the selected id, then read HANDOFF/NEXT resources.
- Do **not** add `gsv_products_open` unless the owner asks.

### P2 — Later (band 155+)

- Grok Bot tunnel runbook (owner opt-in only).
- wasm 0–5% horizon unchanged.

## Success metrics

- Next gsv drain session implements band 151 from the plan; clippy 0; `cargo test` green; `gsv-loc-audit --stretch-96` ≥ 96%.
- `tools/list` length **31**; `resources/list` length **8**.
- `gsv_products_scan` with `id=gsv` returns `ok` + `git_head`; with `id=nope` is an MCP tool error (not a 26-tool “unknown tool”).
- Stand-smoke `/mcp` discovery still 200; Galaxy MCP card shows the new count.

## Open questions (non-blocking)

- Whether `gsv_products_open` ever belongs on MCP — default **no**.

## Phasing (VDT bands)

| Band | PH-S* | Focus | When |
|------|-------|--------|------|
| **this session** | — | Spec + plan + HANDOFF/NEXT/MEMORY · **no product code** | 2026-08-18 |
| **151** | S2149–S2158 | MCP catch-up: 5 tools + 2 resources + `gsv_drain` text + contracts | **✅ 2026-08-18** |
| **152** | S2159–S2168 | MCP `products_select` + scan-without-id | **✅ 2026-08-18** |
| **153** | S2169–S2178 | rust-first `cargo xtask` | **✅ 2026-08-18** |
| **154** | S2179–S2188 | Watchdog ops card + fingerprint model (owner pick) | **✅ 2026-08-18** |

## Constraints

- MSYS2 bash; Rust 95–100% / wasm 0–5%; thin `ui/index.html` glue.
- CSRF + loopback POSTs (band 133); CSP / no-store (band 134). MCP stdio is not CSRF; HTTP `/mcp` stays loopback unless `--allow-lan`.
- Do not kill `target/live/` before `cargo test`.
- Never stage `data/*`, `.env*`, `*.pem`, `comitmsg/*` except `comitmsg/README.md`.
- One commit per drain; push last. Drain close: `gsv-bump-version.sh --band 151` then `gsv-fingerprint.sh`.
