# GSV settings / Telegram / tickets Implementation Plan

> **For agentic workers:** Owner 2026-08-19: bands **166–170** landed; owner pick **171** (ticket lease + stale reclaim) landed this drain. Next `абракадабра` on **gsv** is an **owner pick** after a warnings-first scan. Spec travels with the plan: [`GSV_SETTINGS_TELEGRAM.md`](../gsv/GSV_SETTINGS_TELEGRAM.md).

**Goal:** Queue owner-picked GSV settings (Godfather channel + secret store + co-workflows), then Telegram bind, ticket board with MCP claim, then MCP-to-MCP Telegram bus. Band **166** is settings only so secrets land correctly before any Bot API call.

**Architecture:** New box `settings` (file `data/gsv_settings.json`, gitignored). HTTP `GET`/`POST /api/settings` redacts `bot_token`. MCP `gsv_settings` is read-only in 166. Later boxes `telegram` and `tickets` wrap the same AppState. Tokens never appear in logs, MCP, or git.

**Tech Stack:** Rust 2021, axum, serde_json, existing Galaxy `CARD_NAMES` / `gsv_mcp_openbot`.

**Spec:** [`docs/gsv/GSV_SETTINGS_TELEGRAM.md`](../gsv/GSV_SETTINGS_TELEGRAM.md)

## Global Constraints

- Ratio: `cargo run --bin gsv-loc-audit -- --stretch-96` ≥ 96%; no Python; no `vision.js` port.
- Bind: `127.0.0.1:9999` default; no LAN widen; no Cloudflare on MCP.
- Shell: `C:\msys64\usr\bin\bash.exe -lc '…'`; one `cargo` at a time; `unset CARGO_TARGET_DIR`.
- Do not kill `target/live/gsv-server.exe` before `cargo test`.
- Never stage `data/*`, `.env*`, `*.pem`, `comitmsg/*` except `comitmsg/README.md`.
- Drain: ≤10 PH-S*; **one commit** at band close (`--band N` + fingerprint). No mid-push.

## File map (band 166)

| File | Role |
|------|------|
| `src/boxes/settings.rs` | schema, load/save, redact, `wire` / `wire_post` |
| `src/boxes/mod.rs` | `pub mod settings` |
| `src/server/mod.rs` | `GET`/`POST /api/settings` |
| `src/boxes/ui.rs` | `render_settings` + `CARD_NAMES` |
| `src/mcp.rs` | tool `gsv_settings`; resource `gsv://docs/settings-telegram`; `gsv_drain` text names the spec |
| `tests/gsv_mcp_contracts.rs` + settings unit tests | redaction, missing file empty-ok, POST CSRF |
| `ui/index.html` | `rustCards` +1 if the glue list is explicit |
| docs listed in spec | BOXES / SERVER / MCP / HANDOFF / NEXT / MEMORY / roadmap |

---

# Band 166 — Settings + Godfather store (PH-S2299…S2308)

Landed this `абракадабра` on **gsv**. Do not skip to 167.

### Task 1: Scope + queue (PH-S2299)

- [x] Set vision `active_sprint` / `next_sprint` to `PH-S2299` via bump/sync at close, not in a mid-drain push.
- [x] Confirm spec P0 table still matches (no live Telegram).

### Task 2: Settings schema + disk (PH-S2300)

- [x] `SettingsFile` / `Godfather` / `Workflows` / `Security` with `#[serde(default)]`.
- [x] Path `data/gsv_settings.json`. Missing file → empty defaults, `ok:true`, `token_set:false`.
- [x] Env `GSV_TELEGRAM_BOT_TOKEN` sets `token_set` without writing the env into the file.

### Task 3: HTTP GET/POST redacted (PH-S2301)

- [x] `GET /api/settings` never contains `bot_token`.
- [x] `POST /api/settings` loopback CSRF; stores token; response redacted.
- [x] Unit: round-trip token in file, JSON wire omits it.

### Task 4: Galaxy card (PH-S2302)

- [x] `render_settings`; `CARD_NAMES` includes `settings`.
- [x] Empty + error HTML markers. Stand-smoke card list if required.

