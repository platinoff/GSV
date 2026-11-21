# GASV-G6: Q4 orientation, NanoJev eval-oracle, WGPU cluster adaptation (2026-11-21)

**Status:** ✅ G6 done (slice) + G7 done (Q4_K/Q4_K_M export smoke, 2026-11-21). No full-LLM claim.  
**Owner ask:** адаптація доків/планів під сталеві стани; ресерч рішень; орієнтир квантізація 4; настроїти все що можливо; прогнати виправлення ваг через NanoJev з кластером і WebGPU щоб версус максимально підходящий до екосистеми GSV.

## 0. Steady-state snapshot (2026-11-21)

| Item | Steady state | Evidence |
|---|---|---|
| Canonical artifact | `gsv-versus-ai.gguf` = G2b F32 706,495,136 B, 502 tensors, sha256 `25fadfbb37242047126ed8cbb904acbc56f94685be487ef016d71543222c9995` | `L:\gsv_mirror\models\gsv-versus-ai\gsv-versus-ai.publish.json` |
| 4-bit export (Q4) | `gsv-versus-ai-g2b-q4_0.gguf` 99,346,536 B (99,379,296 payload bytes in report), sha256 `ec1cdf860fcce744b9b19b0fed2833d4a6d0cd6933ed50bc3ca7ec8924a54ef2`, 502 tensors, ggml_type=2, compression 7.1111x | `gsv-versus-ai-g2b-q4_0.report.json` |
| 8-bit export | `gsv-versus-ai-g2b-q8_0.gguf` 187,654,568 B, sha256 `ebfff1a9d5272b9b279bf5cd7d05e7bb3efa8170ccb6dee96f72aeb1e197cf79` | publish.json |
| Live WGPU tensor index | 612 entries (501 prior mirror + 110 large G5 + G2b coverage), 1200 new chunks added in G5; gitignored `S:/rust/GSV/data/wgpu_weights_tensors.json` | G5 report/ticket evidence |
| Cluster | min=hub RDNA2 WGPU + bifrost; med=+valhall; max=+Pi4 spec-gated. Recent canaries pass <1e-4 on peers #37/#39/#40 | G4/G5 reports |
| NanoJev role | **eval/oracle/fitness only**. Checkpoint present `S:/rust/rusty-engine/data/models/nanojev/best.safetensors` (2.3G), not modified. No GGUF generation/transplant. | G3 report, AGENTS decisions |
| Full LLM | **Not claimed**. 122 huge MoE expert tensors (123.79GB F32) infeasible on this box; runtime infer/quality gate missing. | G5 matrix |

## 1. Quantization-4 research (GSV ecosystem fit)

### 1.1 What exists

| Format | File | Bytes | Ratio vs F32 | Max abs err (report) | Mean abs err (report) | Ecosystem fit |
|---|---|---:|---:|---:|---:|---|
| F32 canonical | `gsv-versus-ai.gguf` | 706,495,136 | 1.00x | 0 | 0 | debug/source of truth; heavy for edge |
| Q8_0 | `gsv-versus-ai-g2b-q8_0.gguf` | 187,654,568 | ~3.76x | 0.002277 (max tensor) | 0.00005008 avg | safer quality; larger than Q4 |
| Q4_0 | `gsv-versus-ai-g2b-q4_0.gguf` | 99,346,536 | 7.1111x | 0.0224609375 (blk.0.ffn_down_shexp max in sample) | 0.000946 avg sample | **target 4-bit** for size/edge; quality tradeoff accepted only with eval gates |
| Q4_K_M | not exported | - | - | - | - | **deferred/parity-gated** in rusty-engine quantize-export (unsupported target in current CLI) |

### 1.2 Q4 orientation decision

- **Primary 4-bit artifact for GSV ecosystem:** `Q4_0` versioned export (`gsv-versus-ai-g2b-q4_0.gguf`) — smallest proven 4-bit in current pipeline, fits Wi-Fi/mirror transfer and edge constraints.
- **Canonical remains G2b F32** unless owner explicitly promotes Q4. Reason: F32 is lossless source-of-truth for current partial pack; Q4 introduces quantization error.
- **Q4_K_M** remains research target: better quality/size tradeoff in llama.cpp ecosystem, but current `rusty quantize-export` supports only `q4_0|q8_0`. Implementing Q4_K_M requires quant kernel + GGUF type mapping + parity/eval gate (owner-gated code work).

### 1.3 Configure-everything-possible checklist

| Knob | Current | Possible now | Action | Risk |
|---|---|---|---|---|
| Quant target | Q4_0/Q8_0 exported | select Q4_0 as 4-bit candidate | record in publish/report; no silent promote | low |
| Export report | present | verify sha256/bytes/err | done by report inspection | low |
| WGPU canary | F32 mirror chunks | run weights_matvec on existing F32 chunks for Q4-selected tensor names | feasible; proves transport/compute, not Q4 decode | low |
| NanoJev fitness | eval-oracle role | use as independent oracle gate; no weight transplant | record boundary + run available checks | low |
| Q4_K_M | unsupported | add quantizer in rusty-engine | **owner-gated code**, medium risk | med |
| Huge MoE experts | 122 missing | chunked extract/register plan | owner-gated disk/RAM/compute | high |
| Runtime infer/quality | N/A | load/infer smoke on compatible runtime | needs full compatible graph + runtime target | high |

## 2. "Weight fixes through NanoJev" — correct GSV interpretation

