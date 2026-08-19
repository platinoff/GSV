# GSV settings, Telegram Godfather, tickets, MCP bot bus

**Status:** Landed band **166** (owner pick 2026-08-19) · **next drain = band 167** Godfather bind · bands **168–169** sequenced, not this session  
**Date:** 2026-08-19  
**Deciders:** owner  
**Owner ask:** GSV settings; Telegram channels; MCP bots talk to each other through a Telegram tunnel; a ticket board for people who want to join; MCP claims tickets and marks `in_progress` the same way fingerprints sync; server settings hold **Godfather** data (which channel, how secrets are stored, co-workflows). Next session starts with `абракадабра`.

**Plan:** [`docs/superpowers/plans/2026-08-19-gsv-settings-telegram-tickets.md`](../superpowers/plans/2026-08-19-gsv-settings-telegram-tickets.md)  
**Roadmap:** bands **166–169** in [`GSV_TECH_ROADMAP.md`](./GSV_TECH_ROADMAP.md)

## Problem statement

Galaxy is always-on and MCP (`gsv_mcp_openbot`) already wraps ops boxes, but there is **no GSV settings surface**, no first-party Telegram channel bind, and no join board. Agents cannot claim shared work except by inventing chat text. Fingerprints already prove the pattern: append-only JSONL, Galaxy card, MCP tool, vision-sync. Tickets and bot-to-bot Telegram need the same discipline **plus** a secret store that never lands in git.

Cost of leaving it: the next drain invents a one-off Telegram script, leaks a bot token into a commit, or skips a join board until someone pastes a Google sheet.

## Goals

1. Owner configures GSV on the live Galaxy **Settings** card (Godfather channel, co-workflows, secret policy) without putting tokens in git.
2. Joiners see a **ticket board**; MCP bots **claim** a ticket, mark `in_progress`, and leave a fingerprint-class row (actor / IDE / model / time).
3. Two (or more) `gsv_mcp_openbot` clients can exchange short control messages over a **Telegram channel bus** once Godfather is bound — not a public Cloudflare hop.
4. Next `абракадабра` on **gsv** drains **band 167** (Godfather bind) after S0 + warnings-first scan. Do **not** skip to the ticket board (168) or the bus (169).
5. Ratio stays `gsv-loc-audit --stretch-96` ≥ 96%. No Python. Secrets never in MCP/HTTP JSON.

## Non-goals

- **No** live Telegram Bot API in band **166** (schema + store + redacted UI/MCP only).
- **No** Cloudflare / `cargo xtask tunnel` on MCP (still owner CLI opt-in). Telegram outbound HTTPS ≠ that tunnel.
- **No** copying PoolAI `telegram_seat` / wallet / OAuth into this crate.
- **No** `gsv_products_open`, `update/apply`, or start-tunnel tools.
- **No** User-scope Cursor MCP; no LAN widen; no Python adapters.
- **No** embedding Grok Bot’s cloud computer.
- **No** public internet webhook listener in v1 (poll Bot API from loopback).

## Vocabulary

| Term | Meaning in GSV |
|------|----------------|
| **Godfather** | Owner control identity: Telegram **bot token** (secret) + **one** control channel/chat id + optional allowlisted user ids. v1 is a single channel. |
| **Settings** | Galaxy box + `GET`/`POST /api/settings`. Public fields (channel id, workflow ids, `token_set`) vs secrets (token). |
| **Co-workflow** | Named collaboration mode MCP may enter: `drain`, `ticket-claim`, `telegram-relay`. Stored as ids + enabled flags, not code plugins in v1. |
| **Ticket board** | Append-only `docs/gsv/tickets.jsonl` (git-tracked, **no secrets**). Status `open` → `in_progress` → `done` / `blocked`. |
| **Telegram tunnel** | Channel-as-bus: bots post/read JSON envelopes on the Godfather channel. Not `cloudflared`. Band **169**. |
| **Fingerprint analog** | Each claim/sync appends actor / ide / model / agent / ticket_id / ts — same fields spirit as `docs/gsv/fingerprints.jsonl`. |

