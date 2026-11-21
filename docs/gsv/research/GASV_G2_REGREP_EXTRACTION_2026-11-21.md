# GASV-G2 Regrep Extraction (in-progress, 2026-11-21)

**Status:** in-progress. Sequential F32 extraction from downloaded GGUFs in Rust workspace.  
**Constraint:** no Python; no git weight blobs; no silent coercion. NanoJev = eval/oracle only.

## Source models (downloaded in S:/rust)

| Model | Arch | Tensors | <=16MB F32 | Total F32 | Action |
|---|---|---:|---:|---:|---|
| MiniCPM-V-4_7-35B-A3B-Q4_K_M.gguf | qwen35moe | 733 | 501 | 129.12 GB | ✅ extracted 381 new (121 already in mirror), staging `data/wgpu_weights_staging/35b_f32/` |
| MiniCPM-V-4_7-1B-Q4_K_M.gguf | qwen35 | 321 | 301 | 3.75 GB | ✅ extracted 301/301 <=16MB to `data/wgpu_weights_staging/1b_f32/` |
| granite-3.1-3b-a800m-instruct-Q4_K_M.gguf | granitemoe | 322 | 225 | 12.29 GB | ✅ extracted 225/225 <=16MB to `data/wgpu_weights_staging/granite_f32/` |
| Inkling-2-Mini-UD-Q4_K_M.gguf | inkling2 | 734 | 612 | 97.12 GB | 🔄 extracting to `data/wgpu_weights_staging/inkling_f32/` |

## Policy

- **Direct-copy/dequant:** `rusty dump-f32` per tensor; dtype convert Q4_K/Q6_K->F32 is explicit dequant, recorded.
- **Feasibility cap:** extracted tensors with F32 size <= 16MB in this pass; huge 3D MoE expert tensors (129GB+ full F32 for 35B) excluded as infeasible on current disk.
- **Compatibility:** only tensors compatible with target arch/layout can be transplanted into a candidate. 35B shexp tensors already proven direct-copy into gsv-versus-ai. Other-arch tensors are staged as extracted assets, not silently transplanted.
- **Mirror:** staged under `S:/rust/GSV/data/wgpu_weights_staging/` (gitignored data area). Registration into live `wgpu_weights` index/chunks requires explicit follow-up pass and disk/gate check.

## Next

- Finish 1B/Granite/Inkling <=16MB extraction.
- Build machine-readable extraction manifest (source, tensor, dtype, dims, f32 bytes, sha256, class).
- G2b: register compatible 35B tensors into mirror/chunks and expand candidate only for direct-copy compatible tensors.
- G3: NanoJev eval-oracle fitness on candidate (no weight generation).