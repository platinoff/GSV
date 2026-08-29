# Telenetis — production deploy (ops)

This is the prod-ops companion to the telenetis README. It covers shipping the
Telegram Mini App + Bot on **port 9800** for real use: Docker, bare-metal
systemd, the Windows always-on supervisor, environment/secrets, and a
post-boot verification pass. Rust unit + integration tests (177 + 4) already
verify the HTTP/WS/SSE/webhook surfaces; the boot-verify script below is the
same check run against a live deploy.

## Runtime surfaces (must be live in prod)

| Path | Method | Purpose |
|------|--------|---------|
| `/health` | GET | Liveness probe (healthcheck). |
| `/api/snapshot?lang=` | GET | Mini App consolidated bundle (status + tickets + flows + i18n + live config). |
| `/api/status` `/api/live/config` | GET | Status + server-authoritative reconnect schedule. |
| `/api/verify` | GET | initData HMAC-SHA256 validation. |
| `/api/board/claim\|done\|error\|reclaim` | POST | Mini App board actions forwarded to GSV. |
| `/webhook` | POST | Telegram updates (or long polling when no webhook URL). |
| `/ws` | WS | Primary live broadcast (FlowEvent). |
| `/events` | SSE | Fallback live feed. |

## Environment matrix

All config is env-driven (`src/config.rs`). Copy `.env.example` to `.env`
(gitignored) and fill in. **Never commit a real bot token.** Key variables:

| Var | Required | Meaning |
|-----|----------|---------|
| `TELENETIS_BOT_TOKEN` | yes | Telegram bot token (BotFather). |
| `TELENETIS_GSV_URL` | yes | GSV server (default `http://127.0.0.1:9999`). |
| `TELENETIS_PUBLIC_URL` | prod | Public HTTPS base for the Mini App (phones/remote). |
| `TELENETIS_WEBHOOK_URL` | if webhook | Public base; `/webhook` appended. Empty ⇒ long polling. |
| `TELENETIS_WEBHOOK_SECRET` | webhook (rec.) | Secret sent to Telegram with `setWebhook`; every inbound `/webhook` must echo it in `X-Telegram-Bot-Api-Secret-Token` or is rejected (403). |
| `TELENETIS_PORT` | no | Listen port (9800). |
| `TELENETIS_JAIL_ID` | no | GSV presence/bus jail id. |
| `TELENETIS_GODFATHER_CHANNEL_ID` | no | Numeric channel; 0 disables forwarding. |
| `TELENETIS_TUNNEL_ENABLED` | prod ⇒ `0` | Auto-ngrok tunnel (dev only). |
| `TELENETIS_NGROK_BIN` | no | Explicit ngrok path (dev). |

## Option A — Docker (recommended for container hosts)

```bash
cd telenetis
cp .env.example .env
# fill .env (TELENETIS_BOT_TOKEN, GSV URL, public URL, tunnel off)
docker compose up -d --build
docker compose ps                 # status + HEALTHY
./scripts/telenetis-boot-verify.sh http://127.0.0.1:9800
docker compose logs -f telenetis
```

- `restart: unless-stopped` + `healthcheck` on `/health`.
- `TELENETIS_GSV_URL` defaults to `http://host.docker.internal:9999` for a GSV
  running on the Docker host (Linux needs `host.docker.internal` support /
  `extra_hosts`; use the machine IP otherwise).
- Persistent state in the `telenetis_data` volume (`/app/data`).
- The container runs the plain server binary (`main.rs`, graceful SIGTERM
  shutdown). Container/supervisor handles restart — no in-container
  `telenetis-live` needed.

## Option B — bare-metal systemd (Linux)

1. Build: `cd telenetis && cargo build --release --bin telenetis` and install
   the binary to `/opt/telenetis/telenetis`.
2. Secrets: put the env in `/etc/telenetis.env` (`chmod 600`).
3. Install the unit:

```bash
sudo cp deploy/systemd/telenetis.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now telenetis
systemctl status telenetis
./telenetis/scripts/telenetis-boot-verify.sh
sudo journalctl -u telenetis -f
```

- `Restart=always` + hardening (no new privileges, read-write only
  `/opt/telenetis/data`, private tmp). Tunnel disabled in prod.

## Option C — Windows always-on (developer / on-box)

The `telenetis-live` bin (supervisor) copies the debug binary to
`target/live/telenetis.exe` and respawns it on exit so `cargo build`/`cargo test`
do not fight a Windows file lock:

```bash
cd telenetis
cargo run --bin telenetis-live
# or, after a debug build:
target/release/telenetis-live   # (copy step picks the debug exe)
```

Pair with a task/WakeOnLAN/Startup entry (e.g. `schtasks /create` on ONLOGON)
for always-on on Windows.

## Boot verify

`scripts/telenetis-boot-verify.sh [BASE_URL]` checks, against a live server:

- `GET /health` 200
- `GET /api/snapshot?lang=en`, `/api/status`, `/api/live/config` 200
- `POST /webhook` 200 (with `TELENETIS_WEBHOOK_SECRET` set, add
  `X-Telegram-Bot-Api-Secret-Token: <secret>` or the server returns 403)
- `GET /ws` HTTP 101 Upgrade (WebSocket alive)
- `GET /events` `Content-Type: text/event-stream`

Exit 0 = all pass. Same five surfaces are exercised by the Rust integration
tests (`tests/integration_test.rs`).

## TLS / reverse proxy note

Telegram Mini App (`web_app` buttons) and `initData` are HTTPS-only for
phones. Put telenetis behind a TLS reverse proxy (Caddy/nginx) or an
ngrok/cloudflared tunnel and set `TELENETIS_PUBLIC_URL` to that HTTPS base;
the server itself is HTTP on 9800. `TELENETIS_TUNNEL_ENABLED=0` in prod.