## User stories

- As owner, I open Settings on `http://127.0.0.1:9999/`, paste a Godfather channel id and a bot token, and the UI never echoes the token again (`token_set: true`).
- As owner, I enable co-workflows `drain` and `ticket-claim` so MCP agents know which collaboration modes are allowed.
- As a joiner, I see open tickets on a Galaxy board and understand what is already `in_progress`.
- As an MCP bot, I list tickets, claim one, and the board + a fingerprint-class row show `in_progress` with my ide/model.
- As two MCP bots (Cursor + OpenCode), I send a short bus message through the Godfather channel and the other client sees it without a public HTTP tunnel.
- As owner, I type `абракадабра`, pick **GSV**, and the agent drains **band 167** from this spec — not 168–169.

## Requirements

### P0 — Must (band 166, `PH-S2299…S2308`) — settings + secret store ✅

Settings exist **before** any Telegram network call.

| Piece | Acceptance |
|-------|------------|
| Schema | `SettingsFile` serde: `godfather.channel_id`, `godfather.allowed_user_ids`, `godfather.bot_token` (secret), `workflows.enabled: string[]`, `security.redact: true`. Unknown fields `#[serde(default)]`. |
| Store | Secrets in **`data/gsv_settings.json`** (already gitignored via `/data/*`). Env **`GSV_TELEGRAM_BOT_TOKEN`** wins over file. Never write the env value back to disk unless owner POSTs it. |
| Redaction | `GET /api/settings` and MCP `gsv_settings` return `token_set: bool`, never `bot_token`. POST that includes a token stores it and returns redacted JSON. |
| Galaxy | Ops card `settings` (`CARD_NAMES` +1). Empty-tolerant; error HTML on I/O fail. |
| MCP | Tool `gsv_settings` (read redacted). **No** MCP write of tokens in 166 (HTTP POST stays the owner path). Resource `gsv://docs/settings-telegram` → this file. |
| Security | CSRF + loopback Origin on `POST /api/settings`. Body cap 256 KiB. Log lines must not print the token. Contracts assert redaction. |
| Docs | BOXES / SERVER / MCP_OPENBOT / HANDOFF / NEXT / MEMORY / this spec status. |

### P1 — Should (band 167) — Godfather channel bind

- `boxes/telegram.rs`: Bot API **getMe** / **getChat** against the configured channel; dry-run in tests (`X-Telegram-Dry-Run` or in-process stub).
- Galaxy card `telegram` (status, channel title, last probe). MCP `gsv_telegram` read-only status.
- Poller is **opt-in** (`workflows.enabled` contains `telegram-relay` or a `poll: true` flag). Default off so always-on Galaxy does not hit Telegram until the owner says so.
- Failures are `{ok:false,error}` — no panic, no token in error string.

### P1 — Should (band 168) — ticket board + MCP claim

- `docs/gsv/tickets.jsonl` git-tracked (like fingerprints). Fields: `id`, `ts`, `title`, `body`, `status`, `claimed_by` `{actor,ide,model,agent}`, `product`.
- HTTP: `GET /api/tickets`, `POST /api/tickets` (create), `POST /api/tickets/claim` `{id}`.
- MCP: `gsv_tickets`, `gsv_tickets_claim` (claim **is** allowed on MCP — that is the point). Unknown id → tool error.
- Claim appends a fingerprint-class row (either extend `fingerprints.jsonl` with `kind: ticket-claim` **or** a sibling `docs/gsv/ticket_claims.jsonl` — pick one in 168; default **sibling file** so drain fingerprints stay drain-only).
- `gsv_vision_sync` remirrors ticket snapshot if we add `data/gsv_tickets.json` (optional); git JSONL remains source of truth.
- Galaxy card `tickets`. Join copy: open tickets are the board for people who want to join.

