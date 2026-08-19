# GSV settings / Telegram / tickets Implementation Plan

> **For agentic workers:** Next `абракадабра` on **gsv** drains **band 166 only**. Do not implement 167–169 in that session. Use executing-plans or work the PH-S* table in [`GSV_TECH_ROADMAP.md`](../gsv/GSV_TECH_ROADMAP.md).

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

Queued only. Live Bot API behind dry-run stub in tests. Optional poller default **off**. MCP `gsv_telegram` status. Card `telegram`. No bus yet.

# Band 168 — Ticket board + MCP claim (PH-S2319…S2328)

Queued only. `docs/gsv/tickets.jsonl` + `docs/gsv/ticket_claims.jsonl`. HTTP + MCP claim. Card `tickets`. Co-workflow `ticket-claim` must be enabled.

# Band 169 — Telegram bus (PH-S2329…S2338)

Queued only. Channel envelopes between MCP bots. Co-workflow `telegram-relay`. Still not Cloudflare.
