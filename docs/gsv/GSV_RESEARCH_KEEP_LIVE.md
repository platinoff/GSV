# GSV keep-live research — GSV + Telenetis + llama-rs + OmniRoute + OpenBot MCP

**Status:** Band **223 ✅** (aggregation live); next **224** (telenetis live-copy + watchdog multi-probe). **Date:** 2026-09-01. **Owner pick.**
**Context:** GSV is the VDT entry workspace (`S:\rust\GSV`) and always-on supervisor (`:9999` + watchdog). Telenetis (`S:\rust\GSV/telenetis` `:9800`) bridges Godfather to Mini App; llama-rs (`S:/rust/llama-rs`) runs Qwen 27B IQ2_XXS 0.031 tok/s; OmniRoute (`S:/rust/omniroute`, node) proxies AI. All must stay live with one MCP surface.

## 1. Current keep-live (audit 2026-08-31)

### GSV
- `gsv-server` loop: `cargo xtask live` copies `target/debug/gsv-server.exe` → `target/live/gsv-server.exe` and execs `--host 127.0.0.1 --port 9999`; `cargo test` may overwrite debug without OS error 5.
- `gsv-watchdog` (`src/boxes/watchdog.rs:30`): `DEFAULT_INTERVAL_SECS 3`, `FAIL_THRESHOLD 2` (~6s grace), `COOLDOWN 10s`, `MAX_AGE 20s`; `tick()` + `should_respawn()` + `spawn_live()` (DETACHED_PROCESS|CREATE_NEW_PROCESS_GROUP|CREATE_NO_WINDOW, never CREATE_BREAKAWAY_FROM_JOB); heartbeat `target/live/watchdog.json` {ts, pid, consecutive_failures, last_action, last_apply_status, lockstep_note, bin_version}.
- Lockstep: `debug_newer_server()` || health `version_lag` → POST `/api/update/apply` → SSE offline → exit → supervisor recopy; stale watchdog hops via `successor_plan()` each tick (`hop_successor`, `stop_peer_watchdog`), never POSTs apply; `onshot_bin_version` + `peer_watchdog_running` (fresh + pid_is_alive) prevent split-brain.
- Health: `GET /api/health` {ok, crate_version, version_lag, watchdog_alive, disk_ok/target_gb/violation} — `ok` stays true on disk S0 (band 181 `disk_ok` pattern). `GET /api/watchdog` {alive, debug_newer, server_debug_newer, watchdog_debug_newer, version_lag}. MCP `gsv_health` `gsv_watchdog` `gsv_disk` mirror it.
- Pitfall on 2026-08-31: live health reported `version 0.222.0` + `crate_version 0.196.0` + `version_lag true` while watchdog was `0.196.0` — stale live copy needed `cargo xtask live` recopy before watchdog lockstep could fire.

### Telenetis
- Rust Axum 0.8, port 9800, `src/main.rs:28 spawn_poll_loop` (GSV bus `messages` key, 5s), `src/bot/webhook.rs run_polling` fallback, `src/stream/ws.rs` WS keep-alive 25s + broadcast Lagged drop-tolerant, SSE 30s, `GET /api/live/config` reconnect policy (base 1s/cap 30s/max 6, splitmix jitter), `GET /api/snapshot?lang=` consolidated, `src/security/initdata.rs` HMAC-SHA256 + `X-Telegram-Bot-Api-Secret-Token` ct_eq 403, timeouts `GsvClient` 5s/3s `TelegramBot` 65s/10s.
- Deploy: `Dockerfile` multi-stage + `docker-compose.yml` restart unless-stopped + `.env.example` + `deploy/systemd/telenetis.service` + `scripts/telenetis-boot-verify.sh` 7/7 pass — but Windows keep-live is manual (`cargo run` in Cursor terminal); abort → `:9800` offline until manual restart; no `target/live/telenetis.exe`, no watchdog probe.
- GSV health does not know 9800 is down (no aggregation).

### llama-rs
- `S:/rust/llama-rs` 99.46% Rust (`cargo xtask check/loc`), `llama-cpp-2 0.1.154` (vendored, 4 Windows-GNU build.rs patches), CLI `llama_rs <model> --mmap/--no-mmap --mlock --progress --seed --max-tokens`, `src/safe/staged.rs` StagedLoadOptions (use_mmap/mlock/with_progress), `src/main.rs:19 gsv_report_progress` thin TcpStream to `127.0.0.1:9999` (120ms, no dep), `docs/BENCHMARKS.md` 0.031 tok/s TTF 248s (5500U mmap), E2E `LLAMA_RS_TEST_MODEL=... cargo test generate_with_model_if_env_set` 156s pass.
- No daemon/heartbeat endpoint; GSV `cargo xtask products` discovers it (`registered: true`, `kind: rust`) but health is file-less; OmniRouter catalog has 19 providers but no local llama (ticket `t-1788009924340776700` BunkeRock lama 2.8 open).

