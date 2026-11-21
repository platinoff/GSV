# GASV-G4 WGPU Cluster Eval/Regression (2026-11-21)

**Status:** ✅ done (2026-11-21)  
**Scope:** regression canaries for G2b expanded candidate on live med cluster. No runnable full LLM claim.

## Environment

- Hub: `http://127.0.0.1:9999`, GSV data dir live, wgpu_weights merged to 502 tensors.
- Active peers during G4: #37 pc-wgpu (alive), #39 arm/bifrost (alive), #40 arm/valhall (alive/ready=0 at poll, prior F3/F5 evidence). #36/#38/#41 offline.
- Artifact under test: `gsv-versus-ai-g2b.gguf` (502 tensors, F32, sha256 `25fadfbb37242047126ed8cbb904acbc56f94685be487ef016d71543222c9995`), versioned mirror copy; canonical unchanged.

## WGPU canary results (newly added 35B tensors)

| Group | Tensor | Task | Peer | max_err_model | Status |
|---|---|---|---:|---:|---|
| g-704 | blk.0.ffn_gate_inp.weight | mt-704-0 | 39 bifrost | 3.388131789017202e-21 | done |
| g-704 | blk.0.ffn_gate_inp.weight | mt-704-1 | 37 pc-wgpu | 0.0 | done |
| g-705 | blk.10.ffn_gate_inp.weight | mt-705-0 | 39 bifrost | 1.3552527156068804e-20 | done |
| g-705 | blk.10.ffn_gate_inp.weight | mt-705-1 | 37 pc-wgpu | 0.0 | done |
| g-706 | blk.20.ffn_gate_inp.weight | mt-706-0 | 37 pc-wgpu | 0.0 | done |
| g-706 | blk.20.ffn_gate_inp.weight | mt-706-1 | 39 bifrost | 1.3552527156068804e-20 | done |
| g-707 | blk.30.ffn_gate_inp.weight | mt-707-0 | 37 pc-wgpu | 0.0 | done |
| g-707 | blk.30.ffn_gate_inp.weight | mt-707-1 | 39 bifrost | 1.3552527156068804e-20 | done |

Probe: `mt-703` matmul64 peer 37 `max_err=0`.

**All G4 canaries < 1e-4 (pass).**

## Baseline evidence (retained)

- F3 parity: `ok=121 bad=0 total=121` (aligned data_offset 8352).
- F3/F5 med canaries #37/#39/#40: mt-685/686/690-693/699/701 all `<1e-4`.
- G2b spot parity: 3/3 OK.
- G3 NanoJev eval-oracle: PASS (eval/oracle only; no weight gen).

## Gap to "full working LLM"

Current artifact is a **partial pack** (502 tensors: blk.0.ffn_down + blk.0-39 ffn_{gate,up,down}_shexp + 381 small 35B tensors). Not runnable full LLM.

To reach runnable full LLM, need:

1. **Full tensor coverage** for target arch (qwen35moe 35B-A3B): embeddings, output_norm, all attn q/k/v/o/gate/norm, all ffn norms, MoE router/gate, MoE experts (large 3D), final norm/lm_head - with explicit compatibility matrix and checksums.
2. **Large-tensor feasibility**: 3D MoE expert tensors exceed 16MB F32 pass cap; require chunked/disk/RAM plan (L: mirror has space; host RAM limited).
3. **Architecture/runtime gate**: validate via rusty-engine/llama runtime for qwen35moe (not just weights_matvec canary). 
4. **Quality eval**: perplexity/smoke infer on owner-approved target; NanoJev may be used as oracle only where applicable.
5. **Owner gate** for any canonical mirror replacement/publish.