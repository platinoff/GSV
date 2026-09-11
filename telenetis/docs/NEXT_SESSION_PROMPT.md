# Telenetis — Next Session Prompt

1. `cargo test` in `S:/rust/GSV/telenetis` — keep green; check
   `GET /health` on `:9800` and GSV `keep_live.telenetis` row
   (MCP `gsv_telenetis_health`; band 224 watchdog respawn).
2. Continue role-store / Telegram bus parity with the GSV Godfather relay
   (`docs/gsv/GSV_SETTINGS_TELEGRAM.md`, kinds `sync|presence|claim|done|reclaim`).
3. One commit in the GSV repo (this crate lives in the kit tree), message style:
   `# Telenetis: <what>`; no `git add -A`, no `data/*` staging.

Sources: [`HANDOFF_NEW_SESSION.md`](./HANDOFF_NEW_SESSION.md) ·
[`ops.md`](../../docs/telenetis/ops.md) · [`README.md`](../../docs/telenetis/README.md).
