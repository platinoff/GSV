# GASV-G9 Full 4-bit Assembly (Strategy C) — 2026-11-21

**Status:** ✅ done  
**Ticket:** `t-1795273000000000011`

## Result

- **Artifact:** `L:/gsv_mirror/models/gsv-versus-ai/gsv-versus-ai-full-q4k.gguf`
- **Report:** `L:/gsv_mirror/models/gsv-versus-ai/gsv-versus-ai-full-q4k.report.json`
- **Schema:** `gasv.assemble-copy-report.v1`
- **Source:** `S:/rust/rusty-engine/data/models/MiniCPM-V-4_7-35B-A3B-Q4_K_M.gguf`
- **Mode:** direct-copy raw tensor bytes (Strategy C), no F32 materialization, no requant.
- **Tensors:** 733/733
- **Bytes:** 21,166,255,488
- **SHA256:** `2f37d6093003e95771627b5002654f6233d97b470064307427c2e62989601383`
- **n_kv:** 53 (includes GSV provenance kv)
- **Quant policy:** source-q4k_m-direct-copy (ggml types preserved as in source; 100 Q4_K + 20 Q6_K MoE experts, output Q6_K, token_embd Q4_K, etc.)

## GSV provenance kv

- `gsv.artifact=gsv-versus-ai-full-q4k`
- `gsv.pipeline=GASV-G9-StrategyC`
- `gsv.nanojev_role=eval-oracle-only`
- `gsv.quant_policy=source-q4k_m-direct-copy`
- `gsv.full_llm_claim=false`

## Boundaries

- Not published as canonical (owner-gated). No git weights.
- NanoJev not used for weight gen/transplant.
- "Runnable full LLM" not claimed (G13 gate).