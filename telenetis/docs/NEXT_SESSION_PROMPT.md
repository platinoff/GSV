# Telenetis — Next Session Prompt

1. `cargo test` in `S:/rust/GSV/telenetis` — keep green (280+4+4); check
   `GET /health` on `:9800` and GSV `keep_live.telenetis` row
   (MCP `gsv_telenetis_health`; band 224 watchdog respawn).
2. Gates needing owner/phones (do NOT code around): token rotate
   (`t-1789606660639448500`), group ownership (`t-1789606667836911200`),
   phone verify — worker-controls, tensor PoC, P2P byte measure, compat probe.
3. Follow-ups: host-side chat sessions (poolAI repo, payload already supports
   `history`); Bot API 10.x origin-hardening watch vs tunnel URL (CONCEPT §1).
4. One commit in the GSV repo (this crate lives in the kit tree), message style:
   `# Telenetis: <what>`; no `git add -A`, no `data/*` staging.

Sources: [`HANDOFF_NEW_SESSION.md`](./HANDOFF_NEW_SESSION.md) ·
[`ops.md`](../../docs/telenetis/ops.md) · [`README.md`](../../docs/telenetis/README.md) ·
[`CONCEPT.md`](../../docs/telenetis/CONCEPT.md).
