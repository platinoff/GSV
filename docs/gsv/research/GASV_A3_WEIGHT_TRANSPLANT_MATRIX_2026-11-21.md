# GASV-A3 — Weight transplant matrix schema + tensor inventory spec

**Date:** 2026-11-21  
**Ticket:** `t-1795270403148226800` (gsv, in_progress)  
**Scope:** research/docs/spec only. **Read-only** inventory; no weight copy, no GGUF writes, no model mutation.  
**Plan:** [`docs/superpowers/plans/2026-11-21-gsv-versus-ai-wgpu-cluster-skeleton.md`](../../superpowers/plans/2026-11-21-gsv-versus-ai-wgpu-cluster-skeleton.md)

## 1. Source model inventory (local, read-only snapshot)

Source: `S:\rust\rusty-engine\data\models\` (2026-11-21).

| Model file | Size | Arch/family (expected) | Role candidate | Notes |
|---|---:|---|---|---|
| `granite-3.1-3b-a800m-instruct-Q4_K_M.gguf` | 1.9G | Granite 3.1 MoE A800M | small MoE reference/control | GGUF Q4_K_M |
| `MiniCPM-V-4_7-1B-Q4_K_M.gguf` | 641M | qwen35-hybrid dense (1B) | dense/hybrid control | proven mesh shard path historically |
| `MiniCPM-V-4_7-35B-A3B-Q4_K_M.gguf` | 20G | qwen35moe (35B-A3B) | MoE expert/shared-expert source | heavy; inspect header-only |
| `Inkling-2-Mini-UD-Q4_K_M.gguf` | 16G | verify via inspect | candidate source | large; header-only inspect |
| `mmproj-F16.gguf` | 610M | vision proj | **exclude from text skeleton** | mmproj, not target text tensor set |
| `mmproj-model-f16.gguf` | 1.1G | vision proj | exclude | mmproj |
| `mmproj-model-f16-35b.gguf` | 1.1G | vision proj | exclude | mmproj |
| `nanojev/` | dir | safetensors DecisionModel | eval/oracle only | **not GGUF source** |

## 2. Compatibility classes (normative)

| Class | Meaning | Allowed action | Gate |
|---|---|---|---|
| `direct_copy` | same arch semantic, same tensor name/shape/dtype/quant layout | raw bytes copy with checksum | manifest + sha256 + shape/type check |
| `convert_safe` | same semantic + compatible shape; dtype/quant convertible | dequant/transform/requant via proven encoder | golden parity + checksum + eval |
| `projected_research` | shape/semantic mismatch but research projection possible | **no silent apply**; research-only, owner gate | explicit experiment manifest + rollback + eval |
| `incompatible` | different arch/semantic/unknown | reject | hard fail |
| `excluded` | mmproj/vision/runtime-irrelevant for target | ignore | logged as excluded |

## 3. Transplant manifest schema (proposed v1)

```json
{
  "schema": "gasv.transplant-manifest.v1",
  "generated_at": "ISO-8601",
  "target": {
    "name": "gsv-versus-ai.gguf",
    "arch": "qwen35moe|qwen35|...",
    "quant": "Q4_K_M",
    "seed": 0,
    "context_length": 0
  },
  "sources": [
    {
      "id": "minicpm-v47-35b-a3b-q4km",
      "path": "S:/rust/rusty-engine/data/models/MiniCPM-V-4_7-35B-A3B-Q4_K_M.gguf",
      "sha256": "...",
      "arch": "...",
      "size_bytes": 0
    }
  ],
  "entries": [
    {
      "target_tensor": "blk.0.ffn_gate_shexp.weight",
      "class": "direct_copy|convert_safe|projected_research|incompatible|excluded",
      "source_id": "minicpm-v47-35b-a3b-q4km",
      "source_tensor": "blk.0.ffn_gate_shexp.weight",
      "transform": "none|f32_dequant|q4k_requant|...",
      "source_sha256": "tensor-bytes-or-chunk-hash",
      "target_sha256": "expected-after-transform",
      "shape": [2048, 512],
      "ggml_type": 12,
      "notes": "reason/gate"
    }
  ],
  "policy": {
    "allow_silent_coercion": false,
    "require_checksum_match": true,
    "require_eval_gate_before_publish": true,
    "nanojev_role": "eval_oracle_only"
  }
}
```

## 4. Read-only xtask/spec (proposed)

Proposed command (no writes except explicit `--out` manifest path):

```bash
cargo xtask gasv-inventory \
  --models-dir S:/rust/rusty-engine/data/models \
  --out target/live/gasv_inventory.json \
  --format json
```

Behavior:
- Read GGUF header/KV/tensor-info only (bounded window / existing gguf parse).
- Emit model id, file size, sha256(optional slow flag), arch, n_tensors, tensor name/dims/type/offset/byte-size.
- **Never** copy tensor payloads or create GGUF.
- For large files (20G), default sha256 is file-hash via streaming read only when `--sha256` passed; metadata-only mode is default.

## 5. Initial matrix (framework, to be filled by inspect run)

| Target tensor family | Granite A800M | MiniCPM 1B | MiniCPM 35B-A3B | Inkling | NanoJev |
|---|---|---|---|---|---|
| embed/norm | inspect | inspect | inspect | inspect | incompatible (safetensors/eval) |
| attn q/k/v/o | inspect | inspect | inspect | inspect | incompatible |
| dense FFN | inspect | inspect | n/a or partial | inspect | incompatible |
| MoE routed experts | inspect | n/a | inspect | inspect (if MoE) | incompatible |
| shared expert `*_shexp` | inspect | n/a (if absent) | **candidate direct_copy for same-arch slices** | inspect | incompatible |
| mmproj/vision | excluded | excluded | excluded | excluded | excluded |

> Cells marked `inspect` require header/tensor-table read before classification. No silent assumptions.

## 6. Risks / non-goals

- No transplant without tensor-level evidence.
- No K-quant requant without parity gate.
- NanoJev safetensors are **not** GGUF tensor donors; eval/oracle only.
- `mmproj` excluded from text GASV skeleton.
- Large-model hashing/inspect must be RAM-safe (stream/header window).

## 7. Verdict

**PASS (spec/docs).** Manifest schema + compatibility classes + read-only inventory spec defined. Next implementation ticket: B1/B2 read-only inspector/manifest tooling, then C/D gated build/transplant.