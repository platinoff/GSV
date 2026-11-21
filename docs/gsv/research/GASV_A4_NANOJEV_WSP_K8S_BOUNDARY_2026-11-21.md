# GASV-A4 — NanoJev role clarification + WSP/K8s boundary update

**Date:** 2026-11-21  
**Ticket:** `t-1795270403204663600` (gsv, in_progress)  
**Scope:** docs only. No code/weights/secrets.  
**Plan:** [`docs/superpowers/plans/2026-11-21-gsv-versus-ai-wgpu-cluster-skeleton.md`](../../superpowers/plans/2026-11-21-gsv-versus-ai-wgpu-cluster-skeleton.md)

## 1. Owner intent (2026-11-21, authoritative for GASV planning)

- **NanoJev у poolAI = напрям заміни RL цим методом** (weight-space/decision-method lane), **не** “ще один RL”.
- **Mirror для пірів** — диск `L:`: `L:\gsv_mirror\wgpu_weights\` (підтверджено).
- Решта GASV плану прийнятна після цієї корекції.

## 2. NanoJev role boundary

| Layer | Role | Allowed now | Not allowed |
|---|---|---|---|
| poolAI (тато/FM/WSP) | NanoJev/weight-space method as RL-replacement direction; WSP/ActivationAudit/WeightPatch sketch lane | docs/research, gated eval protocol | un-gated weight patching, production RL replacement claim |
| rusty-engine | NanoJev safetensors DecisionModel (Qwen3-0.6B, 322 tensors) | inspect/predict/replay eval, research | GGUF writer, universal weight transplant, runtime omni GGUF lane |
| GSV mesh/WGPU | NanoJev as **eval/oracle/fitness** signal for GASV pipeline | advisory scoring, acceptance gates, telemetry/docs | direct safetensors→arbitrary GGUF transplant, silent weight mutation |
| GASV artifact | `gsv-versus-ai.gguf` skeleton/transplant/quant pipeline | manifest-driven compatible transplant, checksums, eval gates | “NanoJev transplanted weights” claim without tensor-compatibility proof |

## 3. WSP boundary

- **ECO-9.1/9.2**: docs/sketch only. Hybrid ~55% feasibility; direct RL→WSP low.
- **Guards (must keep):** freeze router/gate; block checksums/base_hash; Ukrainian golden eval; canary; rollback; no binary weights in git.
- **GASV relation:** WSP provides decision/provenance discipline; GASV uses it as gate/oracle input, not as permission to bypass compatibility matrix.

## 4. K8s boundary

- **Current fleet (hub + phones, no Docker/k8s, RAM-constrained hub):** K8s is **not** the product. Source: `docs/gsv/K8S_WGPU_SYNTHESIS_2026-11-18.md`.
- **Future dedicated GPU node (≥24 GB, owner-gated):** K8s + device plugin may be reasonable; separate plan required.
- **poolAI `docs/deployment/KUBERNETES.md`:** generic deployment baseline only; no WGPU mesh/NanoJev/WSP wiring claim.

## 5. Observed gap

- `poolAI/docs/catalog/FUNCTION_MANAGEMENT.md` has no direct WSP/NanoJev/WGPU-mesh rows by targeted grep (A1/A4 observation). 
- **Action:** propose poolAI FM patch in a dedicated poolAI ticket/slice (not done here to avoid cross-repo scope creep). GSV records boundary in this doc and GASV plan.

## 6. GASV pipeline rule (normative)

> NanoJev may score, gate, or prioritize; it may not silently transform incompatible weights. Any weight movement into `gsv-versus-ai.gguf` requires: source tensor identity, shape/dtype/quant compatibility class, transform manifest, checksum, and eval gate.

**Verdict:** PASS (docs boundary clarified; owner intent recorded).