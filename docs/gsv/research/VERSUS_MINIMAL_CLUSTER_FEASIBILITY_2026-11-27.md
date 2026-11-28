# gsv-versus-ai on a minimal cluster — feasibility (2026-11-27)

**Question:** given the actual repo state and the files on `L:/gsv_mirror`, can
`gsv-versus-ai` be run in the cluster on a minimal setup?

**Verdict:** **Numerics-only MoE FFN chain — YES on a minimal 1-peer config,
already partially proven. Real model forward — NO.** The
bottlenecks are *missing semantics* (attention/GDN/norm, real GGUF expert
binding) and *orchestration/RAM*, not the 4-bit kernels. No quality claim is
possible while G13 is blocked. BENCH-2 (3-device sweep) was deliberately not
run: only one phone is online.

## 1. What is actually on the mirror (`L:/gsv_mirror/models/gsv-versus-ai/`)

| Artifact | Size | What it is |
|---|---|---|
| `gsv-versus-ai-full-q4k.gguf` | **21,166,255,488 B** (733 tensors, arch `qwen35moe`, F32/Q4_K/Q6_K, vocab 248 144, hidden 2048) | direct-copy full Q4_K source (`MiniCPM-V-4_7-35B-A3B-Q4_K_M`); contract `runnable=false`, `nanojev_role=eval-oracle-only`, `no_quality_parity_claim=true` |
| `expert-pack/experts-LNN.pack` × 40 + `manifest-LNN.json` × 40 | **522,190,848 B/layer = 20 GB total** | expert-major `[gate Q4_K | up Q4_K | down Q6_K]`, 4096-aligned; 256 experts/layer × 2,039,808 B; sha256 per entry |
| `gsv-versus-ai.gguf` | 137,601,184 B (121 tensors, arch `gsv`, Q8_0, ctx 256) | partial shared-expert transplant skeleton — **not a runnable LLM** (F1/F4 contract) |
| `gsv-versus-ai-q8_0.gguf` / `-q4_0.gguf` | 132 M / 70 M | published canonical Q8_0 + smaller Q4_0 from F4 |
| `gsv-versus-ai-vision-q4k.gguf`, `inkling2-vision-q4k.gguf`, `minicpm1b-vision-q4k.gguf` | 298 M / 171 M / 295 M | Q4_K vision companions; full 4-token prefix exists but needs trained alignment |
| reports / manifests / publish set | small JSON | provenance, sha256, coverage, artifact contract |

`L:` = 63 GB, 56 used, **7.3 GB free**. No room for a second pack — range-read
streaming from the existing pack is the only viable path. `S:` = 43 GB free,
but host RAM is **7.4 GB total / ~0.58 GB free (88 % used)**.

## 2. What the code can actually do

- **GSV numerics lane:** `cargo xtask moe-forward --pack`, `moe-forward-all
  --layers N --pack`, `moe-bench`, `moe-hotset`, `moe-pack`. `moe_forward.rs`
  explicitly declares `handoff_hidden()` **zero-padded with no
  attention/GDN/residual/RMSNorm** — "NUMERICS ONLY, not a model forward".
- **Proven numerically:** pack vs GGUF A/B 101 s vs 103 s, both
  `max_err 5.35e-11`, bit-identical output; 2-layer chained proof PASS;
  resume reached later layers at `1.8e-10`; Belady hot-set + phase prefetch
  (`MOD-1`, `MOD-4`) reduce transfers ~28 % / adapt prefill vs decode.
- **Missing for a real forward:** `PH-406` (real GGUF binding of the
  `ExpertCache` path), `PH-1206` (qwen35moe runner in rusty-engine —
  `runner35.rs` handles the 1B dense GDN/attention geometry but **not** the
  35B-A3B expert path), trained vision→text alignment (G13). Owner closed
  the dedicated-node question ("інших вузлів не буде").

## 3. Live cluster state (at check time)

- Hub `127.0.0.1:9999` alive, v0.248.0, sync rev 516, watchdog alive.
- Mesh `alive 2/3`, `ready 1`: peer **#43** = `pc-wgpu`, measured 33.83 MFLOPS,
  `concurrent_limit=1`, `claimed_now=1`, `within_quota=false`; phone **#39** =
  `arm/bifrost`, alive but `iters_max=0`, `mflops=0` (no fresh proof);
  valhall **#44** dead.
- Several groups show `ok:false` / no peer, and one task result failed to post
  after 3 attempts. With a single JSONL claimed slot and `concurrent=1`,
  an uninterrupted 40-layer chain is **not currently schedulable** — this is
  orchestration/environment, not a kernel deficit.
- RAM headroom (~0.58 GB) is too small to safely start a second full-engine
  process; doing so risks killing the hub (historical incident).

