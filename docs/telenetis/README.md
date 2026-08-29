# Telenetis — Telegram Mini App + Bot for GSV Godfather

Standalone Rust (Axum 0.8, Tokio) server on **port 9800** that bridges the GSV Godfather channel to a Telegram Mini App.

## Architecture

```
GSV (9999)  <--HTTP-->  Telenetis (9800)  <--HTTPS-->  Telegram Bot API
              bus v1 poll (5s)                webhook POST /webhook
              tickets sync                     inline keyboard web_app
              WS/SSE broadcast  <--WS-->  Mini App UI (board/flows/roles)
```

- **Config** (`src/config.rs`): `TELENETIS_BOT_TOKEN`, `TELENETIS_GSV_URL` (default 9999), `TELENETIS_PORT` (9800), `TELENETIS_JAIL_ID`, `TELENETIS_GODFATHER_CHANNEL_ID`, `TELENETIS_WEBHOOK_URL`.
- **AppState** (`src/state.rs`): `bus_queue`, `presence`, `tickets`, `flows` (cap 1000) + `broadcast::Sender<FlowEvent>` for WS/SSE.
- **GSV client** (`src/gsv/client.rs`): `/api/health`, `/api/tickets/list`, `/api/tickets/presence`, `/api/telegram/status`, `/api/telegram/bus`.
- **Bus** (`src/gsv/bus.rs`): v1 envelope `{v,kind,body,from,ts,data}` parse/format.
- **Poll loop** (`src/gsv/poll.rs`): `spawn_poll_loop` every 5s → `handle_bus_value` → `push_bus` + broadcast `FlowEvent`.
- **Bot** (`src/bot/telegram.rs`): `send_message`, `send_mini_app` (inline keyboard `web_app`), `answer_callback`, `set_webhook`.
- **Commands** (`src/bot/commands.rs`): `/start /status /board /flows /roles /help`.
- **Webhook** (`src/bot/webhook.rs`): `POST /webhook` classifies `message | callback_query | my_chat_member`, pushes `FlowEvent`.
- **Streams** (`src/stream/ws.rs` + `sse.rs`): `GET /ws` (WebSocket) + `GET /events` (SSE) from `flows_tx`.
- **Roles** (`src/roles/store.rs`): `Host|Mate|Guest|Observer` + **Timezone** (`tz.rs`) via `chrono-tz`.
- **Security** (`src/security/auth.rs`): `MAX_BODY_BYTES 64 KiB`, `csrf_check`, `security_headers` (nosniff/no-store/CSP).
- **Security** (`src/security/initdata.rs`): Telegram Mini App `initData` HMAC-SHA256 verification (secret key HMAC `WebAppData`, `auth_date` freshness, constant-time compare) — guards `/api/verify` + all `/api/board/*` actions.
- **Actions** (`src/actions.rs`): `BoardAction` (Claim/Done/Error/Reclaim) + `available_actions(status)` + body parsing + GSV forward; `/api/board/claim|done|error|reclaim` POST routes forward on behalf of the verified Mini App user to GSV `/api/tickets/*`.
- **UI** (`src/ui/mod.rs`): `GET /`, `/app`, `/board`, `/flows`, `/roles`, `/health`, `/api/status`, `/api/tickets`, `/api/flows`, `/api/snapshot?lang=`, `/api/mini-app/i18n`, `/api/live/config`, `/api/verify`, `/api/board/*`, `/static/app.css|js` (Askama templates in `src/ui/templates`).
- **Main** (`src/main.rs`): merges `ui + webhook + ws + sse` routers, spawns poll loop, binds `0.0.0.0:{port}`.

## Setup

```bash
export TELENETIS_BOT_TOKEN="123:ABC"
export TELENETIS_GSV_URL="http://127.0.0.1:9999"
export TELENETIS_PORT="9800"
export TELENETIS_JAIL_ID="telenetis-01"
export TELENETIS_GODFATHER_CHANNEL_ID="0"
export TELENETIS_WEBHOOK_URL="https://example.com/webhook" # optional
cd telenetis
cargo run
```

## Commands (Telegram)

- `/start` — welcome + command list
- `/status` — bot status
- `/board` — ticket board link (Mini App)
- `/flows` — live flows view
- `/roles` — role planner
- `/help` — same as /start

## API

- `GET /health` → `{status, service, version}`
- `GET /api/status` → `{online, jail_id, tickets_count, workers_online, recent_flows}`
- `GET /api/tickets` → `{tickets: [{id,title,status,product,claimed_by}]}`
- `GET /api/snapshot?lang=` → consolidated bundle (status + tickets + flows + workers + i18n + live config); tickets carry server-authoritative `actions` + `body`
- `GET /api/mini-app/i18n?lang=` → `{lang, strings}` (en/uk/ru)
- `GET /api/live/config` → server-authoritative reconnect + keep-alive schedule
- `GET /api/verify?initData=&authDate=` → initData HMAC validation `{ok, error?}`
- `POST /api/board/claim|done|error|reclaim` → initData HMAC verify → forward to GSV (`ActionQuery {initData, authDate}` + JSON `{action, ticket_id, note}`)
- `GET /api/flows` → `{flows: [FlowEvent]}`
- `POST /webhook` → `"ok"` (Telegram update)
- `GET /ws` → WebSocket JSON `FlowEvent` stream
- `GET /events` → SSE `text/event-stream`

## Tests

```bash
cd telenetis
cargo fmt --all
cargo clippy --all-targets
cargo test
```

**163** unit tests + **4** integration tests (`tests/integration_test.rs`) = **167** total.
