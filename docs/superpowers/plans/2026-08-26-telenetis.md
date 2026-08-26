# Telenetis Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Build Telenetis — a Rust-first Telegram Mini App + Bot for GSV Godfather channel coordination: live ticket boards, bot status, flow streaming, role planning, timezone support.

**Architecture:** Standalone Axum server (port 9800) + Telegram Bot + Mini App web UI. Connects to GSV as a "mate" jail via Godfather bus protocol. WebSocket/SSE for live flow streaming. JSONL append-only event store.

**Tech Stack:** Rust 2021, Tokio, Axum 0.8, Telegram Bot API (raw HTTP), WebSocket, SSE, Askama + HTMX, serde, chrono/chrono-tz.

**Spec:** Integrates with GSV `ticket_scenarios.json`, `tickets.jsonl`, Godfather bus envelope v1.

## Global Constraints

- Rust 95-100% (no Python/Java)
- MSYS2 bash terminal (never PowerShell)
- Edition 2021, tokio, axum 0.8
- Port 9800 (9999=GSV, 8765=Hyper-V)
- Bus envelope protocol v1 compatible with GSV
- JSONL append-only storage
- CSRF gate, body caps, CSP headers
- Never leak bot_token

---

## Task 1: Scaffold Project

**Files:** `telenetis/Cargo.toml`, `src/main.rs`, `src/lib.rs`

- [x] Create `telenetis/Cargo.toml` with deps: tokio, axum(ws), tower-http, serde, serde_json, reqwest(json), chrono(serde), chrono-tz, uuid(v4), tracing, tracing-subscriber(env-filter), askama
- [x] Create `src/lib.rs` with `pub mod` stubs: bot, config, error, gsv, roles, security, state, stream, ui
- [x] Create `src/main.rs` — init tracing, Config::from_env, AppState::new, merge routers, bind port
- [x] `cargo check` — verify compiles (warnings ok)
- [x] Commit: `feat(telenetis): scaffold project`

---

## Task 2: Config + Error

**Files:** `src/config.rs`, `src/error.rs`, `tests/config_test.rs`

- [x] Test: `Config::from_env()` reads TELENETIS_BOT_TOKEN, _GSV_URL, _PORT, _JAIL_ID
- [x] Implement config.rs — Config struct with from_env()
- [x] Implement error.rs — TelenetisError enum: Telegram, Gsv, Config, Serialization, Io. IntoResponse impl
- [x] `cargo test config_test` — PASS
- [x] Commit: `feat(telenetis): config + error types`

---

## Task 3: AppState

**Files:** `src/state.rs`, `tests/state_test.rs`

Types: BusEnvelope, WorkerPresence, WorkerStatus (Ready/Busy/Offline), TicketRow, FlowEvent

- [x] Test: AppState creation, jail_id(), is_online()
- [x] Implement state.rs — Arc<RwLock<>> stores for bus_queue, presence, tickets, flows. push_flow caps at 1000
- [x] `cargo test state_test` — PASS
- [x] Commit: `feat(telenetis): AppState with stores`

---

## Task 4: GSV Client + Bus

**Files:** `src/gsv/mod.rs`, `src/gsv/client.rs`, `src/gsv/bus.rs`, `src/gsv/tickets.rs`, `tests/gsv_test.rs`

- [x] Test: parse_bus_envelope() for presence and sync kinds
- [x] Implement GsvClient — HTTP client to GSV /api/health, /api/tickets/list, /api/tickets/presence, /api/telegram/status
- [x] Implement bus.rs — parse_bus_envelope(), format_bus_envelope() (v1 protocol)
- [x] Implement tickets.rs — sync_tickets() fetches from GSV and updates AppState
- [x] `cargo test gsv_test` — PASS
- [x] Commit: `feat(telenetis): GSV client + bus connector`

---

## Task 5: Telegram Bot

**Files:** `src/bot/mod.rs`, `src/bot/telegram.rs`, `src/bot/commands.rs`, `src/bot/webhook.rs`, `src/bot/mini_app.rs`, `tests/bot_test.rs`

