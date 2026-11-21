# GASV-G3 NanoJev Eval-Oracle / Fitness Gate (2026-11-21)

**Status:** ✅ done (2026-11-21)  
**Role boundary:** NanoJev = **eval/oracle/fitness only**. No GGUF generation, no weight transplant, no silent coercion.

## NanoJev asset (present, not modified)

- `S:/rust/rusty-engine/data/models/nanojev/best.safetensors` (2.3G, 2025-11-18)
- `predictions_test.jsonl` (2.7M), `unified_hard_test.jsonl`, `config.json`, `backbone_config/`, `tokenizer/`, `source/`, `TRAINING_RECIPE.md`
- Code: `S:/rust/rusty-engine/src/nanojev/{mod,model,prepare,qwen3,replay,safetensors}.rs`
- Applicability to GASV artifact: **indirect**. NanoJev is a decision-model oracle; it does not score GGUF tensor equivalence. GASV fitness uses WGPU/parity/proof gates; NanoJev remains available as independent oracle for decision/routing evals.

## Fitness gate (G2b candidate `gsv-versus-ai-g2b.gguf`)

| Gate | Result | Evidence |
|---|---|---|
| Artifact integrity | PASS | sha256 `25fadfbb37242047126ed8cbb904acbc56f94685be487ef016d71543222c9995`, 706,495,136 B, 502 tensors, F32, arch=gsv, seed 20261121 |
| Transplant report | PASS | replaced=502, kept_skeleton=0, bytes_written=706495136 |
| Spot parity (new tensors) | PASS | `blk.0.attn_norm.weight`, `blk.20.ffn_gate_inp.weight`, `blk.39.post_attention_norm.weight` == staged F32 bins |
| WGPU probe bootstrap | PASS | `mt-703` matmul64 peer 37 pc-wgpu `max_err=0` |
| WGPU weights_matvec canary (new tensor) | PASS | `g-704` `blk.0.ffn_gate_inp.weight`: `mt-704-0` peer 39 arm/bifrost `max_err_model=3.388131789017202e-21`; `mt-704-1` peer 37 pc-wgpu `max_err_model=0.0` (<1e-4) |
| Prior F3 base parity | PASS | `ok=121 bad=0 total=121` aligned data_offset 8352 |
| Prior F3/F5 med canaries | PASS | #37/#39/#40 `max_err_model < 1e-4` (mt-685/686/690-693/699/701) |
| Runnable full LLM / infer quality | N/A | partial shexp-pack; no quality/parity claim beyond proven tensors |

## Verdict

**G3 fitness: PASS for partial-pack regrep/canary gates.**  
Not a full-LLM quality pass. NanoJev was not used to generate/transplant weights; its checkpoint is present for future oracle/eval workflows where decision-model scoring applies.