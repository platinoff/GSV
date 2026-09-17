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
- **Latest work (2026-09-17, bands T1–T7, all ticket-driven)**: reopen-hint on stale initData (403),
  `answerWebAppQuery` builder (wiring verdict: no keyboard-flow → none needed),
  probe compatibility-mode, claim/next hints (snapshot + `/board`),
  Download auto-save (IDB is cache), sticky model auto-Use, worker chain memory
  (`payload.history`), WS tracker `GET /tracker` + client announce.
  Concept: [`CONCEPT.md`](../../docs/telenetis/CONCEPT.md).
- **Tests**: `cargo fmt -- --check` → `cargo clippy --all-targets` (0) → `cargo test`
  (**280** lib + 4 live + 4 integration, green)
  (run in `S:/rust/GSV/telenetis`). Do **not** kill the live `:9800` copy before tests.
- **Health**: `GET http://127.0.0.1:9800/health` (GSV `keep_live.telenetis`, fresh = up).
- **NEXT pointer**: [`NEXT_SESSION_PROMPT.md`](./NEXT_SESSION_PROMPT.md).
