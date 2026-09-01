# Plan — Unified keep-live: GSV + Telenetis + llama-rs + OmniRoute + OpenBot MCP (bands 223-227)

**Goal:** GSV stays the always-on supervisor for the whole VDT kit. Today only `gsv-server :9999` is kept live by `gsv-watchdog` (probe 3s, threshold 2, cooldown 10s, heartbeat `target/live/watchdog.json`). Telenetis `:9800` (standalone Axum, already has `TELENETIS_WEBHOOK_SECRET`, `spawn_poll_loop` 5s, WS `/ws` keep-alive 25s + SSE) runs unsupervised on Windows — Cursor terminal abort = offline until manual restart. llama-rs (`S:/rust/llama-rs`) is a CLI (`Model::load_staged`, `GSV_LIVE=1` → `127.0.0.1:9999` progress push 120ms timeout, 0.031 tok/s on 5500U) with no daemon liveness, no `PRODUCTS.md` health wire beyond `cargo xtask products` discovery. OmniRoute (`S:/rust/omniroute`, node, `omniroute` row `npm test`, `registered` true) has no Rust keep-live — GSV only knows its OmniRouter proxy/catalog.

This plan makes **GSV the health aggregator + watchdog supervisor** for all four, with OpenBot MCP as the uniform control plane.

## Research snapshot (2026-08-31)

| Service | Current live shape | Gaps |
|---------|-------------------|------|
| **GSV** | `gsv-server` `target/live/gsv-server.exe` loop via `cargo xtask live`; `gsv-watchdog` probes `/api/health` (ok + `version_lag` + `disk_ok`), writes heartbeat, `needs_lockstep(debug_newer \|\| version_lag)` → POST `/api/update/apply`; Galaxy `GET /api/watchdog` + `GET /api/health` (version_lag, watchdog_alive, disk) + MCP `gsv_watchdog` `gsv_health` `gsv_disk` | Single-target only; `debug_newer_server` ignores telenetis/llama-rs staleness; no sub-service aggregation |
| **Telenetis** | `telenetis` crate under `S:/rust/GSV/telenetis`, port 9800, `docker-compose`/`systemd`/`telenetis-live` supervisor + 7/7 boot-verify, but Windows path = manual `cargo run` ; poll 5s `GsvClient` 5s/3s timeouts, webhook secret `ct_eq` 403, `freshness_now=Utc::now()` | No `gsv-watchdog` supervision on Windows; no live-copy `target/live/telenetis.exe`; `GET /api/health` on GSV does not know if 9800 is down; no MCP tool for telenetis health |
| **llama-rs** | `S:/rust/llama-rs` 99.46% Rust, `llama-cpp-2 0.1.154`, CLI `llama_rs --mmap/--mlock --progress --seed`, `src/main.rs:19 gsv_report_progress` (thin TcpStream) + `staged.rs` disk→RAM + `ContextParams` presets + benches 0.031 tok/s | No daemon mode, no health endpoint, no watchdog registration; OmniRouter catalog has no local llama provider (ticket `t-1788009924340776700` open: BunkeRock lama 2.8) |
| **OmniRoute** | Node, `v3.8.x rail`, `npm test`, `AGENTS.md` strict docs check (`rg -n`), OpenAI compat `/v1/chat/completions` | Not supervised by Rust watchdog; GSV OmniRouter (`GET /api/omni`, `POST /api/omni/v1/chat/completions` with `X-Omni-Dry-Run`) can proxy to it but has no liveness probe; boot-verify already probes 9800, not omniroute |
| **MCP** | `gsv_mcp_openbot` 56 tools, 13 resources, `GET /mcp` catalog_notify/listed_tool_count/catalog_stale | No telenetis/llama/omniroute health tools; no `gsv_keep_live` aggregate |

## What (bands 223-227)

### Band 223 — Health aggregation (GSV box, no supervision yet) — P0
- New `boxes/keep_live.rs` (or extend `health.rs`+`watchdog.rs`): `KeepLiveReport { gsv, telenetis, llama_rs, omniroute }` each `{ alive, url, version?, lag? }`.
- Probes: `http://127.0.0.1:9999/api/health` (self), `127.0.0.1:9800/health` (telenetis), `http://127.0.0.1:20128` or `GSV /api/omni` for omniroute, llama-rs via `S:/rust/llama-rs/target/live/heartbeat` or no-op if not running (graceful). Timeouts 1s. `ok` stays true on sub-service failure (like `disk_ok` band 181).
- Wire `GET /api/keep-live` + wire into `GET /api/health { keep_live }` (additive, no break). MCP `gsv_keep_live` (read-only).
- Galaxy: health card rows for keep-live; new studio card `keep-live` (`CARD_NAMES` +1).
- Tests: health aggregation unit + server contract (sub-service down → ok true + keep_live.*.alive false).