### Task 5: MCP read + resource (PH-S2303)

- [x] `gsv_settings` tool; `gsv://docs/settings-telegram`.
- [x] `gsv_drain` prompt: after products scan, read this spec; drain 166 not 167.

### Task 6: Contracts (PH-S2304)

- [x] mcp unit + `gsv_mcp_contracts` + settings tests (redact, env override, unknown POST field ignored).

### Task 7: Docs (PH-S2305)

- [x] BOXES / SERVER / ARCHITECTURE / MCP_OPENBOT / README / this spec **Landed** for 166.

### Task 8: Ratio hold (PH-S2306)

- [x] `cargo fmt -- --check` · `cargo clippy --all-targets` · `--stretch-96` ≥ 96%.

### Task 9: Tests (PH-S2307)

- [x] `cargo test` green. Do not kill live copy.

### Task 10: Band close (PH-S2308)

- [x] `cargo xtask bump --band 166` + fingerprint + one commit + push.
- [x] HANDOFF/NEXT: next = **167** (Godfather bind), not 168.

---

# Band 167 — Godfather channel bind (PH-S2309…S2318)

Landed this `абракадабра` on **gsv**. Do not skip to 168.

## File map (band 167)

| File | Role |
|------|------|
| `src/boxes/telegram.rs` | `TelegramStatus` / probe getMe+getChat; dry-run stub; redact token from errors; optional poll flag (default false) |
| `src/boxes/mod.rs` | `pub mod telegram` |
| `src/boxes/settings.rs` | read `godfather.channel_id` + token (env wins); `workflows.enabled` contains `telegram-relay` → poll allowed |
| `src/server/mod.rs` | `GET /api/telegram` (status). No POST that sends Telegram in 167 unless owner later asks. |
| `src/boxes/ui.rs` | `render_telegram`; `CARD_NAMES` **39** |
| `src/mcp.rs` | tool `gsv_telegram` (read status); `gsv_drain` names **167** not 168 |
| `ui/index.html` | ops card `telegram`; `rustCards` +1 |
| `src/bin/gsv_http_stand_smoke.rs` | card + `/api/telegram` |
| `tests/gsv_telegram_contracts.rs` | dry-run, missing channel `{ok:false}`, token never in JSON/error, CSRF N/A on GET |
| docs | BOXES / SERVER / MCP / HANDOFF / NEXT / MEMORY / spec Landed 167 |

### Task 1: Scope (PH-S2309)

- [x] Confirm spec P1 bind table. No `gsv_telegram_bus_*`. No `tickets.jsonl`.
- [x] `gsv_drain` / HANDOFF still say next-after-167 = **168**.

### Task 2: Probe module (PH-S2310)

- [x] `telegram.rs`: `fn status(data_dir, repo_root) -> Value`. Missing channel or missing token → `{ok:false,error}` without secrets.
- [x] Dry-run / cargo-test stub returns fake `bot_username` + `chat_title`; sets `"dry_run": true`.
- [x] Live path: Bot API **getMe** then **getChat** for `godfather.channel_id`. Timeouts short. Map HTTP/API errors to `{ok:false,error}` with token stripped.

### Task 3: HTTP GET (PH-S2311)

- [x] `GET /api/telegram` → redacted status: `ok`, `channel_id`, `token_set`, `bot_username`, `chat_title`, `last_probe`, `polling` (bool, default false), never `bot_token`.
- [x] Header `X-Telegram-Dry-Run: 1` forces stub even on live server (owner debug).

### Task 4: Poller default off (PH-S2312)

- [x] No background Telegram task unless `workflows.enabled` contains `telegram-relay` **or** settings gain `godfather.poll: true` (`#[serde(default)]`).
- [x] Always-on Galaxy must not probe Telegram on boot when poll is off. Status GET is on-demand.

### Task 5: Galaxy card (PH-S2313)

- [x] `render_telegram`; empty + error HTML. Ops group next to `settings`.
- [x] Stand-smoke `CARDS` includes `telegram`.

### Task 6: MCP (PH-S2314)

- [x] `gsv_telegram` read-only (same JSON as GET). **No** MCP send/poll in 167.
- [x] Drain prompt: bind 167; tickets are 168.

