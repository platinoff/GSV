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

## Fleet today (2026-09-14)

| Node | Role | Wire | Notes |
|---|---|---|---|
| PC 5500U | deep tier 27B IQ2 mmap | `llama_serve :8080` (`lama-2.8`) | 0.031–0.045 tok/s (BENCHMARKS.md) |
| PC 5500U | fast interactive | `llama_serve :8082` (`lama-1.5`) | ~4 s E2E head |
| PC 5500U | hub + MCP + watchdog | `gsv-server :9999`, `gsv-mcp` stdio, `gsv-watchdog` | `--no-lockstep` during drains |
| PC 5500U | coordinator | poolAI `:8091` (admin/admin123 dev) | telegram seats + virtual nodes |
| A54 | edge-worker (tasks, not tensors) | `llama_edge` → telenetis/poolAI | **no Termux**, so no ggml-rpc |
| telenetis | bot + edge UI | `:9800` | LAN-only when Telegram is down |
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

### 2. Capacity truth (VRAM) — P0
- poolAI scheduler: boolean `has_gpu`, hardcoded 8192 MB free
  (`scheduler.rs:164-167`); `dispatch.rs` hot-tier vram fields are stubs.
- Hub owns per-device memory profiles and sends placement hints
  (layer ranges per device) → poolAI/`llama_edge` payload `{model, layers,
  rpc_endpoint}`.

### 3. Placement/rebalance — P0
- Layer-map planner: given Σ free RAM across devices + IQ2 weights ≈ 6.9 GiB,
  cut contiguous layer ranges; render `llama_serve --rpc` argv; re-issue on
  worker loss (poolAI has no auto-migration — `re_migrate_prefetch_stub`).
- No Termux ⇒ phones never hold tensors; only Linux ggml-rpc boxes shard the
  27B. A54 stays a task worker (draft holder, probes, side jobs).

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

### 6. Security (hub as the single entry) — P0 for anything beyond LAN
- poolAI edge plane (discovery/jobs/virtual-nodes/grid) has **no JWT**, the
  `RateLimitLayer` is attached nowhere, webhook secret is optional, and
  `admin/admin123` is seeded. Until poolAI fixes these, LAN-only + loopback
  binds; never expose `:8091`.
- Hub adds what poolAI lacks on its edge plane: per-call token + rate limit,
  capability-doc verify pinned (`POOLAI_CAPABILITY_VERIFY_KEY`), id redaction
  on every wire (band-174 rule).
- Service account replaces dev admin (ticket `t-1789360414663375700`).

### 7. Connection stability (OpenCode ⇄ hub) — DONE 2026-09-14, keep
- Watchdog `--no-lockstep` / `GSV_WATCHDOG_LOCKSTEP=0`: ticket-drain rebuilds
  never bounce `:9999`; respawn-on-failure intact.
- `lease_secs=3600`: claims survive `cargo test`.
- OpenCode ⇄ `gsv-mcp` is stdio (survives hub restarts); HTTP MCP (Cursor)
  reconnects SSE by design; keep-live + watchdog heartbeat expose any gap.
- Apply is operator-driven: `cargo xtask live` / dashboard **Apply** at a calm
  moment, never implied by a background timer.

### 8. Offline mode — P1
- Signal in health/keep-live: `telegram=down → LAN-only` (telenetis already
  degrades; hub must surface it, not hide it).
- Phones reach telenetis via LAN `lan_url()` rewrite; tunnel is an upgrade,
  not a dependency.

### 9. Dashboard — P1
- Galaxy VDC card: topology graph + seats + tiers + health history; the same
  data as MCP tool + `gsv://` resource for agents.

## Non-goals

- Node OmniRoute stays down by policy (Rust gateway role = GSV hub).
- Rewards/credits mirroring while poolAI credits are dormant.
- GPU-VRAM accounting inside poolAI itself (hub-side until upstream lands).
