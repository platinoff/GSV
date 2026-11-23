# GASV-G18 — Vision/mmproj weight-change feasibility (2026-11-22)

**Ticket:** `t-1795349626199856600`  
**Scope:** docs/research only. No weight writes, no code changes to vision pipeline (owner-gated).

## 1. Steady-state facts

- Vision addon files (rusty-engine `data/models/`, gitignored):
  - `mmproj-model-f16.gguf` = 1,110,117,984 B, arch **`clip`**, type `mmproj`, 548M, **all weights BF16 (ggml=30)**, 460 tensors.
  - `mmproj-model-f16-35b.gguf` = 1.1 GB (pairs with 35B-A3B text core).
  - `mmproj-F16.gguf` = 610 MB.
- Layout (`rusty-engine/docs/research/minicpm-v-4.7.md` §2, PH-1201): image_size 448, patch 14, embed 1152, ffn 4304, 27 ViT blocks, `v.patch_embd`, `v.position_embd [1152×4900]`, `v.vit_merger.*` (4×/16× compressor), `mm.{input_norm,up,down}` projector. BF16 dequant = upper-16-bit widening (`dequant_bf16_widens_by_left_shift`).
- `rusty-engine/src/patch.rs` supports block patch **only** for Q8_0 (8), Q4_K (12), Q5_K (13), Q6_K (14). `F32`/`F16`/`BF16`/`Q4_0`/`Q5_0` are explicitly **UnsupportedType** (patch.rs:17, 642, 660).

## 2. Feasibility verdict

**Astra-style `pack-mint → serve --patch` on mmproj is NOT possible with the current toolchain.** BF16 tensors are 2-byte scalars with no 64/144/210-byte block, so there is no `base_hash`-pinned block to mint/patch. This is a tool limitation, not evidence about the model.

Prior research (`WSP_JEV_ASTRA_RESEARCH_2026-11-18.md` placement) marked `mmproj (vision) = ❌ not patched`; this ticket re-opened the question per owner request ("невеликий vision-додаток ... зі зміною ваг") and reaches the same mechanical conclusion with a concrete cause.

Even if a BF16 element-level patch lane were added, two gates remain:
1. **Oracle gap** — the vision path has no runnable infer/quality gate yet. PH-1204 (image-prefix) explicitly waits for B13 text core (PH-1301..1304); no end-to-end image→answer evaluator exists. Without it, any weight change cannot be scored (Gemma-law: an edit can remove an obstacle, not add an ability).
2. **Runtime gap** — G13 (runnable full LLM) is blocked; 35B text core + mmproj cannot be loaded/inferred in this environment (RAM ~7.7 GB, no dedicated node — PH-A5 permanently deferred by owner).

## 3. What would be needed (owner-gated, not started)

| # | Work | Reason |
|---|---|---|
| 1 | BF16 element-level patch lane in `rusty-engine` with SHA-256 base-hash per tensor/span and fail-closed validation | `patch.rs` has no BF16 block |
| 2 | B13 qwen35-hybrid text core (PH-1301..1304) + PH-1202/1203/1204 vision path | needed for an image-prefix eval oracle |
| 3 | Quality gate: fixed image prompts, before/after scoring, no NaN/Inf | to distinguish obstacle-removal from ability-gap |
| 4 | Owner go for code + RAM/node | all of the above is owner-gated |

## 4. Decision

Feasibility research **done**; verdict **negative for now** (tool + oracle + runtime gaps). No code, no weight writes. Keep the 4-bit MoE lane (G20–G22) as the active track; revisit vision only after owner explicitly authorizes a BF16 patch lane and a vision evaluator. G13 remains blocked; no quality/parity claim.