### Task 7: Contracts (PH-S2315)

- [x] Stub path never performs UDP/TCP to Telegram.
- [x] Error strings and MCP output omit token substrings.
- [x] `gsv_mcp_contracts` tool list +1; UI `RUST_CARDS` / `CARD_NAMES` lockstep.

### Task 8: Docs (PH-S2316)

- [x] BOXES Telegram row **✅**; spec P1 167 Landed; HANDOFF next = **168**.

### Task 9: Ratio + tests (PH-S2317)

- [x] fmt / clippy / `cargo test` (keep `target/live/`) / `--stretch-96` ≥ 96%.

### Task 10: Band close (PH-S2318)

- [x] `cargo xtask bump --band 167` + fingerprint + one commit + push.
- [x] NEXT: next = **168** (tickets), not 169.

---

# Band 168 — Ticket board + MCP claim (PH-S2319…S2328)

Landed this `абракадабра` on **gsv**. Do not skip to 169.

## File map (band 168)

| File | Role |
|------|------|
| `docs/gsv/tickets.jsonl` | source of truth (create + status). Seed one `open` sample **without secrets**. |
| `docs/gsv/ticket_claims.jsonl` | append-only claim rows `{ticket_id,ts,actor,ide,model,agent}` |
| `src/boxes/tickets.rs` | list / create / claim; parse JSONL; unknown id error |
| `src/boxes/mod.rs` | `pub mod tickets` |
| `src/boxes/settings.rs` | `ticket-claim` in `workflows.enabled` gates claim |
| `src/server/mod.rs` | `GET /api/tickets`; `POST /api/tickets`; `POST /api/tickets/claim` `{id}` (CSRF) |
| `src/boxes/ui.rs` | `render_tickets`; `CARD_NAMES` **40** |
| `src/mcp.rs` | `gsv_tickets` (list); `gsv_tickets_claim` `{id}` |
| `ui/index.html` | ops/studio card `tickets`; create/claim glue |
| `src/bin/gsv_http_stand_smoke.rs` | card + GET tickets |
| `tests/gsv_tickets_contracts.rs` | create, claim open→in_progress, unknown id, workflow gate, CSRF, no secrets in JSONL |
| docs | BOXES / SERVER / MCP / HANDOFF / NEXT / MEMORY / spec Landed 168 |

### Task 1: Scope (PH-S2319)

- [x] Confirm sibling `ticket_claims.jsonl` (spec default). No Telegram create-ticket. No bus.

### Task 2: JSONL schema (PH-S2320)

- [x] Ticket: `id`, `ts`, `title`, `body`, `status` (`open`/`in_progress`/`done`/`blocked`), `claimed_by` optional `{actor,ide,model,agent}`, `product`.
- [x] Claim row: `ticket_id`, `ts`, `actor`, `ide`, `model`, `agent`.
- [x] Missing files → empty list `{ok:true,tickets:[]}`. Never create under `data/` as source of truth.

### Task 3: HTTP (PH-S2321)

- [x] GET list. POST create (loopback CSRF). POST claim `{id}` → `open`→`in_progress`, set `claimed_by` from fingerprint-style resolve (`GSV_MODEL` / Cursor session / `unknown`).
- [x] Unknown id → 404 `{ok:false}`. Claim without `ticket-claim` enabled → 403 `{ok:false}`.

### Task 4: Galaxy card (PH-S2322)

- [x] `render_tickets`: open / in_progress / done columns (join copy: open tickets are the board). Empty/error HTML.

### Task 5: MCP (PH-S2323)

- [x] `gsv_tickets` list. `gsv_tickets_claim` `{id}` allowed. Unknown id → tool error. Workflow off → tool error.
- [x] Drain prompt names claim + 168; bus is 169.

### Task 6: Claim append (PH-S2324)

- [x] Successful claim appends `ticket_claims.jsonl` and rewrites the ticket line (or rewrite-all JSONL — pick append+rewrite file, keep tests deterministic).
- [x] Do not stage `data/*`. JSONL under `docs/gsv/` is committable (no secrets).

