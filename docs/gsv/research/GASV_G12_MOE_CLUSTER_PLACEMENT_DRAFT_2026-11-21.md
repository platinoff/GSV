# GASV-G12 MoE Cluster Placement / Tuning Draft (2026-11-21)

**Status:** draft (prep while G9 assemble-copy runs)  
**Scope:** MoE expert placement/sharding manifest for GSV WGPU cluster. No weight moves without canary/gate.

## Alive peers (steady state snapshot 2026-11-21)

| Peer | Class | Adapter | Alive | mflops_max | Notes |
|---:|---|---|---:|---:|---|
| 37 | pc-wgpu | adapter | true | 30.84 | stable, low latency to hub |
| 39 | phone | arm/bifrost | true | 137.97 | good throughput, Wi-Fi |
| 40 | phone | arm/valhall | true | 328.0 | best measured throughput |
| 36 | pi4-cpu | arm/cortex-a72 | false | 0 | dead/spec-gated |
| 38 | phone | arm/bifrost | false | 0.24 | dead |
| 41 | pc-wgpu | amd/rdna-2 | false | 18.14 | dead |

## Placement principles (GSV)

- **Readiness/proof gate first.** Never route heavy weights_matvec to unproven peers.
- **Fail-open, no silent lockout.** If no ready peer, return readiness hint; do not crash.
- **Hot experts near fast peers.** #40 valhall highest mflops -> hot/shared experts; #39 bifrost secondary; #37 pc-wgpu fallback/low-latency control.
- **Minimize Wi-Fi transfer.** Cache chunks (R3 IndexedDB), prefer shard locality, group by (layer,proj,expert-pack).
- **NanoJev fitness = oracle only.** Placement scoring may use NanoJev/eval signals; no weight transplant.

## 35B-A3B MoE shape (source)

- 40 layers, 256 experts, 8 active, expert_ffn=512, d_model=2048.
- Expert tensors per layer: `ffn_gate_exps.weight`, `ffn_up_exps.weight`, `ffn_down_exps.weight` (3D, 256 experts). 120 expert tensors total.
- Shared expert tensors also present (`ffn_*_shexp.weight`) - small/hot, keep replicated or pinned to fastest ready peers.

## Initial placement policy (draft)

| Tier | Peers | Role |
|---|---|---|
| Hot | 40, 39 | active expert shards, shared-expert hot path, weights_matvec canaries |
| Warm | 37 | fallback, control probes, small tensors, overflow |
| Cold/spec | 36,38,41 | excluded until alive+proof-ready |

## Manifest schema (draft)

```json
{
  "schema": "gasv.moe-placement.v1",
  "artifact": "gsv-versus-ai-full-q4k.gguf",
  "peers": [{"peer_id":40,"role":"hot","mflops_max":328.0}],
  "rules": [{"match":"ffn_*_shexp.weight","prefer":[40,39,37],"replicas":2}],
  "expert_shards": [{"layer":0,"proj":"gate","expert_range":[0,31],"prefer":[40]}]
}
```

## G12 gates

- No placement change without WGPU canary pass <1e-4 on affected tensors.
- Re-run canaries after peer churn/reconnect.
- Record in `GASV_G12_*` report; update tickets.