# Edge deployment plan — hub topology + settings (2026-09-14)

GSV is the hub (`:9999` + MCP openbot). The rust folder that holds GSV is the
host; everything else is a **portable plugin**. Environment security first.
**Telenetis is a Telegram-shell plugin, not a second brain.** Phone compute
joins as an APK peer through hub `/api/edge` (canon [GSV_AGI_PATH.md](GSV_AGI_PATH.md)).
Single source of truth for ports, autostart entries, env, and gaps.
Hub-side virtual-datacenter must-have list (ALLBGP topology, VRAM capacity,
rebalance, burst seats, security): see [GSV_VDC.md](GSV_VDC.md).

## 1. Topology (all on this box, hidden, no consoles)

| Service | Addr | Binary | Autostart | Data / logs |
|---|---|---|---|---|
| GSV hub + MCP | `:9999` | `GSV/target/live/gsv-server.exe` + watchdog | always-on kit | `GSV/data/` |
| llama provider | `:8080` | `llama-rs/target/release/llama_serve.exe` +27B mmap | HKCU `llama-serve` | `llama-rs/target/live/llama_serve.log`, heartbeat json |
| poolAI grid | `:8091` | `poolAI/target/debug/poolai.exe` (`POOLAI_HTTP_PORT`) | HKCU `poolai-edge` → `target/edge_start.vbs` | `poolAI/target/live/edge_data/` (binds+tasks) |
| edge executor | — | `llama-rs/target/debug/llama_edge.exe` as `edge-pc-01` (+ `a54-01`) | HKCU `llama-edge` → `target/edge_start.vbs` (with signing key) | `llama-rs/target/live/llama_edge.log` |
| Telenetis shell | `:9800` | `telenetis/target/live/` + `telenetis-live.exe` supervisor | HKCU `telenetis-edge` → `target/telenetis_start.vbs` | identity + Mini App chrome; not orchestrator |
| ngrok tunnel | `:4040` api | Temp `ngrok.exe` v3.39.11 | HKCU `ngrok-tunnel` | reserved domain `atonable-alibi-unwilling` |
| OmniRoute gate | `:20128` | node (no runtime, policy: Rust only) | DOWN by decision — role served by the hub itself (OmniRouter box) | keep-live probe stays as down-signal |

## 2. Settings inventory

- `llama_serve`: deep `:8080` model `models/Qwen3-30B-A3B-UD-IQ2_XXS.gguf`
  (band 235 MoE swap; 27B dense reserved), fast `:8082`
  `models/Qwen2.5-1.5B-Instruct-Q4_K_M.gguf`, `LLAMA_RS_HEARTBEAT=1`,
  `--log-file`, MinGW DLLs beside exe (0xC0000135 otherwise), `--host 0.0.0.0`
  (band 233 LAN-first default).
- `llama_edge`: `--coordinator http://127.0.0.1:8091`, operator signing key
  via `LLAMA_EDGE_SIGNING_KEY` (dev `[7;32]` retired), `LLAMA_SERVE_URL` → `:8080`.
- poolAI: `POOLAI_HTTP_PORT=8091` (`:8080` is llama!), `POOLAI_VIRTUAL_NODE_DATA_DIR`
  set (binds survive restarts), dev admin `admin/admin123` (change in prod),
  `POOLAI_TELEGRAM_SEAT_LIMIT=5` (flat), capability verify = operator key
  `221c…8235` (dev key rejected — verified live).
- Telenetis `.env`: bot token, `GSV_URL=:9999`. **Grid calls go through hub
  `/api/edge`**, not `POOLAI_URL=:8091` off-box. `NGROK_BIN`=Temp path,
  `NGROK_AUTHTOKEN` set, webhook URL = reserved domain.
  `TELENETIS_PUBLIC_URL` unset (auto via tunnel).
- GSV Godfather: channel `@GSV_OFFICIAL`, `allowed_user_ids` includes owner
  Telegram id `5035500793` + `platinofff` + session words (`solo`/`squad`/
  `local`/`telenetis-01`); chat role host, squad_cap 3. Inbound poller ingests
  allowlisted leftover chat as tickets (`classify_inbound` `chat`); empty
  allowlist still skips plain chat. `/ticket` and hooks unchanged.
- Telegram↔poolAI: binding `5035500793 ↔ a54-01`, VM `a54-01-vm` Running.
  Proto pin for workers: `f5b9bd39` (5.0.0; master is 6.0, no handshake).

## 3. Gaps → tickets (execution order)

1. Telenetis autostart. [done 2026-09-14: HKCU telenetis-edge]
2. Shard map view from completions (telenetis, open).
3. `allowed_user_ids` allowlist (owner dashboard action) + session identity (gsv, open).
4. Grid hardening: seats limit + prod keys (poolai, done 2026-09-14).
5. Hub offline routing + IDE onboarding (gsv ×2, open).
5b. Hub single-entry proxy (gsv) — done 2026-09-17 band 237 (`/api/edge`).
6. OmniRoute gateway UP (omniroute, open; node). **Policy-down** — do not start.
7. MTP draft decision (llama-rs, open).
8. Teach prompt + initData audit (gsv/telenetis, open).
9. AGI path lockstep (gsv) — 2026-09-17: VDC “AGI path” table; Telenetis = shell;
   APK + WiFi debug = `virtual_node` via `/api/edge`; no second hub in Mini App.
10. APK LAN peer (gsv × phone) — **landed** (host-side): `gsv-apk join --json`
    plans GET `/api/edge/health` + POST `discovery/register-remote` +
    `virtual-nodes/{peer}/pool/join`. `--live` POSTs `X-Gsv-Edge-Token` (never
    echoed). WiFi debug stays on-device disk/settings/debug. Do not clone
    Telenetis into the APK. Native package contract **landed**: `gsv-apk
    package --json` (`org.gsv.apk`, no WebView/Java/gradle in product;
    manifest generated into `target/live/apk/`). NDK/cargo-apk binary is later.
11. Freeze Telenetis feature surface (telenetis) — **landed**: identity + Mini App
    chrome + LAN only (`gsv-apk freeze --json`, surface=shell). Tensor/WebGPU
    is probe. No new KVM/swarm-as-OS. Telegram proxy to APK.
12. Service account for Telenetis/APK replacing `admin/admin123` (gsv) —
    **landed**: clients use hub `/api/edge` + `GSV_EDGE_TOKEN` /
    `TELENETIS_EDGE_TOKEN` (`gsv-apk service-account --json`;
    `edge.service.kind=edge_token`; Telenetis no longer compiles
    `admin/admin123`). poolAI login is opt-in env only.
13. Godfather allowlisted free-text ingest (gsv) — **landed**: leftover
    non-slash chat from an allowlisted sender becomes a ticket. Empty
    allowlist still skips (no flood). Unknown `/command` and session lines skip.

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
| `GSV_WATCHDOG_LOCKSTEP` | GSV env (watchdog) | `0` / `--no-lockstep` during drains | auto-`/api/update/apply` off (ticket-flow rebuilds must not bounce `:9999`); heartbeat `lockstep-off`; respawn-on-failure stays; apply stays operator-driven (`cargo xtask live`) |
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

No Termux (owner). Phone compute joins the hub as an APK peer via `/api/edge`;
Telenetis is Telegram identity + Mini App shell only. Never expose `:8091`
off-box. LAN `192.168.2.238`, remote via tunnel with free-tier interstitial
caveat. Rust-first, MSYS2 bash, one commit per drain, no push mid-drain,
secrets never echoed.
