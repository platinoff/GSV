# Telenetis — Next Session Prompt

**Owner product pick: telenetis** (next `agi` in the GSV window skips AskQuestion).

0. Keep-live GSV `:9999` + this `:9800`. Redmi 9 = Wi-Fi debug rabbit.
   Host ADB: `S:/rust/GSV/target/adb/platform-tools/adb.exe` (`adb devices` is
   empty until pair). Owner: Developer → Wireless debugging → pair by code →
   send IP:port + 6 digits. Then `adb pair` + `adb connect` → screenshots / taps
   / logcat from the hub. Never `:8091` off-box. Never a second GSV.
1. `cargo test` in `S:/rust/GSV/telenetis` — keep green (280+4+4); check
   `GET /health` on `:9800` and GSV `keep_live.telenetis` row
   (MCP `gsv_telenetis_health`; band 224 watchdog respawn).
2. Gates needing owner/phones (do NOT code around): token rotate
   (`t-1789606660639448500`), group ownership (`t-1789606667836911200`),
   phone verify — worker-controls, tensor PoC, P2P byte measure, compat probe.
   Two phones in the chat only after a green emulator
   (`cargo test --test phone_emulator`), following
   [`TWO_PHONES.md`](../../docs/telenetis/TWO_PHONES.md) (watch
   `/api/edge/tracker/status` while they tap).
3. Follow-ups: host-side chat sessions (poolAI repo, payload already supports
   `history`); Bot API 10.x origin-hardening watch vs tunnel URL (CONCEPT §1).
4. One commit in the GSV repo (this crate lives in the kit tree), message style:
   `# Telenetis: <what>`; no `git add -A`, no `data/*` staging.

Sources: [`HANDOFF_NEW_SESSION.md`](./HANDOFF_NEW_SESSION.md) ·
[`ops.md`](../../docs/telenetis/ops.md) · [`README.md`](../../docs/telenetis/README.md) ·
[`CONCEPT.md`](../../docs/telenetis/CONCEPT.md).
