# GSV solo / squad / jail (band 186)

> **For agentic workers:** Owner 2026-08-20: GSV update + security check; Git workflow research; federated join of a self-installed `gsv-server` MCP; host bot-admin vs own channel; squad cap = channel members; apps built inside a per-jail MCP sandbox. Spec: [`GSV_SOLO_SQUAD_JAIL.md`](../gsv/GSV_SOLO_SQUAD_JAIL.md). Telegram canon stays [`GSV_SETTINGS_TELEGRAM.md`](../gsv/GSV_SETTINGS_TELEGRAM.md).

**Goal:** Document and wire jail identity + squad capacity + join environment checks so a person who installed GSV can join a squad without sharing `bot_token` or pointing MCP at a remote `/mcp`.

**Architecture:** Settings `jail.id` + `tickets.{squad_cap,member_count,chat_kind}`. Presence refuses a *new* worker when `online >= squad_cap`. `GET /api/tickets` / MCP `gsv_tickets` expose `env` (loopback MCP, sandbox path, caps). No new mutating MCP tools. Resource `gsv://docs/solo-squad-jail`.

**Tech stack:** existing settings/tickets/Galaxy/MCP. Ratio `--stretch-96` ≥ 96%.

## Global constraints

- Loopback MCP; no User MCP; no MCP tunnel/apply.
- Never stage `data/*`, `.env*`, tokens.
- One commit at band close. No mid-drain push.
- Do not kill `target/live/` before `cargo test`.

## File map

| File | Role |
|------|------|
| `docs/gsv/GSV_SOLO_SQUAD_JAIL.md` | Spec (Git + Telegram + jail + join paths) |
| `src/boxes/settings.rs` | jail + squad_cap helpers |
| `src/boxes/tickets.rs` | capped presence + `join_env` |
| `src/boxes/ui.rs` + `ui/index.html` | Galaxy rows + save glue |
| `src/mcp.rs` | resource + `gsv_drain` Band 186 |
| `docs/gsv/ticket_scenarios.json` | `federated-join` / `own-channel` / `jail-app` |

---

# Band 186 — PH-S2499…S2508

### Task 1: Scope (PH-S2499)

- [x] Owner pick GSV; research Git worktrees + Telegram 20/50/200k ceilings.
- [x] Do not reopen 166–185.

### Task 2: Spec (PH-S2500)

- [x] Join path A (host bot admin) vs B (own channel).
- [x] Squad cap = member_count; bot slot cap = Telegram ceiling.
- [x] Security table (remote MCP, token share, process-local presence).

### Task 3: Settings schema (PH-S2501)

- [x] `jail.id`; `tickets.squad_cap` / `member_count` / `chat_kind`.
- [x] Redacted wire; PATCH; tests.

### Task 4: Presence cap + env (PH-S2502)

- [x] `heartbeat_capped`; `join_env` on tickets wire.

### Task 5: Galaxy + glue (PH-S2503)

- [x] Settings inputs; tickets jail/cap row; `saveSettings` posts jail/tickets caps.

### Task 6: MCP resource (PH-S2504)

- [x] `gsv://docs/solo-squad-jail`; drain prompt Band 186.

### Task 7: Scenarios (PH-S2505)

- [x] Catalog three join/jail scenarios.

### Task 8: Docs (PH-S2506)

- [x] Roadmap 186; SETTINGS pointer; HANDOFF/NEXT/MEMORY/BOXES/SERVER/MCP.

### Task 9: Tests (PH-S2507)

- [x] fmt · clippy · `cargo test` · `--stretch-96`.

### Task 10: Band close (PH-S2508)

- [x] `cargo xtask bump --band 186` + fingerprint; one commit + push.
