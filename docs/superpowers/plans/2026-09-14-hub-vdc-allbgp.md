# Plan — Hub-side VDC / ALLBGP: poolAI grid + telenetis edge + GSV openbot (band TBC)

**Goal:** make GSV `:9999` the single operator brain over the physical fleet — a full
topology table ("ALLBGP" — the hub sees every device, every model, every placement,
every seat) instead of point-to-point pokes. Qwen3.8-27B is spread across devices
(today PC 5500U deep `:8080` + PC fast `:8082`; A54 edge-worker online, Pi4 later;
owner policy: **No Termux**, ggml-rpc workers = Linux boxes only). Hybrid routing
stays: cloud free-tier chain first, `bunke-rock` last (config `data/omni.toml`
landed 2026-09-14), and **OpenCode ⇄ hub never drops on ticket drains** (watchdog
`--no-lockstep` + `lease_secs=3600` landed 2026-09-14, commit 8c04635).

## Research snapshot (2026-09-14, live poolAI pid 16904 @ :8091)

| Area | What exists | Gap the hub must close |
|---|---|---|
| topology | poolAI `/api/v1/topology/graph|/latency|/nodes` point-in-time; `/workers`; `/virtual-nodes` | no history, no events; GSV has no aggregated table card |
| capacity | scheduler filters `ram_mb` + **boolean `has_gpu`**; free-mem hardcoded 8192 MB (`scheduler.rs:164-167`); `hot_tier.ram/vram_bytes_used` are telemetry stubs (`dispatch.rs:95-171`) | no VRAM accounting → wrong placement for 27B shards |
| placement | shard contract `{model,layers:"start-end",rpc_endpoint}` (`llama_edge.rs:552-603`); `llama_serve --rpc host:50052` (DISTRIBUTED.md); no auto-rebalance (`re_migrate_prefetch_stub` placeholder) | nobody computes the layer map or re-issues it when a device dies |
| seats | `POOLAI_TELEGRAM_SEAT_LIMIT` flat, in-memory set, `try_admit` → **409 seat_exhausted** (`telegram_seat_service.rs:75-90`) | no burst queue; callers fail instead of waiting |
| security | edge plane (discovery/jobs/virtual-nodes/grid) **no JWT**; `RateLimitLayer` defined, attached nowhere (`rate_limit.rs:142`); dev `admin/admin123` seeded (`user_manager.rs:82-122`); webhook secret only checked if set | hub must front one authenticated, rate-limited entry; service account rotation (open ticket t-1789360414663375700) |
| state | VMs/peers/seats memory-only; jobs/edge_data disk if env set | hub mirror = durable view of memory-only truth |
| tiers | worker-side only: `payload.model=="fast"` → `LLAMA_SERVE_FAST_URL` (`llama_edge.rs:497-511`) | hub omni catalog knows `lama-2.8` (:8080) but **not** `lama-1.5` (:8082) |
| offline | telenetis degrades to LAN (Telegram = warn-only); omniroute node = down by policy | hub status/keep-live must surface "LAN-only mode" explicitly |

## Checklist (open = on the board)

- [x] grid: ALLBGP topology box in GSV hub (mirror poolAI topology/workers/virtual-nodes + history ring, GET /api/grid, MCP gsv_grid, SSE events) — landed 2026-09-14: boxes/grid.rs, fail-open stale-keep, durable ring data/gsv_grid.json, 10 s loop + event:grid
- [x] grid: per-device capacity profiles (VRAM/RAM/class) in hub, feed poolAI placement hints instead of boolean has_gpu — landed 2026-09-14: data/gsv_grid_profiles.json + POST /api/grid/profile + capacity.rows/stub detector; poolAI hint feed needs upstream contract (open)
- [ ] omni: tier routing — add bunke-rock-fast provider (:8082, lama-1.5) to catalog + X-Omni-Tier fast|deep header in proxy
- [x] grid: Qwen-27B layer-map planner — compute shard ranges, render llama_serve --rpc list, re-issue on worker loss (rebalance) — landed 2026-09-14: plan_layers + GET /api/grid/plan (proportional contiguous ranges, --rpc args, advice; derived=rebalanced on every call)
- [ ] grid: burst seat queue at hub — on poolAI 409 seat_exhausted enqueue + retry, expose queue depth on keep-live
- [x] security: hub single-entry proxy for poolAI edge plane (token auth + rate limit where poolAI has none) — landed 2026-09-17 band 237: `boxes/edge.rs`, `GET /api/edge` + `/api/edge/{*path}`, `GSV_EDGE_TOKEN` / `settings.edge.token`, 20/s, allowlist, JSON redact, health `edge_proxy`
- [ ] security: service account for Telenetis **and APK peer** replacing admin/admin123 (ticket t-1789392387275733400 / t-1789360414663375700); clients use hub `/api/edge` only
- [ ] ui: VDC dashboard card — topology graph + seats + tiers + health history in Galaxy
- [x] ops: offline mode signal — telegram-relay auto-degrade to LAN-only surfaced in keep-live + health hint — landed 2026-09-14: wire mode online|lan-only (relay ∧ offset-mtime ≤90s) + hint suffix, durable cross-process signal
- [x] docs: GSV_VDC.md canon — must-have list + EDGE_PLAN matrix; **AGI path 2026-09-17** (GSV brain, Telenetis shell, APK peer via `/api/edge`)
- [x] telenetis: freeze feature surface — identity + Mini App chrome only
- [x] gsv: Godfather allowlisted free-text ingest (allowlisted leftover chat → ticket; empty allowlist still skips)
- [ ] phone: APK + WiFi debug registers as `virtual_node` via hub `/api/edge` (not a second Telenetis)

## Non-goals

- No Node runtime, no Termux workers (owner policy). No rewards/credits mirroring
  while poolAI credits stay dormant. No pushing mid-drain.
