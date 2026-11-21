# GASV-F — Proper next steps after shexp transplant (research + plan adaptation)

**Date:** 2026-11-21  
**Status:** research/adaptation; tickets F1–F5 proposed  
**Plan:** [`docs/superpowers/plans/2026-11-21-gsv-versus-ai-wgpu-cluster-skeleton.md`](../../superpowers/plans/2026-11-21-gsv-versus-ai-wgpu-cluster-skeleton.md)  
**Depends on:** GASV A–E done, D1–D3 tooling, mirror shexp inventory, med cluster live (#37 RDNA2, #39 bifrost, #40 valhall)

## 1. Current fact (verified)

| Item | Fact | Evidence |
|---|---|---|
| Mirror tensors | 121 tensors in `data/wgpu_weights_tensors.json` + chunks: blk.0–39 `ffn_{gate,up,down}_shexp.weight` + `blk.0.ffn_down.weight`, F32, 4MiB each (blk.0 down 14MiB) | `data/wgpu_weights_tensors.json`, mirror snapshot |
| Transplanted artifact | `gsv-versus-ai.gguf` built by `rusty skeleton` + `rusty transplant-mirror`, **518,004,896 B**, 121 tensors, 121/121 transplanted, `kept_skeleton 0`, arch `gsv`, seed 20261121 | temp build 2026-11-21, sha256 `591b6310833e873159f48179ff873b6c8bad7c2da642181452b93214fa8a7669` |
| Eval smoke | `rusty inspect` ✅, `rusty manifest --sha256` ✅ (`gasv.gguf-manifest.v1`, 121 tensors), `cargo xtask gasv-publish-check` dry-run ✅ | local commands 2026-11-21 |
| Not runnable model | `rusty validate` N/A (missing `gsv.attention.head_count`), short infer N/A — artifact is **partial shexp tensor pack**, not full LLM | expected by arch contract |
| Cluster med | #37 pc-wgpu/Firefox localhost alive/ready, #39 bifrost alive/ready, #40 valhall alive/ready | `/api/wgpu/mesh` 2026-11-21 |
| D3 quant | `quantize_q4_0/q8_0`, `quantize_tensor_write`, parity-gated; Q4_K_M deferred | `rusty-engine/src/gguf.rs`, `src/quant.rs` |

## 2. Gaps / risks

- **Artifact contract ambiguous:** current GGUF looks like model file but is partial shexp-only pack. Must be explicitly labeled in manifest/provenance to avoid “runnable model” claim.
- **No tensor-level parity report:** transplant copied raw chunk bytes; we have chunk sha256 in tensors.json but no end-to-end report tying GGUF tensor bytes to mirror chunk hashes.
- **Quant not evaluated:** Q4_0/Q8_0 export possible but needs size/quant-error/parity report per tensor and decision on whether to publish F32, Q8_0, Q4_0, or multi-artifact.
- **Eval gate too weak for “proper”:** inspect/manifest is necessary but not sufficient. Need tensor byte parity, sample dequant sanity, and mesh canary against transplanted tensors (weights_matvec on known blk shexp tensors) with max_err_model threshold.
- **NanoJev boundary:** NanoJev remains eval/oracle/fitness only; no weight transplant/regrep magic. Any “regrep/витягнути ваги” must be explicit direct-copy/convert-safe with manifest and gates.
- **Publish target:** mirror currently contains minimal 16KB skeleton; replacing with 518MB F32 or quantized artifact needs owner-gated publish and clear naming (`gsv-versus-ai-shexp-f32.gguf` or keep canonical name with manifest kind).

## 3. Proper plan (F phase)

### F1 — Artifact contract + provenance
- Define `gasv.artifact-manifest.v1` (or extend existing manifest) fields: `kind=partial-shexp-pack`, `runnable=false`, `arch=gsv`, `seed`, `source=tensors.json+chunks`, `tensor_count`, `coverage` (blk range, shexp only), `sha256`, `bytes`, `transplant_report_sha256`.
- Emit transplant report JSON alongside artifact (not in git if large; in mirror or temp evidence; summary in git).
- Add explicit `no-quality-parity-claim` and `nanojev-role=eval-oracle-only`.

### F2 — Quant/export evaluation (D3)
- Produce Q8_0 and Q4_0 exports in temp from transplanted F32 GGUF.
- For each: size, tensor count, per-tensor max_abs_err/mean_abs_err vs F32 dequant where cheap, inspect/manifest OK.
- Record decision matrix: F32 (debug/canonical source), Q8_0 (safer quant), Q4_0 (smaller). Q4_K_M stays deferred/parity-gated.

### F3 — Eval gate (proper)
- **Structural:** inspect, manifest, sha256, tensor count/dims/dtype match transplant report.
- **Parity:** sample N tensors (or all 121, cheap for 4MiB) — dequant/read bytes match mirror chunk bytes hash.
- **Cluster smoke (med):** weights_matvec canary on at least 3 transplanted tensors across #37 RDNA2, #39 bifrost, #40 valhall (or available peers) with `max_err_model < 1e-4`; record route/latency.
- **No infer claim:** explicitly N/A until full model arch/weights exist.

### F4 — Publish decision
- Owner-gated: which artifact(s) to publish to `L:\gsv_mirror\models\gsv-versus-ai\`.
- Recommended default: keep canonical `gsv-versus-ai.gguf` as selected quantized pack + `gsv-versus-ai.manifest.json` + `gsv-versus-ai.transplant-report.json` (small) + `gsv-versus-ai.f32.gguf` optional if space allows.
- No git weight blobs.

### F5 — Med cluster regression / readiness
- Run med canary set on published artifact tensors (or mirror-registered tensors) after publish.
- Record in plan/tickets: #37 RDNA2, #39 bifrost, #40 valhall alive/ready/measured/max_err.
- Document Firefox localhost secure-context requirement for PC WGPU.

## 4. Ticket mapping

| Ticket | Scope | Gate | Owner |
|---|---|---:|---|
| GASV-F1 | Artifact contract + provenance/report | docs+code | gsv/rusty-engine |
| GASV-F2 | Q8_0/Q4_0 export + parity/size report | eval | rusty-engine |
| GASV-F3 | Proper eval gate (structural+parity+med smoke) | eval | gsv |
| GASV-F4 | Publish selected artifact(s) to mirror | owner-gated | gsv |
| GASV-F5 | Med cluster regression record + docs | docs | gsv |

## 5. Non-goals

- No claim that shexp-only pack is a runnable LLM.
- No NanoJev weight transplant.
- No silent tensor coercion.
- No git weight blobs.
- No Q4_K_M without parity gate.