## 4. Minimal viable configuration (what actually works)

1. **Hub** on `S:`, `target/live` untouched, `:9999` listening.
2. **One WebGPU peer** with a fresh proof (today: #43 PC; #39 only after a
   proving op). `concurrent_limit=1` is enough for 1 expert ≈ 1 chunk —
   coalescing was measured **9× slower** (MOD-5), so do not wire it.
3. **Pack on `L:`**, streamed per byte range with `x-gsv-key`; no full 20 GB
   load, no second pack, no new mirror writes.
4. Run `cargo xtask moe-forward --pack --layer N --role gate|up|down` for a
   single-expert numerics canary, or `moe-forward-all --layers 2..4 --pack`
   for a resumable FFN-chain proof. Keep `--shards` small (1–2) and
   `--wait-secs` bounded so a flapping peer cannot wedge the claim slot.
5. Gate: `max_err_model < 1e-4`, output bit-identical to the GGUF path.
   **Stop there** — this is numerics, not a model answer.

That configuration is the *minimum* that is both possible and honest. It was
effectively already demonstrated (A/B pack proof, 2-layer chain).

## 5. Hard no-go (do not attempt without owner go / new hardware)

- A runnable full `gsv-versus-ai` forward: needs attention + GDN + norm +
  residual layers, real GGUF expert binding (`PH-406`), and a ≥24 GB node —
  owner declined extra nodes; `G13` blocks all quality claims.
- 40-layer uninterrupted chain on today's fleet: blocked by 1 proof-bearing
  peer, `concurrent=1` and a single claimed JSONL slot (SCALE-1 territory,
  parked until a real ≥10-active-peer fleet).
- BENCH-2 3-device sweep: phone count insufficient (explicit owner exclusion).
- Any second pack / re-quant on `L:`: only 7.3 GB free; the 21 GB source is
  immutable.

## 6. Live attempt on the exact GGUF (2026-11-27, VERS-GGUF-MIN)

To remove any doubt about "just run that file", I invoked the GGUF path
directly — **no `--pack`**:

```bash
./target/vers_alt/debug/gsv-xtask.exe moe-forward \
  --artifact "L:/gsv_mirror/models/gsv-versus-ai/gsv-versus-ai-full-q4k.gguf" \
  --layer 0 --shards 1 --wait-secs 180 --seed 1.0
```

(An isolated `target/vers_alt` build was needed: `target/debug/gsv-xtask.exe`
is locked by the resident `cycle-watch` daemon and was too old to contain
`moe-forward`. `target/live` was untouched.)

**Result: two bounded attempts failed — `g-7724` (180 s) and `g-7731`
(120 s) both ended with `no partial after 3 attempts`.** Causes visible in
the live hub, none of them numerical:

- The hub **restarted mid-run**: mesh `done`/`failed` counters reset to 0 and
  all peer proofs (`iters_max`, `mflops_max`) dropped to zero; best_peer became
  `null`.
- Console shows a repeating loop of `task result post FAILED after 3 attempts
  — shard will be re-verified by the next peer` for peer #43, even when the
  peer logged `task mt-7691: done … max_err=0.00e+0`. The peer **computes**
  the shard; the result never reaches the hub, so the group never completes.
- Newly enqueued groups got `peers: []` / `done_shards: 0` — no proof-bearing
  peer available for the heavyweight `weights_matvec` route.

This is exactly the "2 flapping peers + `concurrent=1` + single JSONL claimed
slot" orchestration blocker recorded above, now observed directly. It does
**not** contradict the earlier proven A/B numerics (101 s vs 103 s,
`max_err 5.35e-11`, bit-identical) — that run had a stable hub and fresh
proofs. No wrong numbers were produced and no quality claim is made.

**Practical conclusion:** the command is correct and the artifact is readable;
a successful minimal run requires a hub that stays up long enough and at
least one peer with a fresh proof. Re-attempt after the hub/peer state is
stable; do not work around it by faking results.

## 7. Bottom line

**Yes for a minimal numerics-only cluster run; no for "running versus" as a
model.** The useful next step is *not* to chase a bigger fleet but to keep
the single-peer pack canary and the resumable 2–4-layer proof green, and wait
for owner go on G13 / PH-406 if a real forward is ever wanted. No files were
changed on the mirror, no weights were loaded, no token was generated.

Evidence: GSV `dea01f4` (numerics lane), rusty-engine HEAD (qwen35moe gaps),
`docs/gsv/GSV_TECH_ROADMAP.md` addenda, `L:/gsv_mirror/...` file listings and
artifact reports, live `/api/wgpu/mesh` + `/api/wgpu/console`, `/proc/meminfo`,
and the two failed live attempts in §6 (`g-7724`, `g-7731`). Tickets VER-MIN
and VERS-GGUF-MIN done.
