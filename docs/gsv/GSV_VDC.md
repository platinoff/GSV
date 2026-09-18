# GSV_VDC.md — hub-side virtual-datacenter must-have list (ALLBGP)

Canon for treating the fleet (PC 5500U · A54 · Pi4 later) as one **virtual data
center** with `gsv-server :9999` as the operator brain. Owner policy stays:
**100 % Rust hub, No Node runtime, No Termux workers, no mid-drain push**.
Companion to `GSV_EDGE_PLAN.md` (ports + matrix) and the plan
`docs/superpowers/plans/2026-09-14-hub-vdc-allbgp.md` (band tickets).

**ALLBGP idea:** a BGP edge keeps the *full* routing table, not per-flow probes.
The hub must likewise hold the whole grid as one durable table: every device,
capacity, model placement, seat, tier, and health edge — mirrored from poolAI
`:8091`, kept in GSV (poolAI's peers/VMs/seats are memory-only), never
point-to-point.

**Ecosystem canon:** [`GSV_AGI_PATH.md`](./GSV_AGI_PATH.md) (`gsv://docs/agi-path`) —
rust folder is the host; every other project is a portable plugin; environment
security first; maximum Rust; `agi` is the session rule.

## AGI path (owner 2026-09-17)

The ecosystem brain is **this hub**, not Telenetis and not a phone APK.
The rust folder that contains GSV is the only required host tree; poolAI,
llama-rs, Telenetis, and the APK are **portable plugins**. Telenetis as
“the app that handles everything” is a congestion path: Mini App WebView,
webhook/ngrok, two bots, and direct `:8091` calls pile up until even small
models look “dumb.” Owner policy: **GSV orchestrates; plugins execute.
Environment security first.**

| Layer | Who | Does | Does not |
|---|---|---|---|
| Brain | GSV `:9999` + MCP | tickets, Omni, grid, `/api/edge`, keep-live, placement | run tensors; speak Telegram as a product UI |
| Grid | poolAI `:8091` | seats, virtual-nodes, jobs | be reachable off-box (hub proxy only, §6) |
| Tensors | llama-rs `:8080` / `:8082` | deep / fast local tiers | own the fleet map |
| Telegram shell | Telenetis `:9800` | identity, Mini App chrome, LAN when the bot is down | grow tensor / KVM / swarm / board-as-OS |
| Phone peer | APK + WiFi debug | register as `virtual_node` via `/api/edge/{*path}` | become a second Telenetis |

Path (b) in §3 (WebGPU **inside** the Mini App) stays a probe, not the
control plane. The intended phone join is the **APK peer**: WiFi debug already
exists; the APK talks to the hub edge-proxy with a token, never to `:8091`.
Service-account rotation for Telenetis/APK (`t-1789392387275733400`) is the
security leftover so those clients stop using `admin/admin123`.

Godfather inbound (`gsv_telegram_poll`) classifies `/ticket`, hook phrases,
bus JSON, and leftover chat. Allowlisted non-bot text in `@GSV_OFFICIAL`
becomes a ticket; empty allowlist still skips plain chat. Unknown `/command`
and outbound session lines stay skip.

## Fleet today (2026-09-14)

| Node | Role | Wire | Notes |
|---|---|---|---|
| PC 5500U | deep tier 27B IQ2 mmap | `llama_serve :8080` (`lama-2.8`) | 0.031–0.045 tok/s (BENCHMARKS.md) |
| PC 5500U | fast interactive | `llama_serve :8082` (`lama-1.5`) | ~4 s E2E head |
| PC 5500U | hub + MCP + watchdog | `gsv-server :9999`, `gsv-mcp` stdio, `gsv-watchdog` | `--no-lockstep` during drains |
| PC 5500U | coordinator | poolAI `:8091` (admin/admin123 dev) | telegram seats + virtual nodes |
| A54 | edge-worker (tasks, not tensors) | `llama_edge` → hub `/api/edge` (not `:8091`) | **no Termux** ⇒ no ggml-rpc; Mini App WebGPU = path (b) **probe only** (Start worker / Host tests / KVM superseded); intended peer = **Rust APK** `gsv-apk` `origin=apk_edge` + WiFi debug |
| telenetis | Telegram identity + Mini App shell | `:9800` | LAN-only when Telegram is down; **not** the orchestrator |
| Pi4 / spare PCs | future ggml-rpc workers | `llama_serve --rpc <ip>:50052` | proto-5.0.0 pin `f5b9bd39` |

## Hub-side must-haves

### 1. Topology (ALLBGP) — LANDED 2026-09-14 (`boxes/grid.rs`)
- `GET /api/grid` + MCP `gsv_grid` mirror poolAI `/topology/nodes`,
  `/workers`, `/discovery/virtual-nodes`, `/grid/telegram-seats`, `/status`
  into a durable ring `data/gsv_grid.json` (cap 64); 10 s `gsv-server` loop
  emits SSE `event: grid` on change; poolAI down ⇒ last-known kept +
  `poolai_alive:false`/`stale` (never an empty view). UI card = separate open
  ticket below.
- One durable GSV table: devices, capacity (VRAM/RAM/class), model→shard
  placement, rpc endpoints, seats, tiers, health edges + history ring.
- Why: poolAI graph is point-in-time and memory-only; OpenCode/Cursor need a
  stable table that survives a poolAI restart.

### 2. Capacity truth (VRAM) — LANDED 2026-09-14 (hub profiles)
- poolAI scheduler: boolean `has_gpu`, hardcoded 8192 MB free
  (`scheduler.rs:164-167`); `dispatch.rs` hot-tier vram fields are stubs —
  live proof: it echoes 7560/7560 for **both** edge-pc-01 and a54-01.
- Hub owns durable per-device profiles (`data/gsv_grid_profiles.json`,
  `POST /api/grid/profile {id, class, vram_mb, ram_mb, note, delete}`),
  merged into `/api/grid` as `capacity.rows[]` with `source: hub|poolai` +
  automatic `poolai_capacity_stub` flag (≥2 un-profiled nodes with identical
  totals). Feeding these as placement hints into poolAI job payloads stays
  open (needs a poolAI-side contract).

### 3. Placement/rebalance — LANDED 2026-09-14 (planner core)
- `plan_layers` (boxes/grid.rs) + `GET /api/grid/plan`: proportional contiguous
  shard ranges per budgeted device, rendered `llama_serve --rpc` argv, honest
  `advice` + `uncovered_layers` when 27B cannot fit. Derived from the live
  mirror ⇒ a dead device is rebalanced away on the very next plan call (poolAI
  itself still has no auto-migration for *running* jobs — planner fixes the
  map, not the in-flight task).
- **Two swarm paths, two rules.** (a) *Native* ggml-rpc sharding needs a
  `ggml-rpc-server` binary per host — Linux boxes only (Pi4 / spare PC); phones
  would need Termux, which the owner forbids ⇒ phones never join path (a).
  (b) *Browser* swarm (Nehanth/swarmllm: WebGPU + WebRTC, 10 KB activations/token)
  runs **inside the Telenetis mini-app**, so the A54's **Adreno GPU + RAM can
  host a contiguous IQ2 layer slice with zero Termux** — gated only on
  `chrome://gpu` WebGPU being enabled on the device. This is path (b) and is
  the intended hybrid: PC `:8080` mmap core + N browser/phone GPU slices chained
  pipeline-style. LlamaWeb (llama.cpp WebGPU, arXiv 2605.20706) / WebLLM /
  wllama are the in-browser engines. enapt/SwarmLLM (Rust p2p) stays rejected
  (Q4–Q8 only, no IQ2 27B, public-swarm default). Reuse poolAI
  virtual-nodes/seats/capability-doc services for orchestration; the browser
  peer registers via the existing `telegram_edge`/`virtual_node` plane, not a
  new protocol.

### 4. Hybrid routing — DONE 2026-09-14, keep
- `data/omni.toml`: cloud free-tier chain first, `bunke-rock` last;
  `X-Omni-Tier: fast|deep` → `lama-1.5`/`lama-2.8` via
  `bunke-rock-fast`/`bunke-rock` (catalog 2026-09-14).
- Offline rule: hub must always resolve to a local tier when every remote is
  cooling (already true via `record_unreachable` + fallback chains).

### 5. Burst seats — P1
- poolAI `try_admit` → hard 409 `seat_exhausted` (flat limit, in-memory set).
- Hub-side wait queue + retry + depth on keep-live = "burst" without touching
  poolAI's cap semantics; seat set is memory-only → hub re-admits on restart.

### 6. Security (hub as the single entry) — LANDED 2026-09-17 (band 237)
- poolAI edge plane (discovery/jobs/virtual-nodes/grid) has **no JWT**, the
  `RateLimitLayer` is attached nowhere, webhook secret is optional, and
  `admin/admin123` is seeded. Never expose `:8091` off-box.
- Hub front: `GET /api/edge` (redacted status) + `GET`/`POST /api/edge/{*path}`
  reverse proxy (`boxes/edge.rs`). Per-call token (`GSV_EDGE_TOKEN` env wins,
  else `settings.edge.token`; never on the wire), 20 req/s per token, path
  allowlist (`topology` / `workers` / `discovery` / `virtual-nodes` / `grid` /
  `jobs` / `health` GET-only). `login` / `vm` / `users` stay 404. JSON key
  redaction on the way out (`token` / `password` / `bot_token` / …). Health
  carries `edge_proxy`; VDC card shows the proxy line.
- Service account for Telenetis **and** the APK peer: hub `/api/edge` +
  `GSV_EDGE_TOKEN` (`edge.service.kind=edge_token`). poolAI `admin/admin123`
  is not a client credential. Clients never dial `:8091` off-box.

### 7. Connection stability (OpenCode ⇄ hub) — DONE 2026-09-14, keep
- Watchdog `--no-lockstep` / `GSV_WATCHDOG_LOCKSTEP=0`: ticket-drain rebuilds
  never bounce `:9999`; respawn-on-failure intact.
- `lease_secs=3600`: claims survive `cargo test`.
- OpenCode ⇄ `gsv-mcp` is stdio (survives hub restarts); HTTP MCP (Cursor)
  reconnects SSE by design; keep-live + watchdog heartbeat expose any gap.
- Apply is operator-driven: `cargo xtask live` / dashboard **Apply** at a calm
  moment, never implied by a background timer.

### 8. Offline mode — LANDED 2026-09-14
- Signal in health/keep-live wire: `mode: online | lan-only` (+ hint suffix).
  Fresh = relay workflow enabled AND `data/telegram_offset.json` mtime ≤ 90 s
  (written by every live `getUpdates` pass — durable across processes, so MCP
  clients read the same truth). telenetis already degrades silently; the hub
  now names it. Cut cable ⇒ `mode:"lan-only"`, omni falls through to local
  tiers, chat keeps answering.
- Phones reach telenetis via LAN `lan_url()` rewrite; tunnel is an upgrade,
  not a dependency.

### 9. Dashboard — P1
- Galaxy VDC card: topology graph + seats + tiers + health history; the same
  data as MCP tool + `gsv://` resource for agents.

## Non-goals

- Node OmniRoute stays down by policy (Rust gateway role = GSV hub).
- Rewards/credits mirroring while poolAI credits are dormant.
- GPU-VRAM accounting inside poolAI itself (hub-side until upstream lands).
- Telenetis as control plane / “do everything from the Mini App.”
- APK as a second Telenetis (no ticket board, no bot, no KVM inside the APK).
- Termux workers on phones.
