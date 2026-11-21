# GASV-G11 NanoJev Eval-Oracle Fitness Draft (2026-11-21)

**Status:** draft (prep)  
**Boundary:** NanoJev = eval/oracle/fitness only. No GGUF generation, no weight transplant, no silent coercion.

## Inputs

- Candidate: `L:/gsv_mirror/models/gsv-versus-ai/gsv-versus-ai-full-q4k.gguf` (G9 Strategy C direct-copy, pending)
- NanoJev checkpoint: `S:/rust/rusty-engine/data/models/nanojev/best.safetensors` (present, not modified)
- Alive peers: #37 pc-wgpu, #39 bifrost, #40 valhall

## Fitness gates

| Gate | Method | Pass criteria |
|---|---|---|
| Artifact integrity | sha256, tensor count 733, ggml types, GGUF parse | matches report/manifest |
| Structural parity | inspect/manifest; tensor names/dims/types vs source | 733/733 match |
| WGPU canary | weights_matvec on representative tensors across #37/#39/#40 | max_err_model < 1e-4 |
| NanoJev oracle | replay/eval oracle signal where applicable | no regression vs baseline; record only |
| No-fake claim | doc/report check | no "runnable full LLM" unless G13 green |

## Non-goals

- No NanoJev weight transplant into GGUF.
- No silent quant coercion.
- No canonical publish without owner gate.