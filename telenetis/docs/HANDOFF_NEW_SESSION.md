# Telenetis — HANDOFF (new session)

- **Root**: `S:/rust/GSV/telenetis` (crate inside the GSV kit tree; no separate repo).
- **What it is**: standalone Rust (Axum 0.8, Tokio) Telegram Mini App + Bot bridging the
  GSV Godfather channel — server on **port 9800**. Docs:
  [`README.md`](../../docs/telenetis/README.md) · [`ops.md`](../../docs/telenetis/ops.md)
  (Docker / systemd / Windows always-on supervisor).
- **Kit registration**: row in
  [`docs/gsv/PRODUCTS.md`](../../docs/gsv/PRODUCTS.md); keep-live band 224
  (Windows parity: watchdog copies debug → live and respawns `:9800`);
  GSV MCP probe `gsv_telenetis_health`.
- **Latest work**: `301aa2d` Telenetis role-store data wiring + integration test README.
- **Tests**: `cargo fmt -- --check` → `cargo clippy --all-targets` → `cargo test`
  (run in `S:/rust/GSV/telenetis`). Do **not** kill the live `:9800` copy before tests.
- **Health**: `GET http://127.0.0.1:9800/health` (GSV `keep_live.telenetis`, fresh = up).
- **NEXT pointer**: [`NEXT_SESSION_PROMPT.md`](./NEXT_SESSION_PROMPT.md).