### Task 7: Contracts (PH-S2325)

- [x] Round-trip claim; CSRF; MCP claim; token/settings files untouched; `CARD_NAMES` / tool count lockstep.

### Task 8: Docs (PH-S2326)

- [x] BOXES Tickets row **✅**; spec P1 168 Landed; HANDOFF next = **169**.

### Task 9: Ratio + tests (PH-S2327)

- [x] fmt / clippy / `cargo test` (keep `target/live/`) / `--stretch-96` ≥ 96%.

### Task 10: Band close (PH-S2328)

- [x] `cargo xtask bump --band 168` + fingerprint + one commit + push.
- [x] NEXT: next = **169** (bus).

---

# Band 169 — Telegram bus (PH-S2329…S2338)

After 168. Channel envelopes between MCP bots. Still **not** Cloudflare. Still loopback Galaxy. Co-workflow `telegram-relay` required. Dry-run round-trip in tests **without** network.

## File map (band 169)

| File | Role |
|------|------|
| `src/boxes/telegram.rs` | envelope serde; `bus_send` / `bus_poll`; cap body; allowlist `godfather.allowed_user_ids`; dry-run in-memory queue for tests |
| `src/mcp.rs` | `gsv_telegram_bus_send` / `gsv_telegram_bus_poll` |
| `src/server/mod.rs` | optional `POST /api/telegram/bus` + `GET /api/telegram/bus` (same redact/cap; CSRF on POST) — keep MCP as primary |
| `src/boxes/ui.rs` | telegram card shows last bus ok/error (no new card required) |
| `src/boxes/settings.rs` | gate: `telegram-relay` enabled |
| `tests/gsv_telegram_contracts.rs` | two dry-run messages round-trip; workflow off errors; body cap; no token in envelope/logs |
| docs | BOXES bus row **✅**; spec P2 169 Landed; HANDOFF next = owner pick (no band 170 in this plan) |

Envelope: `{v:1, kind:"bus", from, to?, ticket_id?, body}`. Cap `body` (e.g. 2 KiB). `kind` other than `bus` ignored in v1.

### Task 1: Scope (PH-S2329)

- [x] No public webhook. No `cloudflared`. Poll from process / dry-run queue only.
- [x] Telegram create-ticket stays **out** (spec 169+ later; not this band unless owner asks).

### Task 2: Envelope + dry-run queue (PH-S2330)

- [x] Serialize/validate envelope. Invalid JSON → `{ok:false}`.
- [x] Test/dry-run: process-local VecDeque; send then poll returns the same item. No sockets.

### Task 3: Gates (PH-S2331)

- [x] Missing `telegram-relay` → tool/HTTP error.
- [x] If `allowed_user_ids` non-empty, `from` must match or error.
- [x] Rate-limit cheap (e.g. last-send timestamp; burst 1/s in-process).

### Task 4: MCP tools (PH-S2332)

- [x] `gsv_telegram_bus_send` `{from,to?,ticket_id?,body}`. `gsv_telegram_bus_poll` `{limit?}`.
- [x] Never return `bot_token`. Live send uses Godfather channel; tests stay on dry-run queue.

### Task 5: HTTP optional (PH-S2333)

- [x] Same JSON as MCP or skip HTTP if MCP-only is enough — prefer thin `GET`/`POST /api/telegram/bus` for Galaxy debug. CSRF on POST.

### Task 6: Card + contracts (PH-S2334)

- [x] Telegram card: `polling`, last bus ts/error.
- [x] Two-message dry-run test; token redact; workflow gate.

### Task 7: Docs (PH-S2335)

- [x] Spec P2 169 Landed; BOXES bus **✅**; HANDOFF: this plan **complete**; do not invent 170.

### Task 8: Ratio (PH-S2336)

- [x] fmt / clippy / `--stretch-96` ≥ 96%.

### Task 9: Tests (PH-S2337)

- [x] `cargo test` green; live copy stays.

### Task 10: Band close (PH-S2338)

- [x] `cargo xtask bump --band 169` + fingerprint + one commit + push.
- [x] NEXT: no queued band from this spec. Next drain = owner pick / warnings-first scan.
