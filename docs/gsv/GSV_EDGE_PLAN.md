# Edge deployment plan — hub topology + settings (2026-09-14)

GSV is the hub (`:9999` + MCP openbot). Everything else hangs off it.
Single source of truth for ports, autostart entries, env, and gaps.

## 1. Topology (all on this box, hidden, no consoles)

| Service | Addr | Binary | Autostart | Data / logs |
|---|---|---|---|---|
| GSV hub + MCP | `:9999` | `GSV/target/live/gsv-server.exe` + watchdog | always-on kit | `GSV/data/` |
| llama provider | `:8080` | `llama-rs/target/release/llama_serve.exe` +27B mmap | HKCU `llama-serve` | `llama-rs/target/live/llama_serve.log`, heartbeat json |
| poolAI grid | `:8091` | `poolAI/target/debug/poolai.exe` (`POOLAI_HTTP_PORT`) | HKCU `poolai-edge` → `target/edge_start.vbs` | `poolAI/target/live/edge_data/` (binds+tasks) |
| edge executor | — | `llama-rs/target/debug/llama_edge.exe` as `edge-pc-01` (+ `a54-01`) | HKCU `llama-edge` → `target/edge_start.vbs` (with signing key) | `llama-rs/target/live/llama_edge.log` |
| Telenetis edge | `:9800` | `telenetis/target/live/` + `telenetis-live.exe` supervisor | HKCU `telenetis-edge` → `target/telenetis_start.vbs` | roles jsonl (gitignored) |
| ngrok tunnel | `:4040` api | Temp `ngrok.exe` v3.39.11 | HKCU `ngrok-tunnel` | reserved domain `atonable-alibi-unwilling` |
| OmniRoute gate | `:20128` | node (no runtime, policy: Rust only) | DOWN by decision — role served by the hub itself (OmniRouter box) | keep-live probe stays as down-signal |

## 2. Settings inventory

- `llama_serve`: model `models/Qwen3.8-27B-UD-IQ2_XXS.gguf`, `LLAMA_RS_HEARTBEAT=1`,
  `--log-file`, MinGW DLLs beside exe (0xC0000135 otherwise), `--host` loopback
  (LAN mode = `0.0.0.0`, needs firewall rule).
- `llama_edge`: `--coordinator http://127.0.0.1:8091`, operator signing key
  via `LLAMA_EDGE_SIGNING_KEY` (dev `[7;32]` retired), `LLAMA_SERVE_URL` → `:8080`.
- poolAI: `POOLAI_HTTP_PORT=8091` (`:8080` is llama!), `POOLAI_VIRTUAL_NODE_DATA_DIR`
  set (binds survive restarts), dev admin `admin/admin123` (change in prod),
  `POOLAI_TELEGRAM_SEAT_LIMIT=5` (flat), capability verify = operator key
  `221c…8235` (dev key rejected — verified live).
- Telenetis `.env`: bot token, `GSV_URL=:9999`, `POOLAI_URL=:8091` (explicit),
  `NGROK_BIN`=Temp path, `NGROK_AUTHTOKEN` set, webhook URL = reserved domain.
  `TELENETIS_PUBLIC_URL` unset (auto via tunnel).
- GSV Godfather: channel `@GSV_OFFICIAL`, `allowed_user_ids` = EMPTY (spoof
  risk — sec band), chat role host, squad_cap 3.
- Telegram↔poolAI: binding `5035500793 ↔ a54-01`, VM `a54-01-vm` Running.
  Proto pin for workers: `f5b9bd39` (5.0.0; master is 6.0, no handshake).

## 3. Gaps → tickets (execution order)

1. Telenetis autostart. [done 2026-09-14: HKCU telenetis-edge]
2. Shard map view from completions (telenetis, open).
3. `allowed_user_ids` allowlist (owner dashboard action) + session identity (gsv, open).
4. Grid hardening: seats limit + prod keys (poolai, done 2026-09-14).
5. Hub offline routing + IDE onboarding (gsv ×2, open).
6. OmniRoute gateway UP (omniroute, open; node).
7. MTP draft decision (llama-rs, open).
8. Teach prompt + initData audit (gsv/telenetis, open).

## 4. Cross-project settings matrix (research 2026-09-14)

| Var | Where | Value here | Notes |
|---|---|---|---|
| Ports | — | GSV 9999 · llama 8080/8082 · poolAI 8091 · telenetis 9800 · omniroute 20128 · ngrok API 4040 | poolAI default 8080 collides with llama — always override |
| `GSV_TELEGRAM_BOT_TOKEN` | GSV env | set (secret) | wins over file, never written back |
| `TELENETIS_BOT_TOKEN` / `NGROK_AUTHTOKEN` / webhook secret | telenetis `.env` (gitignored) | set (secrets) | never stage, never echo |
| poolAI admin | compiled default | `admin/admin123` | CHANGE for anything beyond LAN |
| capability keys | poolAI env + edge env | operator `221c…8235` | dev `[7;32]` retired everywhere except poolAI unit tests |
| `POOLAI_TELEGRAM_SEAT_LIMIT` | poolAI env | `5`, flat | raise when workers grow |
| `POOLAI_VIRTUAL_NODE_DATA_DIR` | poolAI env | `target/live/edge_data` | binds+tasks survive restarts; jobs default JSON, leases in-memory |
| `LLAMA_EDGE_SIGNING_KEY` | edge env (HKCU TR/vbs) | operator privkey | never echo, never commit |
| `LLAMA_SERVE_URL` / `LLAMA_SERVE_FAST_URL` | edge env | `:8080` / `:8082` | tier routing |
| `OMNI_*_API_KEY` | GSV env | out of `omni.toml` | keys never in toml |
| `GSV_ACTOR`/`GSV_IDE`/`GSV_MODEL`/`GSV_AGENT` | GSV env | session identity | feeds claims + fingerprints |
| Jail/allowed list | GSV `data/gsv_settings.json` | `["5035500793","solo","squad","local","telenetis-01"]` | owner edits via dashboard only |

Secrets hygiene: real secrets live in env or gitignored `.env`/data files only.
Known gaps: poolAI admin password, job/lease stores memory-only, omniroute
has no runtime at all.

## 5. GSV ↔ poolAI concept map (keep in sync; poolAI is ahead in places)

| GSV hub | poolAI grid | Note |
|---|---|---|
| presence heartbeat + leases | `heartbeat-remote` + capabilities + job leases | same cycle |
| bus claim/done/reclaim | virtual-node tasks poll/complete | tasks carry payloads, bus carries hints |
| ranks ladder (earned) | `rewards_service` (credits dormant) + reputation | poolAI credits OFF — do not mirror until live |
| `allowed_user_ids` allowlist | signed capability docs (ed25519) | poolAI is STRICTER — candidate pattern for GSV identity evolution |
| telegram seats + bindings | `telegram_edge` seats + bindings + wallets | same shape |
| squad_cap / jail | pools + virtual nodes + worker health | — |
| fingerprints.jsonl | completion records + virtual_node_store | both git-tracked JSONL-ish audit |

## 6. Rules carried over

No Termux (owner). Everything through poolAI services; phones see Telenetis
only (no 127.0.0.1 leaks; LAN `192.168.2.238`, remote via tunnel with free-tier
interstitial caveat). Rust-first, MSYS2 bash, one commit per drain, no push
mid-drain, secrets never echoed.