- [x] Test: parse_command() for /start, /board, unknown
- [x] Implement TelegramBot — send_message, send_mini_app (inline keyboard with web_app URL), answer_callback, set_webhook
- [x] Implement commands.rs — Command enum (Start/Status/Board/Flows/Roles/Help/Unknown), command_response()
- [x] Implement webhook.rs — Axum router POST /webhook, classify Telegram updates
- [x] Implement mini_app.rs — mini_app_url helper
- [x] `cargo test bot_test` — PASS
- [x] Commit: `feat(telenetis): Telegram bot + commands + webhook`

---

## Task 6: WebSocket Live Stream

**Files:** `src/stream/mod.rs`, `src/stream/ws.rs`, `src/stream/sse.rs`, `tests/stream_test.rs`

- [x] Test: flow event push + recent_flows retrieval
- [x] Implement ws.rs — Axum WebSocket upgrade, broadcast::Receiver<FlowEvent>, serialize to JSON per message
- [x] Implement sse.rs — SSE fallback endpoint, same FlowEvent stream
- [x] Add broadcast channel to AppState (tokio::sync::broadcast)
- [x] `cargo test stream_test` — PASS
- [x] Commit: `feat(telenetis): WebSocket + SSE live stream`

---

## Task 7: Role Management + Timezone

**Files:** `src/roles/mod.rs`, `src/roles/store.rs`, `src/roles/tz.rs`, `tests/roles_test.rs`

- [x] Test: role assignment, timezone conversion
- [x] Implement store.rs — RoleStore: assign_role(), list_roles(), remove_role(). Roles: host, mate, guest, observer
- [x] Implement tz.rs — TimezoneStore: user_tz map, convert_event_time(), format_for_user()
- [x] `cargo test roles_test` — PASS
- [x] Commit: `feat(telenetis): roles + timezone manager`

---

## Task 8: Security Layer

**Files:** `src/security/mod.rs`, `src/security/auth.rs`

- [x] Implement auth.rs — CSRF token extraction, body size cap, CSP headers middleware
- [x] Implement mod.rs — security middleware stack (tower layer)
- [x] `cargo test` — all pass
- [x] Commit: `feat(telenetis): security layer`

---

## Task 9: Mini App UI (HTMX)

**Files:** `src/ui/mod.rs`, `src/ui/templates/*.html`, `src/ui/static/app.css`, `src/ui/static/app.js`

- [x] Implement ui/mod.rs — Axum router for /, /app, /board, /flows, /roles, /api/status, /static/*
- [x] Templates: base.html (layout), dashboard.html (overview), board.html (ticket table), flows.html (live stream log), roles.html (role planner)
- [x] app.js — WebSocket connect, HTMX-style DOM updates for live flows
- [x] `cargo check` — compiles
- [x] Commit: `feat(telenetis): Mini App UI`

---

## Task 10: Poll Loop + Federation

**Files:** `src/gsv/poll.rs` (new), extend `src/bot/webhook.rs`

- [x] Implement poll.rs — spawn_poll_loop(): poll GSV /api/telegram/bus-poll, classify envelopes (presence/claim/done/sync), update AppState, broadcast to WS clients
- [x] Extend webhook.rs — full Telegram update classification (message, callback_query, my_chat_member)
- [x] Federation: on presence/claim/done events, push to bus_queue and broadcast as FlowEvent
- [x] `cargo test` — all pass
- [x] Commit: `feat(telenetis): poll loop + federation`

---

## Task 11: Integration Test + Docs

**Files:** `tests/integration_test.rs`, `docs/telenetis/README.md`

- [x] Integration test: start server, send webhook, verify WS broadcast
- [x] README: setup (env vars), architecture diagram, commands, API reference
- [x] `cargo test` — all pass
- [x] `cargo fmt --all`
- [x] Commit: `feat(telenetis): integration tests + README`

---

## Task 12: Register as GSV Product

**Files:** `docs/gsv/PRODUCTS.md` (modify), `docs/gsv/ticket_scenarios.json` (modify)

- [x] Add `telenetis` row to PRODUCTS.md
- [x] Add scenarios: `telenetis-setup`, `telenetis-bot`, `telenetis-stream`, `telenetis-roles` to ticket_scenarios.json
- [x] Add band 208 sprints to GSV_TECH_ROADMAP.md
- [x] Commit: `feat(telenetis): register product + scenarios + roadmap band 208`
