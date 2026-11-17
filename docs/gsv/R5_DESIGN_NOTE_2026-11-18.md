# R5 Design Note — "Replicated-shexp local-inference mode" (2026-11-18)

**Question (research doc §3, R5):** якщо peer може тримати 1 blk shared-expert tensors — виконувати forward **локально** без mesh round-trip на кожен shard?

**Short answer: not now — build nothing; the cheap 80% is already shipped (R3).**

## What R3/R1/R2 already solved

| Problem | Status |
|---|---|
| Phone wake latency dominated by re-download | ✅ R3 — persistent IndexedDB chunk cache (`gsv-mesh-weights`, LRU 128 MiB) |
| Ghosts grabbing MiniCPM dispatches | ✅ R2 — stale-proof heavyweight gate |
| Sub-optimal route order | ✅ R1 — EWMA tier routing, live A/B `g-199` |
| Per-task telemetry visibility | ✅ `ewma_elapsed_ms` in `/api/sys/metrics` + /sys card |

## Why full local-inference mode is not recommended right now

1. **Verification model conflict.** The mesh's correctness guarantee is *per-task* `max_err_model` against a hub-side golden reference — every MiniCPM dispatch returns a numeric proof. A local autonomous forward would drop dispatches (no per-op proofs) or require a new audit protocol with zero gain in trust.
2. **The bottleneck is not HTTP overhead.** Per-task round-trip over LAN/4g is 20–100 ms; a `weights_matvec` shard compute is 13–22 ms on phones (hot) — the phone is compute/network-bound on the *first* download, not on steady-state dispatch. R3 addresses exactly the first-download cost.
3. **Phones sleep.** An autonomous "keep resident + run silent forward loops" mode fights the hot-swap model (browser tab freeze/discard). The page must stay foregrounded or re-download resumes — same as today.
4. **Storage pressure.** blk.N shexp tensors ≈ 16 MiB × 3 × 40 blocks ≈ 1.9 GB if fully resident — exceeds typical phone IDB budgets (~128–200 MiB usable with quota eviction). R3's cap already forces LRU at exactly this regime.
5. **It duplicates what peers already do.** `buildWeightsMatvec` + `CACHE[key]` memory retention + R3 persistence = every repeated tensor/shard for the session is computed locally with one chunk read. There is no missing "locality" tier left below the current design.

## What a *minimal* R5 would look like (if ever)

- **Sequence batching:** one claim carries a *recorded* sequence of ops (`weights_matvec` ×N for N tensors) with one combined report — strictly subsumed by **R4** (multi-tensor envelope, owner-gated).
- **Warm-resident hint:** peer advertises "I hold tensors [a,b,c]" in `peer_join`/poll; hub prefers those peers in `route_peers` (soft, no protocol change — a `resident[]` hint field, peers ignore if absent). Benefit: fewer cold chunk-fetches for repeat tensors. Effort: S–M. **Needs owner ok** because it changes what `peer_poll` prefers, similar review surface to R1.
- **Never:** replace per-task proofs with "peer self-certifies forward" — verification is the point of this mesh.

## Decision

- **R5 = park.** Revisit only after R4 lands and one real end-to-end forward (35B) is demonstrated on peers. No code written.
- Telemetry shipped in the meantime: `ewma_elapsed_ms` (per-peer predicted makespan, and the R1 routing key).
- Owner gates unchanged: R4, R8, Vision rev mirror, ECO-8.2, mesh-node remote, Qwen3.8-27B GDN/SSM.

## References

- [`MESH_DISPATCH_RESEARCH_2026-11-18.md`](MESH_DISPATCH_RESEARCH_2026-11-18.md) — R1..R8 roadmap
- [`MESH_BENCHMARK_2026-11-18.md`](MESH_BENCHMARK_2026-11-18.md) — p50/p90/p99 distribution
- `src/boxes/wgpu.rs` — `ewma_elapsed_ms()`, `proof_gate()`, `page()` (peer agent, IDB cache)
- `src/boxes/sysmon.rs` — `proofs_fresh` / `ewma_elapsed_ms` mesh telemetry
