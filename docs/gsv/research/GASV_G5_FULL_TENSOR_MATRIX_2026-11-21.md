# GASV-G5 Full Tensor Regrep Matrix (35B-A3B, 2026-11-21)

**Status:** ✅ done (2026-11-21)  
**Model:** `S:/rust/rusty-engine/data/models/MiniCPM-V-4_7-35B-A3B-Q4_K_M.gguf` (qwen35moe, 733 tensors)

## Classification (by F32 materialized size)

| Class | Count | F32 GB | Action |
|---|---:|---:|---|
| already-in-mirror | 501 | 0.64 | present in `data/wgpu_weights_tensors.json` (502 entries total incl prior) |
| large-chunked-feasible (16-128MB) | 110 | 4.69 | extracted ok=110 fail=0 to `L:/gsv_mirror/staging_g5/35b_large_f32/`; registered into live wgpu_weights (added=110, new_chunks=1200, total=612). Canary `g-708` `blk.3.attn_output.weight`: mt-708-0 p39 `6.939e-18`, mt-708-1 p37 `6.939e-18`. |
| huge-moe-expert-needs-plan (>128MB) | 122 | 123.79 | **infeasible on this box** (disk/RAM); needs dedicated plan/node/mirror chunking strategy |

Machine-readable matrix: `docs/gsv/research/GASV_G5_FULL_TENSOR_MATRIX.json` (git-trackable metadata; no weights).

## Boundaries

- No silent coercion. `dump-f32` is explicit dequant/convert; class recorded.
- NanoJev eval/oracle only.
- Large 3D MoE experts (123.79GB F32) block "full working LLM" on this box; they require dedicated storage/compute plan and runtime support.
- `already-in-mirror` covers 502 live tensors after G2b merge; matrix count 501 reflects manifest-name presence.

## Next (updated by G6)

- G5 extraction/register/canary done (g-708 PASS). Remaining blocker: 122 huge MoE experts (123.79GB F32) - owner-gated dedicated storage/compute/chunking plan.
- Q4 orientation: G2b Q4_0 export is the 4-bit ecosystem candidate (`ec1cdf86...`, 7.1111x); canonical remains F32 until owner-gated promotion.
- NanoJev: eval/oracle/fitness only; no weight transplant/generation. Use as fitness gate alongside WGPU canaries.
- Runtime infer/quality gate remains N/A until full compatible graph is assembled and validated.