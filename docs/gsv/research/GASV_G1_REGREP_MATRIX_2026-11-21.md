# GASV-G1 Regrep Matrix (2026-11-21)

**Status:** G1 read-only analysis (no weight mutation, no mirror publish).  
**Scope:** current mirror tensors vs published `gsv-versus-ai` partial shexp-pack candidate.  
**NanoJev role:** eval/oracle/fitness only — **not** a GGUF generator/transplanter.

## Sources (facts)

- Mirror index: `S:/rust/GSV/data/wgpu_weights_tensors.json` (121 entries, F32, chunked).
- Candidate GGUF (F32): `L:\gsv_mirror\models\gsv-versus-ai\gsv-versus-ai-f32.gguf` (518,004,896 B, 121 tensors, sha256 `591b6310833e873159f48179ff873b6c8bad7c2da642181452b93214fa8a7669`).
- Canonical published: `gsv-versus-ai.gguf` Q8_0 (137,601,184 B, sha256 `99938926831d34aa5fe56d65fb7de5005b6114a81436a354fce7ffb4d86e480e`), variant 1 owner-gated F4.
- Parity gate: `ok=121 bad=0 total=121` (aligned data_offset 8352, tensor bytes vs mirror chunks).
- WGPU med canaries: #37/#39/#40 `max_err_model < 1e-4` (F3/F5 evidence).

## Regrep classification (current 121 tensors)

| Class | Count | Bytes (F32) | Rule | Status |
|---|---:|---:|---|---|
| Direct-copy compatible (mirror chunk bytes == GGUF tensor bytes) | 121 | 518,004,896 | same name, same dims, same dtype F32, sha256/chunk parity proven | ✅ transplanted |

### Tensor groups

| Group | Tensors | Dims | Dtype | Class |
|---|---:|---|---|---|
| `blk.0.ffn_down.weight` | 1 | [3584,1024] | F32 | direct-copy compatible |
| `blk.N.ffn_gate_shexp.weight`, N=0..39 | 40 | [2048,512] | F32 | direct-copy compatible |
| `blk.N.ffn_up_shexp.weight`, N=0..39 | 40 | [2048,512] | F32 | direct-copy compatible |
| `blk.N.ffn_down_shexp.weight`, N=0..39 | 40 | [512,2048] | F32 | direct-copy compatible |

## Missing for runnable full LLM (not in mirror/candidate)

The candidate is a **partial shexp-pack**, not a full model. Missing tensor classes (35B-A3B qwen35moe-style full graph; exact names depend on source arch) include:

| Class | Expected | Current | Regrep class |
|---|---|---|---|
| Token embedding / output head | required | missing | needs-source + compatibility check |
| Attention norm / FFN norm per block | required | missing | needs-source |
| Attn Q/K/V/O (+ gate/bias where applicable) | required | missing | needs-source |
| MoE router/gate | required | missing | needs-source |
| MoE expert weights (non-shexp) | required for full MoE | missing | needs-source + large mirror budget |
| Final norm / lm_head | required | missing | needs-source |
| KV/cache/runtime metadata | runtime | N/A in static GGUF | out of scope for static regrep |

**No silent coercion:** any shape/dtype/arch mismatch must be classified `incompatible` or `needs-transform` with explicit checksum and gate; never auto-transplanted.

## Regrep manifest schema (G1 minimal)

```json
{
  "schema": "gasv.regrep-matrix.v1",
  "source": "S:/rust/GSV/data/wgpu_weights_tensors.json",
  "target": "gsv-versus-ai candidate",
  "entries": [
    {
      "source_tensor": "blk.0.ffn_down.weight",
      "target_tensor": "blk.0.ffn_down.weight",
      "class": "direct-copy",
      "dtype": "F32",
      "dims": [3584,1024],
      "sha256_chunks": ["..."],
      "status": "proven"
    }
  ]
}
```

## Next (G2-G4, gated)

- **G2:** expand transplant only for direct-copy compatible tensors with source+checksum evidence; produce new candidate + manifest/report.
- **G3:** NanoJev eval-oracle fitness (latency/max_err/proof-gate/canary pass), no weight generation.
- **G4:** WGPU med cluster eval on obtained artifact; no infer/quality claim unless full runnable graph is proven.