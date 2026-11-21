# GASV-A5 — Cluster set spec min/med/max + WGPU smoke criteria (bifrost minimal)

**Date:** 2026-11-21  
**Ticket:** `t-1795270403265695800` (gsv, in_progress)  
**Scope:** docs/spec only. No code/network changes.  
**Plan:** [`docs/superpowers/plans/2026-11-21-gsv-versus-ai-wgpu-cluster-skeleton.md`](../../superpowers/plans/2026-11-21-gsv-versus-ai-wgpu-cluster-skeleton.md)

## 1. Cluster sets (owner-confirmed)

| Set | Composition | Status | GASV role |
|---|---|---|---|
| **Min (execution target)** | **Hub + bifrost #38/#39-class Android peer** | live when bifrost online | first smoke/eval for mirror/chunk/probe/weights_matvec readiness |
| **Med** | Min + **valhall #35** (Galaxy A54) | opportunistic; re-peer by `device_key` | dual-phone routing/failover/EWMA sanity |
| **Max (spec only)** | Med + **Pi 4** (NAND/SD restored) | **deferred**; do not test now | future CPU-lane/native agent spec only |

**Hard constraints:** 2.4 GHz Wi-Fi reconnect thrash; Android Doze/PWA keepalive; hub RAM ~7.7 GB pressure; no disk writes on peers; no Pi resurrection in this phase.

## 2. Bifrost minimal smoke criteria (Min set)

All checks are read-only/normal ops unless owner explicitly asks for dispatch.

### 2.1 Hub health
- `GET /api/health` returns healthy.
- `cargo xtask status` (or `/api/sys/metrics`) shows hub up, no crash-loop.
- Disk/RAM not in red zone for smoke (S0 disk guard).

### 2.2 Peer presence/readiness
- `/api/wgpu/mesh` or `/api/sys/metrics` shows bifrost peer **alive**.
- `ready_alive` counted only for peers with fresh proofs; `ready_stale` not treated as dispatch-ready.
- Rejoin gives stable identity via `device_key` where possible; new UA may allocate new peer id (#38→#39 pattern) — acceptable if proofs re-earned.

### 2.3 Proof gate / fail-open semantics
- **Probe ops** (`matmul64, matvec, matvec_shard, mlpblock, layernorm, softmax, minilm`) allowed for bootstrap (no total lockout).
- **Heavyweight** (`weights_matvec`) requires ready/proven peer; if not ready, response gives `no_ready/readiness_hint` (not silent hang).
- **`weights_matvec_block`** browser-only; `400 no ready peer alive` when none eligible — expected fail-open/fail-loud behavior, not deadlock.
- Dead peer stale proofs demoted when another fresh peer exists; never total lockout when fleet has no fresh proofs but has legacy proofs.

### 2.4 Golden canary (when owner requests dispatch)
- Op: `weights_matvec` on known-good tensor (e.g. blk.5 shexp gate/up/down as historical canary).
- Pass: `status=done`, `max_err_model < 1e-4` (current proof horizon), route peer is expected bifrost peer.
- Record: task id, peer id, max_err_model, elapsed, mflops, route.

### 2.5 Reconnect/failover
- Dead-days: probe ops still enqueueable; heavyweight blocked/hinted; UI shows no ready worker.
- Rejoin: peer alive → probe proofs → ready flip → heavyweight routable.
- Console/telemetry evidence captured via `/api/wgpu/console`, `/api/wgpu/telemetry`, `/api/sys/history` (no phone disk writes).

## 3. Smoke command checklist (Min set)

```bash
# hub
curl -s http://127.0.0.1:9999/api/health
curl -s http://127.0.0.1:9999/api/wgpu/mesh
curl -s http://127.0.0.1:9999/api/sys/metrics
cd /s/rust/GSV && cargo xtask status --json

# evidence (read-only)
curl -s "http://127.0.0.1:9999/api/wgpu/console?limit=20"
curl -s "http://127.0.0.1:9999/api/wgpu/telemetry?limit=5"
```

Dispatch canary only with owner go and a ready peer:

```bash
# example shape only - use real registered tensor/chunks
curl -s -X POST http://127.0.0.1:9999/api/wgpu/task/enqueue \
  -H 'content-type: application/json' \
  -d '{"op":"weights_matvec","tensor_name":"...","rows":2048,"cols":512,"shards":4}'
```

## 4. Capacity/budget notes

| Resource | Budget/Rule |
|---|---|
| Mirror chunk | <=4 MiB; sha256 content-addressed |
| Store cap | default 4 GiB LRU (`GSV_WGPU_WEIGHT_STORE_MAX_BYTES`) |
| Peer lease | 60s; poll 2s; stuck claim requeue 300s |
| Proof freshness | 3600s (`PROOF_FRESH_SECS`) |
| Heavyweight gate | ready/proven peer; fail-open/no total lockout semantics as per wgpu.rs |
| Hub RAM | treat as constrained; avoid heavy parallel cargo/build during smoke |

## 5. Exit criteria for A5

- Min/Med/Max sets documented with explicit status.
- Bifrost minimal smoke checklist actionable and read-only by default.
- Fail-open/no-lockout/proof-gate behavior captured.
- Dispatch canary marked owner-gated.

**Verdict:** PASS (spec/docs). Bifrost minimal is the only live execution target; valhall opportunistic; Pi4 spec-only.