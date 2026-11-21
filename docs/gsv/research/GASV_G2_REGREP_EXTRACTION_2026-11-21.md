# GASV-G2 Regrep Extraction (in-progress, 2026-11-21)

**Status:** ✅ done (2026-11-21). Sequential F32 extraction from downloaded GGUFs in Rust workspace. Total extracted: **1519 tensors** (35B 381 + 1B 301 + Granite 225 + Inkling 612), all <=16MB F32, fail=0.  
**Constraint:** no Python; no git weight blobs; no silent coercion. NanoJev = eval/oracle only.

## Source models (downloaded in S:/rust)

| Model | Arch | Tensors | <=16MB F32 | Total F32 | Action |
|---|---|---:|---:|---:|---|
| MiniCPM-V-4_7-35B-A3B-Q4_K_M.gguf | qwen35moe | 733 | 501 | 129.12 GB | ✅ extracted 381 new (121 already in mirror), staging `data/wgpu_weights_staging/35b_f32/` |
| MiniCPM-V-4_7-1B-Q4_K_M.gguf | qwen35 | 321 | 301 | 3.75 GB | ✅ extracted 301/301 <=16MB to `data/wgpu_weights_staging/1b_f32/` |
| granite-3.1-3b-a800m-instruct-Q4_K_M.gguf | granitemoe | 322 | 225 | 12.29 GB | ✅ extracted 225/225 <=16MB to `data/wgpu_weights_staging/granite_f32/` |
| Inkling-2-Mini-UD-Q4_K_M.gguf | inkling2 | 734 | 612 | 97.12 GB | ✅ extracted 612/612 <=16MB to `data/wgpu_weights_staging/inkling_f32/` |

## Policy

- **Direct-copy/dequant:** `rusty dump-f32` per tensor; dtype convert Q4_K/Q6_K->F32 is explicit dequant, recorded.
- **Feasibility cap:** extracted tensors with F32 size <= 16MB in this pass; huge 3D MoE expert tensors (129GB+ full F32 for 35B) excluded as infeasible on current disk.
- **Compatibility:** only tensors compatible with target arch/layout can be transplanted into a candidate. 35B shexp tensors already proven direct-copy into gsv-versus-ai. Other-arch tensors are staged as extracted assets, not silently transplanted.
- **Mirror:** staged under `S:/rust/GSV/data/wgpu_weights_staging/` (gitignored data area). Registration into live `wgpu_weights` index/chunks requires explicit follow-up pass and disk/gate check.

## G2b result (2026-11-21)

- **Expanded candidate:** `L:/gsv_mirror/staging_g2b/gsv-versus-ai-g2b.gguf`
- **502 tensors**, 706,495,136 B, F32, `arch=gsv`, seed 20261121
- **sha256:** `25fadfbb37242047126ed8cbb904acbc56f94685be487ef016d71543222c9995`
- **transplant-mirror:** replaced=502, kept_skeleton=0, bytes_written=706495136
- **Spot parity:** OK for `blk.0.attn_norm.weight`, `blk.20.ffn_gate_inp.weight`, `blk.39.post_attention_norm.weight` vs staged F32 bins
- **Mirror copy:** versioned `gsv-versus-ai-g2b.gguf` + manifest/report copied to `L:\gsv_mirror\models\gsv-versus-ai\` (canonical unchanged = Q8_0). 
- **Live weights:** 381 new chunks copied to `S:/rust/GSV/data/wgpu_weights/chunks`, `data/wgpu_weights_tensors.json` merged to 502 entries (gitignored data).
- **WGPU canary:** `g-704` `blk.0.ffn_gate_inp.weight` (newly added tensor) — `mt-704-0` peer 39 bifrost `max_err_model=3.388131789017202e-21`, `mt-704-1` peer 37 pc-wgpu `max_err_model=0.0` (<1e-4). Probe `mt-703` peer 37 `max_err=0`.

Still partial pack, no runnable full LLM, no quality claim.

## Next

- G3: NanoJev eval-oracle fitness on candidate (no weight generation).
- G4: WGPU cluster eval/regression on obtained artifact / broader tensor coverage for runnable full LLM (needs source + compatibility matrix).