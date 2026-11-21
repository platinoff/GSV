# Local model roles and NanoJev verification (2026-11-21)

**Status:** Verified (static + rusty-engine inspect)  
**Scope:** Local model inventory in Rust workspace and role separation for GSV ecosystem routing/symbiosis.

## 1. NanoJev verification

- **Path:** `S:\rust\rusty-engine\data\models\nanojev\` (~2.3 GB, kept)
- **Main artifacts:** `best.safetensors` (2,385,039,280 B), `config.json`, `summary.json`, `tokenizer/`, `predictions_test.jsonl`, `unified_hard_test.jsonl`, `unified_hard_manifest.json`, `TRAINING_RECIPE.md`, `backbone_config/`.
- **Checkpoint identity:** `schema_version=nanojev-unified-games-v1`, `model=Qwen/Qwen3-0.6B`, SFT unified-games policy (maze/snake/shooting basic+predict_position), selection/best-step context in `summary.json`.
- **Verification command:**
  - `S:\rust\rusty-engine\target\release\nanojev.exe inspect --checkpoint S:/rust/rusty-engine/data/models/nanojev`
  - **Result:** `tensors: 322`, readable safetensors header/tensor map (Qwen3-0.6B-style backbone: `embed_tokens [151936,1024]`, 28 transformer layers, q/k/v/o projections, norms, MLP gate/up/down).
- **Integration:** `rusty-engine` provides `nanojev` CLI/bin (`inspect`, `predict`, `replay`) and module `rusty_engine::nanojev::*`.

## 2. Role separation (important)

| Asset class | Examples | Use | Omni GGUF serve lane |
|---|---|---|---|
| **GGUF LLM/VLM weights** | Granite A800M, MiniCPM-V 4.7 1B/35B (+mmproj), Inkling 2 Mini UD-Q4_K_M (+mmproj-F16) | Local inference/routing via bunke-rock/rusty-engine providers (when service+heartbeat ready) | **Yes** (file/heartbeat-gated catalog entries) |
| **NanoJev safetensors DecisionModel** | `nanojev/best.safetensors` | Decision eval/replay/predict experiments, WSP/Astra/NanoJev research context | **No** (not a GGUF omni serve model) |
| **Mesh shard/cache** | `L:\gsv_mirror\*`, `wgpu_weights/chunks/*` | Mesh/WGPU shard distribution cache | N/A (not model catalog entry) |

## 3. Current local GGUF inventory (Rust workspace)

- `S:\rust\rusty-engine\data\models\granite-3.1-3b-a800m-instruct-Q4_K_M.gguf` (1.9 GB)
- `S:\rust\rusty-engine\data\models\MiniCPM-V-4_7-1B-Q4_K_M.gguf` (641 MB) + `mmproj-model-f16.gguf` (1.1 GB)
- `S:\rust\rusty-engine\data\models\MiniCPM-V-4_7-35B-A3B-Q4_K_M.gguf` (20 GB) + `mmproj-model-f16-35b.gguf` (1.1 GB)
- `S:\rust\rusty-engine\data\models\Inkling-2-Mini-UD-Q4_K_M.gguf` (~15.47 GiB) + `mmproj-F16.gguf` (610 MB)

Catalog wiring: see `docs/gsv/GSV_OMNI_CATALOG.md` and `src/boxes/omni/catalog.rs` (local hybrid bunke-rock/rusty providers).

## 4. Operational notes

- Do **not** commit model weights/checkpoints; keep them in workspace data/model paths or approved mirror locations.
- GGUF omni lanes are routable only when provider gates pass (`host_ready`: file exists; rusty serve lanes also require fresh heartbeat).
- NanoJev replay/predict smoke should be run on demand via rusty-engine CLI; treat as eval/research workflow, separate from mesh/GGUF serving.
- Related plan: `docs/superpowers/plans/2026-11-21-ecosystem-symbiosis-tuning.md` (SYM-1/2/4/6).