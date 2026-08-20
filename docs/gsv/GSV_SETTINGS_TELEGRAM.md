# GSV settings, Telegram Godfather, tickets, MCP bot bus

**Status:** Bands **166–186 ✅** (Telegram Godfather through solo/squad/jail). Latest: **186** jail identity + `squad_cap` = channel members + join `env`. **185** `catalog_stale` / Galaxy **restart Cursor** (agent refresh does not re-list). **184** JSON POST keeps `tools/list_changed` for the SSE hold · `listed_tool_count`. **183** `gsv_tickets_next` + MCP `tools/list_changed`. **182** dual session line + JSON `data` / Galaxy MCP signal / `gsv_telegram_decode`. **180–181** are watchdog/health (roadmap). **Next drain:** owner pick after a warnings-first scan.  
**Date:** 2026-08-20  
**Deciders:** owner  
**Owner ask:** GSV settings; Telegram channels; MCP bots talk to each other through a Telegram tunnel; a ticket board for people who want to join; MCP claims tickets and marks `in_progress` the same way fingerprints sync; server settings hold **Godfather** data (which channel, how secrets are stored, co-workflows). Federated join / jail: [`GSV_SOLO_SQUAD_JAIL.md`](./GSV_SOLO_SQUAD_JAIL.md). Next session starts with `абракадабра`.

**Plan:** [`docs/superpowers/plans/2026-08-19-gsv-settings-telegram-tickets.md`](../superpowers/plans/2026-08-19-gsv-settings-telegram-tickets.md)  
**Roadmap:** bands **166–169** in [`GSV_TECH_ROADMAP.md`](./GSV_TECH_ROADMAP.md)

## Problem statement

Galaxy is always-on and MCP (`gsv_mcp_openbot`) already wraps ops boxes, but there is **no GSV settings surface**, no first-party Telegram channel bind, and no join board. Agents cannot claim shared work except by inventing chat text. Fingerprints already prove the pattern: append-only JSONL, Galaxy card, MCP tool, vision-sync. Tickets and bot-to-bot Telegram need the same discipline **plus** a secret store that never lands in git.

Cost of leaving it: the next drain invents a one-off Telegram script, leaks a bot token into a commit, or skips a join board until someone pastes a Google sheet.

## Goals