### Band 224 — Telenetis keep-live (Windows parity) — P1
- `cargo xtask live` copies `telenetis` debug → `target/live/telenetis.exe` (like `gsv-mcp` band 158). `cargo xtask telenetis-live` live loop (or merge into `gsv-live` with second child) + `watchdog` multi-probe: `debug_newer_telenetis` + POST `telenetis` apply/restart path (or spawn live directly — telenetis has no `/api/update/apply`, so watchdog respawns).
- `gsv-watchdog` heartbeat gains `telenetis_alive` / `telenetis_debug_newer`; `wire()` reports them; Galaxy watchdog card shows telenetis row.
- Docs: `GSV_SERVER.md` live-copy matrix, `telenetis/README` Windows keep-live, `GSV_BOXES.md`.
- Gate: `telenetis` still builds `0.222.0` → 223 version; `cargo test` telenetis 177 passes.

### Band 225 — llama-rs keep-live + OmniRouter local provider — P1
- `llama-rs` heartbeat: when `GSV_LIVE=1` or new `LLAMA_RS_HEARTBEAT=1`, write `target/live/llama_heartbeat.json` (pid, model, ts, alive) every 30s or on inference start. GSV `keep_live` reads it (file probe, not HTTP). Optional `POST /api/llama/heartbeat` if daemonized.
- OmniRouter catalog: add local provider `bunke-rock` / model `lama-2.8` (2-bit Qwen 27B GGUF, `models/Qwen3.8-27B-UD-IQ2_XXS.gguf`) as `kind: local`, `base_url: http://127.0.0.1:11434` or file-path, wired so `gsv_omni_route task=rust` can pick it; ticket `t-1788009924340776700` close.
- `cargo xtask products` already discovers `llama-rs`; scan enriches heartbeat path.
- Tests: provider catalog has `bunke-rock`, keep-live reads heartbeat file.

### Band 226 — Unified Galaxy keep-live dashboard + MCP + E2E — P2
- Galaxy `keep-live` card: 4 rows (GSV/telenetis/llama-rs/omniroute) with alive/dot, version, lag, uptime, probe latency. `GET /api/ui/card/keep-live` Rust-rendered.
- MCP: `gsv_keep_live` + `gsv_telenetis_health` + `gsv_llama_status` aggregate; `GET /mcp` `keep_live` summary; `gsv_drain` names keep-live.
- `scripts/keep-live-boot-verify.sh` (or extend `telenetis-boot-verify.sh` → `keep-live-boot-verify`) probes 9999 + 9800 + llama heartbeat + omniroute; 4/4 pass against live.
- Stand-smoke: `keep-live` card + health `keep_live` shape.
- Perf: probes share 1s timeout, no extra 5s poll duplication; watchdog interval unchanged (3s).

### Band 227 — OmniRoute node keep-live (optional, owner pick)
- If owner runs `omniroute` locally (`npm run dev` / `npm start`), add optional `OMNIROUTE_URL` probe to keep-live (fail-open, like omniroute usage pull). Do not start node from Rust (owner opt-in). Document `S:/rust/omniroute` env matrix.

## Non-goals
- Do not port-forward `/mcp` to public internet (tunnel stays `cargo xtask tunnel` opt-in).
- Do not add MCP `products/open`, `update/apply`, or `tunnel` tools.
- Do not add Python.

## Acceptance
- `GET /api/health` has `keep_live` with 4 entries, `ok` true even if telenetis down (same `disk_ok` pattern).
- `cargo xtask live` copies telenetis to `target/live`; `gsv-watchdog` probes 9800 and respawns on miss; Galaxy shows telenetis alive.
- `llama-rs` heartbeat file read by GSV; OmniRouter catalog lists local `bunke-rock` `lama-2.8`.
- Galaxy `keep-live` card + MCP `gsv_keep_live` + stand-smoke + boot-verify 4/4 pass.
- `cargo fmt --all` · `cargo clippy --all-targets` 0 · `cargo test` GSV + telenetis + llama-rs pass · `gsv-loc-audit --stretch-96` ≥96% · `gsv_vision_sync --check` 0.