### OmniRoute
- Node, `PRODUCTS.md` `kind: node` `npm test`, `AGENTS.md` strict `rg -n` docs check, `v3.8.x` rail → LTS 3.9 → 4.0 modular. GSV has `boxes/omni` (`GET /api/omni`, proxy `POST /api/omni/v1/chat/completions` with `X-Omni-Dry-Run`, `GET /api/omni/route task=rust|web` timer-aware, `quota.rs` `data/omni_quota.json`, `omni.toml`). Fail-open `GET {base}/api/usage/history` (band 155) shows precedent: probe but never treat upstream as `ok=false`.
- Not supervised by Rust watchdog; no boot-verify for omniroute.

### MCP
- `gsv_mcp_openbot` 57 tools, 13 resources, `GET /mcp` catalog_notify/listed_tool_count/catalog_stale; Cursor `type: http` `http://127.0.0.1:9999/mcp`, stdio `target/live/gsv-mcp.exe`. Band 223 added **`gsv_keep_live`** aggregate (read-only); legacy `gsv_health`/`gsv_watchdog` kept for compat. No telenetis/llama/omniroute health tools; no aggregate — done (single aggregate covers all four).

## 2. Optimization levers

1. **Single watchdog, multi-target probe** — extend heartbeat to `{ gsv_alive, telenetis_alive, llama_rs_alive, omniroute_alive }` with per-target `debug_newer_*`; fail-open so `health.ok` and `watchdog_alive` stay true when a peer is down (same as `disk_ok`). Share 3s interval, 1s per-probe timeout, no extra 5s polling loop duplication. **Band 223 built the aggregation half** (`boxes/keep_live.rs`, probes 1s, fail-open `ok`); the watchdog multi-probe half is band 224.
2. **Live-copy parity** — `cargo xtask live` already copies `gsv-server` + `gsv-mcp` + `gsv-watchdog`; add `telenetis` (`target/live/telenetis.exe`) and document `cargo xtask telenetis-live` as alias. Telenetis has no `/api/update/apply`, so watchdog respawns it directly instead of POSTing apply. (**band 224**)
3. **Heartbeat-over-file for llama-rs** — daemonizing llama-rs is overkill (CLI, not server). Lightweight: when `GSV_LIVE=1` or `LLAMA_RS_HEARTBEAT=1`, write atomic `target/live/llama_heartbeat.json` {pid, model, epoch_secs} every 30s or on `Model::load_staged` success; GSV reads it (file probe, no HTTP — **done band 223**, `LLAMA_HEARTBEAT_PATH`, stale if `age > 60s` → `alive:false`). Old JSON is stale if `age > 60s` → `alive:false`. (**llama-rs writes it in band 225**)
4. **OmniRoute as OmniRouter local provider** — add `catalog.rs` entry `id: "bunke-rock", kind: local, models: [{id:"lama-2.8", ctx 32768}]` pointing at `models/Qwen3.8-27B-UD-IQ2_XXS.gguf`; wire so `select_provider(task=rust, prefer_free)` skips cooling and picks it; persist in `omni.toml` routing. (**band 225**)
5. **Galaxy single card > N cards** — one `keep-live` studio card with 4 rows (alive dot, version/lag, latency, uptime) replaces 4 separate health rows; `GET /api/ui/card/keep-live` Rust-rendered, `CARD_NAMES` 43 (**done band 223**), `health` card stays minimal (disk/watchdog only).
6. **MCP uniform surface** — add `gsv_keep_live` aggregate (read-only) + keep `gsv_health`/`gsv_watchdog` for compat; `GET /mcp keep_live` summary; `gsv_drain` prompt names keep-live so `абракадабра` steers next session. (**`gsv_keep_live` done band 223 → 57 tools**)

## 3. Risks & non-goals
- Do not public-tunnel `/mcp` (owner opt-in `cargo xtask tunnel` only).
- Do not add MCP `products/open`, `update/apply`, or tunnel starters.
- Do not port `telenetis` or `llama-rs` into `gsv-server` binary (separate processes, separate repos).
- Do not start `omniroute` from Rust (node run stays owner `npm run dev`); probe is read-only.

## 4. Phasing (next abracadabra sessions)
- **223 ✅** aggregation only (wire + probe + MCP read + Galaxy rows) — done: no respawn, fail-open `ok`, `gsv_keep_live` MCP (57 tools).
- **224** telenetis live-copy + watchdog multi-probe + respawn.
- **225** llama-rs heartbeat file + local provider catalog (`bunke-rock`).
- **226** unified dashboard + MCP aggregate polish + boot-verify + stand-smoke.
- **227** (optional) omniroute probe when owner runs it.

Sources: `src/boxes/watchdog.rs`, `src/boxes/health.rs`, `docs/gsv/GSV_SERVER.md:135`, `docs/telenetis/README.md:2`, `telenetis/src/main.rs:28`, `S:/rust/llama-rs/src/main.rs:19`, `S:/rust/llama-rs/docs/HANDOFF.md:6`, `S:/rust/GSV/docs/gsv/PRODUCTS.md:15-20`.
