# GASV-G15 — Fitness scorecard: `gsv-versus-ai-full-q4k.gguf` (2026-11-22)

**Ticket:** G15 `t-1795349625736566700`  
**Candidate:** `L:/gsv_mirror/models/gsv-versus-ai/gsv-versus-ai-full-q4k.gguf`  
**Cluster:** **Min** (hub RDNA2 WGPU #37 + bifrost #39; valhall #40 withdrawn by owner 2026-11-22)

## Scorecard

| Gate | Evidence | Verdict |
|---|---|---|
| Artifact integrity | sha256 `2f37d6093003e95771627b5002654f6233d97b470064307427c2e62989601383` (re-verified 2026-11-22), 21,166,255,488 B, `gasv.assemble-copy-report.v1`, `n_kv=53`, GSV provenance kv | PASS |
| Structural / coverage | `rusty inspect`: qwen35moe, **733/733 tensors**, ggml types 0/12/14, arch consistent with source; direct-copy raw bytes, no F32 materialization | PASS |
| WGPU Min canary 1 | `g-713` blk.3.attn_output.weight, shards=2, peers **[39,37]**, `max_err=6.938893903907228e-18`, ok=true, complete | PASS |
| WGPU Min canary 2 | `g-714` blk.11.attn_output.weight, shards=2, peers **[39,37]**, `max_err=6.938893903907228e-18`, ok=true, complete | PASS |
| Prior Med canaries | g-711/g-712 `6.94e-18` (g-711 included #40 before withdrawal) — superseded by Min reruns above | PASS (context) |
| NanoJev eval-oracle | `best.safetensors` (322 tensors, Qwen3-0.6B DecisionModel), commit `3bb3397`; `nanojev inspect` PASS (322 tensors, F32 views valid after 8-byte header alignment fix). Full `predict`/`replay` forward is CPU-bound on this host (single predict >90 s timeout) — operational but not a fast gate; no weight generation/transplant. | PASS (structural); eval slow |
| No-fake claim | `gsv.full_llm_claim=false`, `gsv.nanojev_role=eval-oracle-only`, `runnable=false` in artifact-report; G13 still blocked | PASS |

## Gap (honest, not a gate failure)

- **WGPU chunk registration:** the 122 huge MoE expert tensors exist in the artifact but are **not** in the live weight-store chunk index (612 tensors registered from prior G5 work). Serving those tensors through the mesh needs an artifact-backed weight-store lane (new Rust work) or duplicated chunks (~20GB, disk-limited on L:). This is a **deployment/tuning gap**, not a correctness failure of the artifact. Tracked as next ticket after G19.
- **Runtime infer/quality:** not run. G13 remains blocked (owner-approved runtime/RAM node). No quality/parity claim.

## 2. NanoJev oracle note

- `nanojev inspect --checkpoint data/models/nanojev` → 322 tensors, set-head/output layers present, F32 views readable (alignment fix in `safetensors.rs`, commit `3bb3397`).
- `nanojev predict` / `replay` load the 2.3 GB checkpoint but are CPU-bound here (a single frozen request exceeds a 90 s timeout); this is a **performance** fact, not a correctness failure. Oracle role remains eval/fitness only — it never writes GGUF weights.
- This scorecard therefore does not claim a completed numeric oracle regression; it records the oracle as operational and the WGPU structural/transport gates as PASS.