**Constraint (established):** NanoJev is eval/oracle/fitness only. It is not a GGUF generator/transplanter and cannot directly patch GGUF weights.

**Allowed/factual meaning in GASV:**
1. **Oracle fitness gate:** evaluate candidate artifacts/quant choices against NanoJev-available oracle signals where applicable; reject/regress candidates that fail gates.
2. **Cluster/WebGPU fitness:** run WGPU `weights_matvec`/probe canaries on selected tensors/peers; max_abs_err_model < 1e-4 required.
3. **Quant/parity fixes:** fix selection/config (F32 vs Q8_0 vs Q4_0), reject tensors with unacceptable quant error, re-export with correct params, never silent coercion.
4. **Provenance:** every candidate keeps sha256, manifest, transplant/quant report.

**Not done / not claimed:** NanoJev did not modify GGUF weights. No automatic weight repair/transplant was performed by NanoJev.

## 3. Feasible execution (this pass)

### 3.1 Doc adaptations (steady state)

- Update master plan status to include G5 done and G6 in-progress.
- Mark G5 report Next items as partially addressed by G6.
- Keep canonical/publish facts aligned with `gsv-versus-ai.publish.json`.

### 3.2 WGPU cluster canaries (G6)

Run additional canaries on large G5-registered tensors and/or representative G2b tensors across med peers. Record task IDs, peers, max_err_model.

### 3.3 NanoJev oracle check

- Checkpoint present: `S:/rust/rusty-engine/data/models/nanojev/best.safetensors`.
- Rusty nanojev lib tests: observe current state; do not claim green if failing. Use NanoJev role as fitness boundary regardless of test harness state.

## 4. G6 canary log

| Group | Tensor | Peers/Tasks | max_err_model | Verdict |
|---|---|---|---|---|
| g-708 | blk.3.attn_output.weight | mt-708-0 p39, mt-708-1 p37 | 6.939e-18, 6.939e-18 | PASS |
| g-709 | blk.11.attn_output.weight | mt-709-0 p37, mt-709-1 p37/39 (requeued holder-dead) | 6.939e-18 (mt-709-0); mt-709-1 in-flight at report time | PASS (mt-709-0) / confirm mt-709-1 |

## 5. Gaps / owner gates

- **Q4_K_M export:** needs rusty-engine quantizer implementation + parity gate. Owner-gated.
- **Promote Q4 as canonical:** owner-gated; only after infer/quality/eval policy says so.
- **122 huge MoE experts (123.79GB F32):** owner-gated dedicated storage/compute/chunking plan. Blocks full runnable LLM.
- **Full infer/quality gate:** requires compatible runtime target + full tensor graph coverage. Not claimed.

## 6. Definition of done (G6 slice)

- Docs adapted to steady states (plan + G5/G6 reports).
- Q4 orientation documented with concrete artifacts/errors.
- At least one additional WGPU canary recorded.
- NanoJev boundary stated factually; no false "weights fixed by NanoJev" claim.
- Commit/push docs/reports only.

## 7. G7 Q4_K / Q4_K_M export (2026-11-21, owner option 1)

**Status:** ✅ done (code+smoke export)  
**rusty-engine commits:** `692b8ce` (Q4_K quantizer + CLI q4_k/q4_k_m), `bf60c90` (small-tensor fallback).

### Implementation

- Added `quantize_q4_k` (GGML type 12, 256 values / 144 bytes) in `rusty-engine/src/quant.rs`: affine 6-bit scales/mins, superblock f16 d/dmin, min_off>=0 robust fallback for mixed-sign subblocks. Round-trip tests added.
- `rusty quantize-export` now accepts `q4_k` and `q4_k_m`.
- **Q4_K_M policy:** `q4_k_m-no-imatrix-fallback=q4_k` (no imatrix; no Q6_K encoder). All 256-aligned tensors -> Q4_K; tensors with numel%256!=0 fall back to Q8_0 (or F32 if not %32). Fallback recorded per-tensor in report (`requested_ggml_type`, `dst_ggml_type`, `fallback`).
- Quant dispatch supports type 12.

### Smoke export (G2b 502-tensor F32 canonical)

- Input: `L:/gsv_mirror/models/gsv-versus-ai/gsv-versus-ai-g2b.gguf` (F32, 706,495,136B, 502 tensors)
- Command: `rusty quantize-export ... q4_k_m --report ...`
- Output report: `L:/gsv_mirror/models/gsv-versus-ai/gsv-versus-ai-g2b-q4_k_m.report.json`
- Result: `n_tensors=502`, `bytes_written=99383136`, `total_src_bytes=706464256`, `total_out_bytes=99349416`, `compression_ratio=7.1109`, `sha256=883f2c0349f27fb396f6401ab3b94aeb812dc1f3ee70dcb30bf14659223c1504`, `quant_policy=q4_k_m-no-imatrix-fallback=q4_k`.
- Sample err (blk.0.ffn_down.weight): max_abs_err=0.015535, mean_abs_err=0.0005677 (vs Q4_0 0.017212 / 0.0007044). 
- Tmp GGUF removed after smoke; report kept on mirror. **Not published as canonical/distribution artifact** (owner-gated).

### Caveats

- Not true llama.cpp Q4_K_M importance-mixed quant (no imatrix, no Q6_K encoder). Report states fallback explicitly.
- Small-tensor fallback to Q8_0/F32 is documented per-tensor.
- No runtime infer/quality claim. Artifact still partial pack.
