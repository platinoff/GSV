# GASV-G16 — G10/G12 reconcile + Min MoE placement manifest (2026-11-22)

**Ticket:** `t-1795349625946716200`  
**Scope:** docs/manifest only. No weight writes, no hub changes, no canary dispatch.

## 1. Ticket reconcile (fact-checked vs repo state)

| Ticket | Board status | Actual state | Action |
|---|---|---|---|
| G10 `t-1795273000000000012` | `todo` | Full 4-bit artifact built (733 tensors, `2f37d609…`), `artifact-report` written on mirror; versioned companion published in `publish.json` (G15). Canonical F32 unchanged. | Evidence complete → mark done |
| G12 `t-1795273000000000014` | `todo` | Min placement now provable via G15 (F32 canaries) + G21 (Q4_K `g-721` `6.94e-18` on [39,37]). This document is the manifest. | Evidence complete → mark done |

No code change; only board reconcile via GSV MCP.

## 2. Min placement (active cluster: hub RDNA2 #37 + bifrost #39)

| Peer | Role | Adapter | Proven capability |
|---|---|---|---|
| 37 | hot/control | pc-wgpu (RDNA2 via localhost) | `weights_matvec` canary mt-721-1 `6.94e-18`, 40 MFLOPS; probe mt-719 matmul64 `max_err=0` |
| 39 | hot/high-throughput | arm/bifrost phone | mt-721-0 `6.94e-18`, 107 MFLOPS; repeated <1e-4 history |
| 40 (valhall) | spec only | arm/valhall | withdrawn by owner 2026-11-22; not in Min gates |
| 36 (Pi4) / 38 / 41 | excluded | — | dead/unavailable; spec Max only |

### Placement rules (derived from evidence)
- **proof/readiness gate first:** enqueue heavyweights only to alive+proven peers. After hub restart proofs reset → probe (matmul64) bootstrap required before `weights_matvec` (learned in G21).
- **fail-open, no lockout:** no ready peer → `readiness_hint`, never hard lockout. Stuck claims requeued via FW-7 (~300 s).
- **Min gate criteria:** `max_err_model < 1e-4` on both shards, group `ok=true`. G15 (F32) and G21 (Q4_K) both meet this.
- **Q4_K dispatch:** requires dequant on both hub (reference) and page (peer); verified `gasv.artifact-chunks` → artifact-sources → `register_tensor` → range-served chunk (sha256 key) lane.
- **Router/gate weights:** freeze, never patch (G17 verdict).

## 3. Tensor placement manifest (schema `gasv.moe-placement.v1`)

| Tensor class | Example | Preferred peers | Gate | Notes |
|---|---|---|---|---|
| F32 store-backed shexp/attn (612 index) | `blk.3.attn_output.weight` | [37,39] | G15 canaries `6.94e-18` | baseline, proven |
| Q4_K artifact-backed moderate | `artifact/blk.0.attn_gate.weight` | [39,37] | G21 g-721 `6.94e-18` | full lane proven |
| Q4_K huge MoE experts (122 tensors) | `blk.5.ffn_gate_exps.weight` (36 chunks registered) | spec [39,37] | NOT canary-verified at scale | generator ready; streaming shard dequant needed for phone memory |
| Valhall #40 | — | excluded 2026-11-22 | — | owner withdrew phone |

## 4. Honest gaps
- Only one moderate Q4_K tensor was canary-verified. 122 huge experts (~151 MB each, dequant ~1 GB) need a **streaming shard-range dequant** lane on the peer (current page dequantizes the whole tensor) before they can run on a phone — a separate owner-gated engineering ticket.
- No full-LLM/quality claim; G13 blocked.

No Perl/Python, no git weight blobs, no canonical change.