### P2 — Later (band 169) — Telegram bus between MCP bots

- Envelope: `{v:1, kind: bus, from, to?, ticket_id?, body}` posted to the Godfather channel.
- MCP `gsv_telegram_bus_send` / `gsv_telegram_bus_poll` (cap body, no token out).
- Still **not** Cloudflare. Still loopback Galaxy. Rate-limit + allowlist `godfather.allowed_user_ids`.
- Co-workflow `telegram-relay` must be enabled in settings or the tools error.

## Security (how we store)

| Layer | Rule |
|-------|------|
| Git | Never stage `data/*`, `.env*`, `*.pem`. Tokens are not in `docs/`. |
| Disk | `data/gsv_settings.json` mode is “local owner file”. Windows ACL is owner-machine; we do not invent a vault in v1. |
| Env | `GSV_TELEGRAM_BOT_TOKEN` overrides file; process env is not dumped to `/api/*`. |
| API / MCP / logs | Redact. `token_set` only. Preview confine still cannot read `../` or `file://`. |
| Telegram | v1 poll from the server process; no public webhook URL. Godfather channel is private/invite. |
| MCP write | Band 166: settings **read**. Band 168: ticket **claim**. Never `update/apply` / tunnel start. |

## Co-workflows (v1 ids)

| id | Who | Effect |
|----|-----|--------|
| `drain` | VDT `абракадабра` | Unchanged drain; settings card may show it as enabled. |
| `ticket-claim` | MCP / Galaxy | Allows `gsv_tickets_claim` (band 168). |
| `telegram-relay` | MCP / poller | Allows bus send/poll (band 169) and optional channel poll (167). |

Unknown ids in the file are kept but ignored (forward compatible).

## Success metrics

- Band 166: `GET /api/settings` redacted; POST token → `token_set:true` and file exists under `data/`; `git status` does not show that file; MCP `gsv_settings` has no token key; card `settings` in `CARD_NAMES`; tests green; `--stretch-96` ≥ 96%.
- Band 168: claim from MCP flips `open` → `in_progress` and appends a claim row with ide/model.
- Band 169: two dry-run bus messages round-trip in tests without network.

## Open questions (non-blocking)

- Multi-channel Godfather (work vs announce) — **v1 = one channel**.
- Public join via a second Telegram group vs Galaxy-only board — **v1 = Galaxy + JSONL**; Telegram create-ticket is 169+.
- Whether claim rows join `fingerprints.jsonl` — **default sibling `ticket_claims.jsonl`**.

## Phasing (VDT bands)

| Band | PH-S* | Focus | When |
|------|-------|--------|------|
| **166** | S2299–S2308 | Settings box + secret store + redacted MCP/UI | **✅ this drain** |
| **167** | S2309–S2318 | Godfather channel bind (dry-run tests + optional poll) | after 166 |
| **168** | S2319–S2328 | Ticket board + MCP claim + claim JSONL | after 167 |
| **169** | S2329–S2338 | Telegram bus between MCP bots | after 168 |

Do **not** invent band 170 in the 166 drain.

## Constraints

- MSYS2 bash; Rust 95–100% / wasm 0–5%; thin `ui/index.html` glue.
- CSRF + loopback POSTs (band 133); CSP / no-store (band 134).
- Do not kill `target/live/` before `cargo test`.
- One commit per drain; push last. Close: `cargo xtask bump --band 166` then `cargo xtask fingerprint`.

## See also

- Fingerprints pattern: [`GSV_BOXES.md`](./GSV_BOXES.md) · `src/boxes/fingerprint.rs`
- MCP: [`GSV_MCP_OPENBOT.md`](./GSV_MCP_OPENBOT.md)
- Omni secrets (same redact idea): `data/omni.toml` + `GET /api/omni/config`
- Tunnel (different): `cargo xtask tunnel` — not this spec