1. Owner configures GSV on the live Galaxy **Settings** card (Godfather channel, co-workflows, secret policy) without putting tokens in git.
2. Joiners see a **ticket board**; MCP bots **claim** a ticket, mark `in_progress`, and leave a fingerprint-class row (actor / IDE / model / time).
3. Two (or more) `gsv_mcp_openbot` clients can exchange short control messages over a **Telegram channel bus** once Godfather is bound — not a public Cloudflare hop.
4. Band **186** is landed: solo/squad/jail (`jail.id`; `squad_cap` = Godfather `member_count`; join `env`; presence cap; `gsv://docs/solo-squad-jail`). Band **185** is landed: Cursor catalog restart lockstep (`catalog_stale`; Galaxy **restart Cursor** — agent refresh does not re-list tools). Band **184** is landed: MCP session catalog lockstep (JSON `POST /mcp` keeps `notifications/tools/list_changed` for Cursor's GET hold). Band **183** is landed: squad next-action (`gsv_tickets_next`) + MCP `notifications/tools/list_changed`. Bands **166–186** are landed. Next drain: owner pick after a warnings-first scan.
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
| **Co-workflow** | Named collaboration mode MCP may enter: `drain`, `ticket-claim`, `telegram-relay`, `ticket-squad`. Stored as ids + enabled flags, not code plugins. |
| **Ticket mode** | Settings `tickets.mode`: `solo` (one online MCP, stable pick) or `squad` (random among online heartbeats). Squad only applies when `ticket-squad` is enabled. |
| **Ticket board** | Append-only `docs/gsv/tickets.jsonl` (git-tracked, **no secrets**). Status `open` → `in_progress` → `done` / `blocked`. |
| **Telegram tunnel** | Channel-as-bus: bots post/read JSON envelopes on the Godfather channel. Not `cloudflared`. Band **169**. |
| **Fingerprint analog** | Each claim/sync appends actor / ide / model / agent / ticket_id / ts — same fields spirit as `docs/gsv/fingerprints.jsonl`. |

## User stories

- As owner, I open Settings on `http://127.0.0.1:9999/`, paste a Godfather channel id and a bot token, and the UI never echoes the token again (`token_set: true`).
- As owner, I enable co-workflows `drain` and `ticket-claim` so MCP agents know which collaboration modes are allowed.
- As a solo MCP bot, I ingest a Godfather `/ticket` message (`gsv_telegram_ticket`) and claim the board row when I am the only online worker.

- As an MCP bot, I list tickets, claim one, and the board + a fingerprint-class row show `in_progress` with my ide/model.
- As two MCP bots (Cursor + OpenCode), I send a short bus message through the Godfather channel and the other client sees it without a public HTTP tunnel.
- As owner, I type `run mcp bot hook up scenario band 177` (Godfather or MCP) and the board fills from `GSV_TECH_ROADMAP.md` PH-S* rows (open first; replay closed if none).
- As an MCP bot, I `gsv_telegram_bus_poll` or `gsv_telegram_decode` a Godfather post and read `data.hint` (`work-ticket` / `claim-next` / `work-assigned` / `record-bench` / `hook-placed`) plus `next` (PH-S*) and `disk_ok` to steer the next drain.
- As a squad of MCP clients, I heartbeat `gsv_tickets_presence`; a new development ticket from a scenario is assigned to one random online bot. Solo mode always uses the single MCP.
- As an MCP bot, I mark a ticket `done` or `blocked` (error) and the event JSONL + board stay in lockstep with fingerprints.
- As an MCP bot, my `in_progress` lease expires if I stop heartbeating; the ticket returns to `open` with `kind:reclaimed` so another worker can claim it.

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

### P1 — Should (band 167) — Godfather channel bind ✅

- `boxes/telegram.rs`: Bot API **getMe** / **getChat** against the configured channel; dry-run in tests (`X-Telegram-Dry-Run` or in-process stub).
- Galaxy card `telegram` (status, channel title, last probe). MCP `gsv_telegram` read-only status.
- Poller is **opt-in** (`workflows.enabled` contains `telegram-relay` or a `poll: true` flag). Default off so always-on Galaxy does not hit Telegram until the owner says so.
- Failures are `{ok:false,error}` — no panic, no token in error string.

### P1 — Should (band 168) — ticket board + MCP claim ✅

- `docs/gsv/tickets.jsonl` git-tracked (like fingerprints). Fields: `id`, `ts`, `title`, `body`, `status`, `claimed_by` `{actor,ide,model,agent}`, `product`.
- HTTP: `GET /api/tickets`, `POST /api/tickets` (create), `POST /api/tickets/claim` `{id}`.
- MCP: `gsv_tickets`, `gsv_tickets_claim` (claim **is** allowed on MCP — that is the point). Unknown id → tool error.
- Claim appends a fingerprint-class row (either extend `fingerprints.jsonl` with `kind: ticket-claim` **or** a sibling `docs/gsv/ticket_claims.jsonl` — pick one in 168; default **sibling file** so drain fingerprints stay drain-only).
- `gsv_vision_sync` remirrors ticket snapshot if we add `data/gsv_tickets.json` (optional); git JSONL remains source of truth.
- Galaxy card `tickets`. Join copy: open tickets are the board for people who want to join.

### P2 — Later (band 169) — Telegram bus between MCP bots ✅

- Envelope: `{v:1, kind: bus, from, to?, ticket_id?, body}` posted to the Godfather channel.
- MCP `gsv_telegram_bus_send` / `gsv_telegram_bus_poll` (cap body, no token out).
- Still **not** Cloudflare. Still loopback Galaxy. Rate-limit + allowlist `godfather.allowed_user_ids`.
- Co-workflow `telegram-relay` must be enabled in settings or the tools error.

### P2 — Should (band 170) — ticket scenarios + solo/squad ✅

Research (agents working the same backlog together): production MCP orchestrators (Delegator, Reactive Multi-Agent) use a **shared durable board**, **atomic claim/lease**, **heartbeat presence**, and **lifecycle events** (`claimed` / `done` / `error`) rather than chat-only coordination. MCP is agent-to-tool; the board is the agent-to-agent blackboard. GSV v1 keeps that pattern on JSONL + process-local presence (no extra SQLite).

| Piece | Acceptance |
|-------|------------|
| Scenarios | `docs/gsv/ticket_scenarios.json` — named templates (`id`, `title`, `body`, `workflow`, `product`). Galaxy tickets card places them as **add** buttons. |
| Product gate | `POST /api/tickets` / `gsv_tickets_create` require a **registered** `PRODUCTS.md` id (`gsv`, `poolai`, `omniroute`). Temp kits without that file only allow `gsv`. |
| Solo | `tickets.mode=solo` (default): dispatch picks the first online MCP (stable sort by actor/ide/agent). |
| Squad | `tickets.mode=squad` **and** workflow `ticket-squad`: dispatch picks `seed % online.len()`. Heartbeat `POST /api/tickets/presence` / `gsv_tickets_presence` (TTL 120s). Nobody online → ticket stays `open`. |
| Events | `ticket_claims.jsonl` `kind`: `claimed` · `assigned` · `done` · `error`. HTTP `POST /api/tickets/done` · `/error`. MCP `gsv_tickets_done` · `gsv_tickets_error`. |
| Tools | **46** MCP tools (`+create/done/error/presence`). `CARD_NAMES` stays **40**. Bench `gsv_dev` times pick_assignee + create/claim/done. |

### P2 — Should (band 171) — ticket lease + stale reclaim ✅

Band 170 research named **atomic claim/lease**. v1 claim had no timeout, so a dead MCP left `in_progress` forever.

| Piece | Acceptance |
|-------|------------|
| Lease | Claim/assign sets `lease_until = now + tickets.lease_secs` (default **300**, clamp 60–3600). Done/error clears it. Legacy WIP without `lease_until` is stale. |
| Renew | `POST /api/tickets/presence` / `gsv_tickets_presence` renews this worker's `in_progress` leases (even if the clock already passed — heartbeat grace). |
| Auto-reclaim | `GET /api/tickets` / claim / dispatch reclaim expired rows → `open`, `claimed_by` none, event `kind:reclaimed`. |
| Explicit | `POST /api/tickets/reclaim` `{id?}` (CSRF). Empty id = all stale. Id = that `in_progress` row. MCP `gsv_tickets_reclaim`. Requires `ticket-claim`. |
| Tools | **47** MCP tools. `CARD_NAMES` stays **40**. Galaxy WIP row shows lease + reclaim. |

### P2 — Should (band 174) — solo bot tickets from Telegram ✅

Owner pick (`абракадабра` gsv / Telegram messages): a **solo** MCP bot turns a Godfather channel message into a board row and claims it when it is the only online worker. Cargo tests use the dry-run stub (no sockets).

| Piece | Acceptance |
|-------|------------|
| Parse | `/ticket title` or JSON `{v:1,kind:ticket,body}`. Plain body allowed on the explicit API. Bus JSON is rejected. |
| Gates | `telegram-relay` **and** `ticket-claim`. Allowlist `from`. Body cap 2 KiB. |
| Solo | Presence heartbeat → `try_dispatch` claims as the one MCP. Nobody online → ticket stays `open`. |
| HTTP / MCP | `POST /api/telegram/ticket` (CSRF). `gsv_telegram_ticket` → **48** tools. Never `bot_token`. |
| Events | `kind:telegram` then `kind:claimed` when dispatched. Workflow `telegram`. |
| Galaxy | Telegram card shows `last_ticket_id` + MCP hint. Scenario `telegram-solo`. `CARD_NAMES` stays **40**. |

### P2 — Should (band 175) — MDS scenario band + solo walk + Telegram sync ✅

Owner pick: place a **band of tickets** that build a light Rust memory/disk/speed app; a solo bot walks claim → work → Telegram sync → done. Cargo tests stay dry-run (no sockets).

| Piece | Acceptance |
|-------|------------|
| Band | Scenario `tickets[]` creates N board rows (`create_band_from_scenario`). Catalog `memory-disk-speed` (6 steps). Single-ticket scenarios unchanged. |
| App | `gsv-mds` / `boxes/mds.rs`: 1 MiB alloc sample + Windows phys, `xtask::disk_report`, xor-fold ns/iter. `GET /api/mds`. |
| Walk | `solo_walk` claims then dones every `open` row (optional scenario filter). HTTP `POST /api/tickets/walk`. MCP `gsv_tickets_walk`. |
| Sync | Each claimed/done step enqueues `{v:1,kind:sync,ticket_id,body}`. Requires `telegram-relay` + `ticket-claim`. |
| Tools | **50** MCP tools. `CARD_NAMES` stays **40**. Bench `gsv_dev` times band create + walk + mds_report. |

### P2 — Should (band 176) — visible MCP session walk (solo / squad / bench) ✅

Owner pick (`абракадабра` gsv / watch the bot): the next drain is a **session you can read** on Godfather. The MCP bot walks a catalog scenario that looks like `абракадабра` (S0 → scan → solo work → squad assign → bench → close) and posts a **plain-text** line for each step. Cargo tests stay dry-run (no sockets). Live Bot API is 1 message/s.

| Piece | Acceptance |
|-------|------------|
| Catalog | Scenario `abrakadabra-session` with `tickets[]` (S0, warnings-first, solo MDS, squad claim, `gsv_dev` bench, close). |
| Copy | `kind:sync` body is a session line (`solo claimed …`, `squad assigned … to {worker}`, `bench gsv_dev … ns`), not only `{phase} {id}`. |
| Live | When token is set and not dry-run, walk **sendMessage**s those lines to the Godfather chat (1/s). Dry-run still only enqueues. |
| Solo | Existing `solo_walk` / `gsv_tickets_walk` remains the one-worker path. |
| Squad | Walk (or dispatch) with `tickets.mode=squad` + two presence rows posts **assigned** lines; one online worker is still a valid demo (`seed % 1`). |
| Bench | After walk, one sync line with `gsv_dev` medians (band create / solo walk / mds / enqueue) — from recorded bench JSON or a dry-run stub in tests. |
| Tools | Keep `CARD_NAMES` **40**. New MCP only if a dedicated `gsv_tickets_walk` mode arg is cleaner than extra tools. |

### P2 — Should (band 177) — roadmap/plan hook-up ✅

Owner pick (`абракадабра` gsv / Telegram bot messages / “run mcp bot hook up scenario”): continue Godfather sync, then **parse** project plans into scenarios and tickets so solo/squad MCP can pick them up. Cargo tests stay dry-run (no sockets).

Research (agents turning specs into a shared board): GitHub and Linear import markdown checklists as issues; production MCP orchestrators keep a **durable board** plus a **hook** that binds a worker to a named scenario. GSV v1 parses in-tree markdown (no extra SQLite): `GSV_TECH_ROADMAP.md` `PH-S*` tables and `docs/superpowers/plans/*.md` `- [ ]` items. Phrase grammar is stable: `run mcp bot hook up scenario <id|band N|plan stem> [walk]`.

| Piece | Acceptance |
|-------|------------|
| Phrase | `run mcp bot hook up scenario …`, `/hook …`, JSON `{kind:hook,source,id,walk?}`. Catalog id · `band N` · `plan <stem>`. Trailing `walk` optional. |
| Roadmap | `parse_roadmap_bands`: `## … band N` + `PH-S*` rows. Open = no ✅. Hook uses open rows, else replay all (cap **10**). Scenario id `roadmap-band-N`. |
| Plan | `parse_plan_open_items`: `- [ ]` / `* [ ]` only (skip `[x]`). Stem `[A-Za-z0-9._-]`, no `..`. Scenario id `plan-<stem>`. |
| Catalog | Same as `create_band_from_scenario`, but **idempotent**: skip titles already `open`/`in_progress`. |
| HTTP / MCP | `POST /api/tickets/hook`. MCP `gsv_tickets_hook` → **51** tools. Telegram ingest of the phrase uses the same path. |
| Sync | One `kind:sync` line `hook {source} {id} n={n}`. `walk:true` then `gsv_tickets_walk` (solo/squad). |
| Galaxy | Hook button per scenario + phrase field. `CARD_NAMES` stays **40**. |
| Bench | `gsv_dev` `hook_parse_phrase` + `hook_roadmap_band`. |

### P2 — Should (band 178) — scenario benchmark ✅

Owner pick (`абракадабра` gsv): the Godfather bench line was reading `speed_index` Criterion history (always zeros). Persist Instant timings for `abrakadabra-session` create+walk so session copy, Galaxy, and MCP show real ns.

| Piece | Acceptance |
|-------|------------|
| Harness | `time_session_walk` + `gsv_dev` `session_walk_abrakadabra` on a throwaway kit. |
| Persist | `docs/gsv/scenario_bench.json` (`create_ns` / `walk_ns` / `session_walk_ns` / `mds_ns` / `enqueue_ns`). |
| HTTP / MCP | `GET`/`POST /api/tickets/bench`. MCP `gsv_tickets_bench` `{run?}` → **52** tools. |
| Session line | `bench gsv_dev create=… walk=… mds=… enqueue=… session=… ns`. Prefers JSON; speed-index fallback keeps `session=0`. |
| Galaxy | Last bench on tickets card + record button. `CARD_NAMES` **40**. |
| xtask | `cargo xtask record-scenario-bench`. |

### P2 — Should (band 179) — Godfather inbound poller ✅

Owner pick (`абракадабра` gsv): `poller_wanted` was status-only. Always-on `gsv-server` now `getUpdates` when `godfather.poll` or `telegram-relay` is on. Cargo tests stay dry-run (stub queue, no sockets). Stdio `gsv-mcp` does **not** spawn the loop (shared offset file).

| Piece | Acceptance |
|-------|------------|
| Classify | `classify_inbound`: hook phrase · bus/sync JSON · `/ticket` / `{kind:ticket}`. Skip plain chat and outbound session lines (`solo claimed …`, `bench gsv_dev …`, `hook … n=`). |
| Loop | `spawn_poll_loop` from `gsv-server` after `enable_live_api`. 1/s. No-op when live API is off (cargo test). |
| Offset | `data/telegram_offset.json` (gitignored). |
| HTTP / MCP | `POST /api/telegram/poll`. MCP `gsv_telegram_poll` → **53** tools. CSRF. Never `bot_token`. |
| Galaxy | Telegram card: poll loop / last poll / last ingest + **poll now**. `CARD_NAMES` **40**. |

Bands **180–181** (watchdog hop / Galaxy glue + health `disk_ok`) live in [`GSV_TECH_ROADMAP.md`](./GSV_TECH_ROADMAP.md) — this spec stays Telegram-only so those tables are not copied here.

### P2 — Should (band 182) — MCP-readable Godfather envelopes ✅

Owner ask: session comments on Telegram must be parseable by MCP so bots can correct the next drain (product, ticket, next PH-S*, disk, crate). Cargo tests stay dry-run (no sockets). Live `sendMessage` stays 1/s.

| Piece | Acceptance |
|-------|------------|
| Dual post | Walk/hook live body is `{human session line}\n{JSON envelope}`. JSON is `{v:1,kind:sync,from,ticket_id,body,data}`. |
| `data` | `product`, `scenario`, `phase`, `mode`, `actor`, `crate`, `next`, `disk_ok`, `disk_free_gb`, `hint`. |
| `hint` | `work-ticket` · `claim-next` · `work-assigned` · `record-bench` · `hook-placed`. |
| Parse | `extract_envelope`: compact JSON · `GSV1 ` prefix · dual line+JSON. Legacy plain session lines still `skip`. |
| HTTP / MCP | `POST /api/telegram/decode`. MCP `gsv_telegram_decode` → **54** tools. No telegram-relay gate. Never `bot_token`. |
| Inbound | `classify_inbound` / poll / live bus poll treat dual JSON as `bus` so the other MCP can `gsv_telegram_bus_poll`. |
| Galaxy | Telegram card **MCP signal** row (`last_hint` / `last_next` / `last_body`) — tickets card does not repeat the envelope. Walk/hook/bench refresh Telegram. Vision remirror is one `syncVision` glue (`data-action=vision-sync` = Power soft). `CARD_NAMES` stays **40**. |

### P2 — Should (band 183) — squad next-action + MCP catalog lockstep ✅

Owner pick (`абракадабра` gsv): analyze project logic, find broken functions, research squad/MCP workflows and standardizations. Industry split: **MCP** is agent-to-tool; **A2A** is agent-to-agent with a task lifecycle. GSV already uses the JSONL board as a local blackboard. Gap: envelopes have `hint` but no inbox tool; Cursor can freeze a stale MCP catalog (36 tools) while live `GET /mcp` has 54+. Cargo tests stay dry-run.

| Piece | Acceptance |
|-------|------------|
| Next | `next_action`: WIP → `gsv_tickets_done`; `record-bench` → bench; `hook-placed` → walk; else first open → claim; empty → idle. |
| HTTP / MCP | `POST /api/tickets/next`. MCP `gsv_tickets_next` → **55** tools. GET list includes `next`. Never `bot_token`. |
| Catalog | `initialize` `tools.listChanged=true`. `notifications/initialized` emits `notifications/tools/list_changed`. GET `/mcp` `tools_list_changed`. |
| Galaxy | Tickets card next row + `tickets-next` glue. MCP card shows `listChanged`. `CARD_NAMES` **40**. |

### P2 — Should (band 185) — Cursor catalog restart lockstep ✅

Owner pick (`абракадабра` gsv): Cursor agent **Оновити** still showed **36** tools while live crate listed **55**. `GET /mcp` `listed_tool_count` stayed **0**. A full **Cursor restart** called `tools/list` (55 tools + 11 resources). Agent refresh only resubscribes resources.

| Piece | Acceptance |
|-------|------------|
| Stale | `catalog_stale` when sessions exist and `listed_tool_count` is 0, or listed ≠ `tool_count`. |
| Hint | `restart Cursor — agent refresh does not re-list tools`. |
| Galaxy | MCP card `<span class='warn'>restart Cursor</span>`. `CARD_NAMES` **40**. |
| Health | `gsv_health` includes `catalog_stale` / `catalog_hint` / `listed_tool_count`. |

### P2 — Should (band 186) — solo/squad/jail + federated join ✅

Owner pick: update + security check; Git worktree research; a joiner who installed GSV must connect **their** MCP to a squad without sharing `bot_token` or pointing at a remote `/mcp`. Spec: [`GSV_SOLO_SQUAD_JAIL.md`](./GSV_SOLO_SQUAD_JAIL.md).

| Piece | Acceptance |
|-------|------------|
| Jail | Settings `jail.id` (empty → `local`). Redacted GET/MCP. |
| Cap | `tickets.squad_cap` `0` → `member_count` else `bot_slot_cap` (50 channel / 20 group). Hard clamp 200 000. |
| Presence | New worker refused when `online >= squad_cap`. Holder renews. |
| Env | `GET /api/tickets` `env` (loopback MCP, sandbox, caps). MCP `gsv_tickets`. |
| Scenarios | `federated-join` · `own-channel` · `jail-app`. |
| Resource | `gsv://docs/solo-squad-jail` → **12** resources. **55** tools. `CARD_NAMES` **40**. |

## Security (how we store)

| Layer | Rule |
|-------|------|
| Git | Never stage `data/*`, `.env*`, `*.pem`. Tokens are not in `docs/`. |
| Disk | `data/gsv_settings.json` mode is “local owner file”. Windows ACL is owner-machine; we do not invent a vault in v1. |
| Env | `GSV_TELEGRAM_BOT_TOKEN` overrides file; process env is not dumped to `/api/*`. |
| API / MCP / logs | Redact. `token_set` only. Preview confine still cannot read `../` or `file://`. |
| Telegram | v1 poll from the server process; no public webhook URL. Godfather channel is private/invite. |
| MCP write | Band 166: settings **read**. Band 168: ticket **claim**. Band 170: ticket **create/done/error/presence**. Band 171: ticket **reclaim**. Band 175: ticket **walk**. Band 177: ticket **hook** (catalog / roadmap / plan). Band 178: ticket **bench** (throwaway kit; persist JSON). Band 179: Telegram **poll** (one `getUpdates` pass; loop is `gsv-server` only). Band 182: Telegram **decode** (parse-only; no token). Band 183: ticket **next** (inbox; heartbeat). Never `update/apply` / tunnel start. |

## Co-workflows (v1 ids)

| id | Who | Effect |
|----|-----|--------|
| `drain` | VDT `абракадабра` | Unchanged drain; settings card may show it as enabled. |
| `ticket-claim` | MCP / Galaxy | Allows `gsv_tickets_claim` / done / error / reclaim (band 168–171). |
| `telegram-relay` | MCP / poller | Allows bus send/poll (band 169), ticket ingest (174), solo-walk `kind:sync` (175), live session lines (176), and hook Godfather lines (177). |
| `ticket-squad` | MCP / Galaxy | Allows `tickets.mode=squad` random assign among online MCP (band 170). |

Unknown ids in the file are kept but ignored (forward compatible).

## Success metrics

- Band 166: `GET /api/settings` redacted; POST token → `token_set:true` and file exists under `data/`; `git status` does not show that file; MCP `gsv_settings` has no token key; card `settings` in `CARD_NAMES`; tests green; `--stretch-96` ≥ 96%.
- Band 168: claim from MCP flips `open` → `in_progress` and appends a claim row with ide/model.
- Band 169: two dry-run bus messages round-trip in tests without network.
- Band 170: scenario create gated by workflow; unregistered product rejected; solo picks one MCP; squad pick is `seed % n`; done/error append `kind` events; `gsv_dev` bench prints pick_assignee + create/claim/done.
- Band 174: `/ticket` ingest creates a row; one online MCP in solo mode claims it; MCP `gsv_telegram_ticket`; `--stretch-96` ≥ 96%.
- Band 175: scenario `memory-disk-speed` places 6 tickets; solo walk claims/dones them and enqueues `kind:sync`; `gsv-mds` reports memory/disk/speed; `--stretch-96` ≥ 96%.
- Band 176: Godfather (live) or bus queue (dry-run) shows session lines for solo, squad, and bench; scenario `abrakadabra-session`; `--stretch-96` ≥ 96%.
- Band 177: phrase `run mcp bot hook up scenario band 177` places ≤10 tickets from the roadmap; catalog/plan sources work; idempotent re-hook; MCP `gsv_tickets_hook`; `--stretch-96` ≥ 96%.
- Band 178: `GET /api/tickets/bench` empty-ok; `POST {run:true}` writes `scenario_bench.json`; Godfather line includes `session=`; MCP `gsv_tickets_bench`; `--stretch-96` ≥ 96%.
- Band 179: `POST /api/telegram/poll` classifies stub/live updates; MCP `gsv_telegram_poll`; `gsv-server` loop when live; offset in `data/telegram_offset.json`; `--stretch-96` ≥ 96%.
- Bands 180–181: watchdog process lockstep and Galaxy glue / S0 `disk_ok` — roadmap only ([`GSV_TECH_ROADMAP.md`](./GSV_TECH_ROADMAP.md)); no extra Telegram P2 copy here.
- Band 182: walk posts human line + JSON `data`; MCP `gsv_telegram_decode`; Galaxy MCP signal row (no envelope copy on tickets); walk refreshes Telegram; `--stretch-96` ≥ 96%.
- Band 183: `POST /api/tickets/next` + MCP `gsv_tickets_next` returns `hint`/`tool`/`ticket_id`; `initialize` advertises `tools.listChanged`; `notifications/initialized` emits `notifications/tools/list_changed`; `--stretch-96` ≥ 96%.
- Band 184: JSON `POST /mcp` keeps the notification queue; session GET SSE hold emits `tools/list_changed`; GET `/mcp` `listed_tool_count` / `catalog_notify`; `--stretch-96` ≥ 96%.
- Band 185: GET `/mcp` `catalog_stale` / `catalog_hint` when a session exists but `tools/list` is missing or short; Galaxy MCP card `restart Cursor`; agent refresh is not enough; `--stretch-96` ≥ 96%.
- Band 186: jail id + `squad_cap` = member_count; presence `accepted`; tickets `env`; resource `gsv://docs/solo-squad-jail`; `--stretch-96` ≥ 96%.

## Open questions (non-blocking)

- Multi-channel Godfather (work vs announce) — **v1 = one channel**.
- Public join via a second Telegram group vs Galaxy-only board — **v1 = Galaxy + JSONL**; Telegram create-ticket is 169+.
- Whether claim rows join `fingerprints.jsonl` — **default sibling `ticket_claims.jsonl`**.

## Phasing (VDT bands)

| Band | PH-S* | Focus | When |
|------|-------|--------|------|
| **166** | S2299–S2308 | Settings box + secret store + redacted MCP/UI | **✅ landed** |
| **167** | S2309–S2318 | Godfather channel bind (dry-run tests + optional poll) | **✅ landed** |
| **168** | S2319–S2328 | Ticket board + MCP claim + claim JSONL | **✅ this drain** |
| **170** | S2339–S2348 | Ticket scenarios + solo/squad MCP + registered-product create + done/error events | **✅ this drain** |
| **171** | S2349–S2358 | Ticket lease + stale reclaim + `gsv_tickets_reclaim` | **✅ this drain** |
| **174** | S2379–S2388 | Solo bot tickets from Telegram (`gsv_telegram_ticket`) | **✅ landed** |
| **175** | S2389–S2398 | MDS scenario band + solo walk + Telegram `kind:sync` + `gsv-mds` | **✅ this drain** |
| **176** | S2399–S2408 | Visible MCP session walk (solo / squad / bench on Godfather) | **✅ this drain** |
| **177** | S2409–S2418 | Roadmap/plan hook-up (`run mcp bot hook up scenario`) | **✅ this drain** |
| **178** | S2419–S2428 | Scenario benchmark (`abrakadabra-session` Instant timings) | **✅ this drain** |
| **179** | S2429–S2438 | Godfather inbound poller (`getUpdates` loop + `gsv_telegram_poll`) | **✅ landed** |
| **180** | S2439–S2448 | Watchdog process lockstep (not Telegram — see roadmap) | **✅ landed** |
| **181** | S2449–S2458 | Galaxy glue + S0 `disk_ok` (not Telegram — see roadmap) | **✅ landed** |
| **182** | S2459–S2468 | MCP-readable Godfather envelopes (`gsv_telegram_decode`) + Galaxy MCP signal | **✅ this drain** |
| **183** | S2469–S2478 | Squad next-action (`gsv_tickets_next`) + MCP `tools/list_changed` | **✅ this drain** |
| **184** | S2479–S2488 | MCP session catalog lockstep (JSON POST keep-queue + SSE hold notify) | **✅ this drain** |
| **185** | S2489–S2498 | Cursor catalog restart lockstep (`catalog_stale` · restart Cursor, not agent refresh) | **✅ this drain** |
| **186** | S2499–S2508 | Solo/squad/jail (`jail.id` · squad_cap = members · join env) | **✅ this drain** |

Next drain: **owner pick** after a warnings-first scan.

## Constraints

- MSYS2 bash; Rust 95–100% / wasm 0–5%; thin `ui/index.html` glue.
- CSRF + loopback POSTs (band 133); CSP / no-store (band 134).
- Do not kill `target/live/` before `cargo test`.
- One commit per drain; push last. Close: `cargo xtask bump --band N` then `cargo xtask fingerprint`.

## See also

- Fingerprints pattern: [`GSV_BOXES.md`](./GSV_BOXES.md) · `src/boxes/fingerprint.rs`
- MCP: [`GSV_MCP_OPENBOT.md`](./GSV_MCP_OPENBOT.md)
- Omni secrets (same redact idea): `data/omni.toml` + `GET /api/omni/config`
- Tunnel (different): `cargo xtask tunnel` — not this spec
