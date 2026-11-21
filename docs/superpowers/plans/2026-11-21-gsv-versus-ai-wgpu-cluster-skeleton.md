# GASV-VERSUS-AI — Wi-Fi WGPU cluster skeleton / weight-mirror research plan

**Date:** 2026-11-21  
**Status:** A–F executed (2026-11-21). F3 eval gate passed (parity 121/121, med canaries #37/#39/#40 <1e-4); F4 owner-gated publish done (variant 1, canonical Q8_0); F5 regression recorded. Artifact remains **partial shexp-pack** (no runnable full LLM, no quality/parity claim). 
**Owner ask:** перевірка екосистеми, ресерч/адаптація харнесів + MCP toolchain, доадаптація GSV MCP openbot / poolAI / Kubernetes / WGPU mirror / NanoJev; робота з WGPU у Wi-Fi кластері через NanoJev; зібрати наново ваги у mirror з поточних моделей (де можливо); спробувати зібрати skeleton з random weights під Wi-Fi cluster архітектуру; потім через NanoJev + regrep повитягувати ваги з поточних моделей у Rust у mirror і назвати 4-квантовану збірку `gsv-versus-ai.gguf`.  
**Rule:** спочатку план з deep research, потім тікети. Жодних ваг/секретів у git. Rust-first. MSYS2 bash. No `rebook/**`.

## 0. Executive summary (чесно)

Це **research + staged engineering** план, не “одна команда — і готова модель”.

- **Можливо зараз (низький ризик):** аудит/док-план, inventory моделей/мірору/кластеру, адаптація MCP/xtask/harness спостережуваності, формалізація cluster sets, GGUF skeleton/spec research, NanoJev eval/replay як decision/oracle сигнал (не weight transplant engine).
- **Можливо з середнім ризиком (Rust engineering):** GGUF writer/skeleton builder у `rusty-engine` (або окремий crate), deterministic random-init GGUF skeleton, mirror manifest/packing pipeline, weight-mapping/regrep з існуючих GGUF тензорів у сумісний layout, Q4 quantization/export.
- **Високий ризик / research-needed:** “витягнути ваги через NanoJev” з несумісних моделей у новий skeleton. NanoJev — **safetensors DecisionModel** (Qwen3-0.6B, 322 tensors, eval/replay) — **не GGUF writer і не універсальний weight transplant tool**. Трансплант ваг можливий лише там, де є **сумісність архітектури/tensor shapes/dtype/quant layout** або явно визначений конвертер.
- **Не зараз:** Kubernetes як primary для поточного fleet (owner/hardware reality: hub+phones, Pi dead; k8s synthesis каже “не для цього fleet зараз”). Max set з Pi — задати як target spec, не тестити до появи hardware.

## 1. Current state (facts, 2026-11-21)

### 1.1 Cluster / mesh / WGPU

| Item | Fact | Evidence |
|---|---|---|
| **Minimal set (execution target)** | **Hub RDNA2 WGPU + bifrost** (PC WGPU = Firefox RDNA2 peer; зараз #37 stale/navigator.gpu blocked на ПК, bifrost #39 alive/ready) | `docs/gsv/MESH_ROADMAP_TASKS_2026-11-19.md` + owner 2026-11-21 |
| Medium set | **Hub + bifrost + valhall** (valhall #40 alive/ready 2026-11-21) | MESH_ROADMAP_TASKS + owner 2026-11-21 |
| Max set (deferred spec) | **Medium + Pi4** через ~місяць (можливо; NAND/SD/remote owner-gated) | Owner ask 2026-11-21 |
| Hub | `gsv-server` `:9999`, crate 0.248.0, RAM pressure | HANDOFF/MESH docs |
| WGPU mirror | **Підтверджено owner:** peer mirror на диску `L:` — `L:\gsv_mirror\wgpu_weights\` (chunks + blk f32 shexp); ~184M total seen | local fs snapshot 2026-11-21 + owner confirm |
| L: snapshot 2026-11-21 | `gsv_mirror/blk7..12` shexp f32 = 18×4MiB (72MiB); `wgpu_weights/chunks` = 31 chunks, ~112MiB, `index.json` present, hits>0; `wgpu_weights_tensors.json` не видно в цьому dir | `ls/du/find` snapshot 2026-11-21 |
| Mesh op proven | `weights_matvec` shared-expert 35B-A3B, 40/40 block-verified history | HANDOFF |
| Dispatch | predicted-latency EWMA-tier routing, proof gate, fail-open | MESH_DISPATCH_RESEARCH |
| Constraints | 2.4 GHz Wi-Fi, phone reconnect thrash, RAM 7.7 GB hub, no Pi SD | HANDOFF/MESH_ROADMAP_TASKS |

### 1.2 Models / engines

| Item | Fact | Evidence |
|---|---|---|
| GGUF local models | Granite A800M, MiniCPM-V 4.7 1B/35B (+mmproj), Inkling 2 Mini UD-Q4_K_M (+mmproj-F16) in `S:\rust\rusty-engine\data\models\` | local fs + LOCAL_MODEL_ROLES |
| NanoJev | safetensors DecisionModel, `nanojev-unified-games-v1`, Qwen3-0.6B, **322 tensors**, inspect OK | `docs/gsv/LOCAL_MODEL_ROLES_2026-11-21.md`, NanoJev runbook |
| rusty-engine GGUF | has `gguf.rs` reader/parser; **no GGUF writer found** (`write_gguf/GgufWriter/save_gguf` absent in src) | `rusty-engine/src/gguf.rs`, grep 2026-11-21 |
| rusty-engine arch | MoE GGUF-first, qwen35/gdn/qwen2moe paths; nanojev module exists | `rusty-engine/docs/ARCHITECTURE.md`, src tree |
| llama-rs | llama.cpp-backed engine; bunke-rock lane in GSV omni | GSV omni catalog/FM |

### 1.3 Control plane / MCP / plans

| Item | Fact | Evidence |
|---|---|---|
| GSV MCP | `gsv_mcp_openbot` hub, stdio live `target/live/gsv-mcp.exe`, Cursor HTTP 9999/mcp | `docs/gsv/GSV_MCP_OPENBOT.md` |
| poolAI FM | canonical dad/management FM; WSP brief/sketch exist | `poolAI/docs/catalog/FUNCTION_MANAGEMENT.md`, WSP docs |
| K8s | GSV synthesis: not product for current fleet; poolAI has deployment/K8S doc | `docs/gsv/K8S_WGPU_SYNTHESIS_2026-11-18.md`, `poolAI/docs/deployment/KUBERNETES.md` |
| Steel plans | symbiosis plan, CNV-1 wave plan, ecosystem next tasks, mesh roadmap, WSP plans | `docs/superpowers/plans/*`, `docs/gsv/*` |

## 2. Goal decomposition

### G1 — Ecosystem verification / “steel plan” alignment
- Verify all registered repos: git state, FM, MCP wiring, toolchain, key gates, model/mirror refs.
- Update plans/docs where reality drifted (mesh sets, NanoJev role, mirror layout, CNV backlog, WSP posture).

### G2 — Harness/MCP/toolchain adaptation
- Extend GSV MCP/openbot observability for mesh+mirror+model-skeleton workflows (read-only first).
- Add xtask/diagnostic surfaces for: mirror inventory, GGUF metadata scan, skeleton manifest validation, cluster-set readiness.
- Keep folder-scoped MCP, no User-scope leaks, no secrets.

### G3 — WGPU Wi-Fi cluster through NanoJev (realistic scope)
- **Owner intent (2026-11-21):** NanoJev у poolAI — напрям заміни RL цим методом (weight-space/decision-method lane), не “ще один RL”. У GASV pipeline NanoJev лишається **decision/eval/oracle** сигналом (safetensors), **не** GGUF weight generator і **не** прямий трансплантер ваг у довільний GGUF skeleton.
- Define how NanoJev signals can influence routing/skeleton params/eval acceptance (advisory first); будь-який “transplant” лише через сумісний manifest+gates.

### G4 — Mirror weight rebuild / skeleton
- Inventory existing GGUFs and mirror chunks; define canonical mirror manifest.
- Design `gsv-versus-ai.gguf` skeleton: architecture family, tensor set, experts, context, dtype/quant target (Q4), random-init seed policy.
- Build Rust pipeline: skeleton GGUF writer → random init → (optional) selective weight transplant from compatible tensors → quantize Q4 → validate.

## 3. Deep research tracks (before implementation tickets)

### R1. GGUF format/write capability (rusty-engine)
**Question:** can we add a safe GGUF writer/skeleton exporter in Rust?  
**Research:**
- Map current `gguf.rs` read structures to writer requirements (header, kv, tensor infos, alignment, data offsets).
- Identify minimum GGUF subset for target arch (likely qwen35moe/qwen-family used by rusty-engine).  
- Define deterministic random-init (seeded PRNG, per-tensor stats) for reproducible skeletons.
- Validate with `rusty inspect/validate` and llama.cpp/llama-rs readers where possible.

**Deliverable:** `docs/gsv/research/GGUF_SKELETON_WRITER_RESEARCH.md`.

### R2. Weight transplant feasibility (regrep from current models)
**Question:** which tensors can be copied/transformed from existing models into target skeleton?  
**Research:**
- Tensor name/shape/dtype map per source model (Granite A800M, MiniCPM 1B/35B, Inkling).
- Classify: **direct-copy compatible**, **shape-compatible but quant/dtype convert**, **architecture-incompatible**, **needs projection/adaptation (research-only)**.
- Define “regrep” as auditable mapping manifest (`source_model:tensor -> target:tensor + transform + checksum`), not ad-hoc copy.

**Deliverable:** `docs/gsv/research/WEIGHT_TRANSPLANT_MATRIX.md` + machine-readable manifest schema.

### R3. NanoJev role in weight pipeline
**Question:** how NanoJev can help without pretending to be a GGUF transplant engine?  
**Research:**
- NanoJev as eval/decision oracle: acceptance tests, routing hints, skeleton param search fitness.
- Optional: use NanoJev to score candidate skeleton configs (context/experts/layers) in small eval loop.
- Explicit non-goal: direct safetensors→GGUF weight transplant into arbitrary arch.

**Deliverable:** `docs/gsv/research/NANOJEV_WEIGHT_PIPELINE_ROLE.md`.

### R4. Wi-Fi WGPU cluster sets and ops envelope
**Question:** what is stable for min/med/max sets?  
**Research:**
- Define cluster profiles: `min=hub RDNA2 WGPU+bifrost`, `med=hub+bifrost+valhall`, `max=med+pi4 (~month, maybe, spec/gated)`.
- Capacity, latency, reconnect thrash, proof gate, mirror bandwidth/size budgets.
- WGPU op coverage needed for skeleton validation (matvec/matmul/layernorm/softmax/mlpblock; weights_matvec block path).

**Deliverable:** `docs/gsv/research/WGPU_CLUSTER_SETS_2026-11.md`.

### R5. poolAI/K8s/MCP harness adaptation
**Question:** where K8s belongs, and where not.  
**Research:**
- K8s only for future dedicated GPU node scenarios; no k8s for current phone fleet.
- poolAI FM/WSP integration points: job kinds for skeleton build/eval, artifact hash registry, not on-chain weights.
- GSV MCP openbot read-only tools first; mutating/build tools behind explicit flags.

**Deliverable:** update `K8S_WGPU_SYNTHESIS` addendum + `docs/gsv/GSV_MCP_OPENBOT.md` delta plan.

## 4. Proposed target artifact: `gsv-versus-ai.gguf`

> **Name reserved by owner request.** No claim of quality/parity until eval gates pass.

### 4.1 Skeleton-first definition

| Field | Initial proposal (research to confirm) |
|---|---|
| Filename | `gsv-versus-ai.gguf` |
| Location | `L:\gsv_mirror\models\gsv-versus-ai\` (not git) + manifest/checksums in gitignored data, metadata doc in docs |
| Build mode | skeleton random-init first; selective compatible transplant second; quantize third |
| Quant target | **Q4_K_M** (or project-selected Q4 variant supported by target runtime) |
| Cluster target | min/med Wi-Fi WGPU sets; hub-constrained |
| Provenance | manifest with source tensor hashes, transforms, seed, tool versions |

### 4.2 Safety/gate rules

- Never commit weights; commit only manifests/schemas/docs.
- Every generated GGUF gets: SHA256, tensor count, arch, context, quant, seed, source manifest.
- No “transplant” without shape/dtype/semantics check.
- Eval gate before naming artifact “ready”: load/inspect/short infer/smoke on selected engine.

## 5. Architecture sketch (pipeline)

```text
[Current GGUF models] --inspect/regrep--> [Tensor map + compatibility matrix]
           |                                        |
           +--------------------+                   |
                                v                   v
                     [Skeleton spec YAML/JSON] -> [rusty-engine gguf writer]
                                |                   |
                                v                   v
                        [random-init GGUF skeleton] |
                                |                   |
                                +----selective transplant (compatible only)----+
                                                    |
                                                    v
                                         [quantize/export Q4]
                                                    |
                                                    v
                                         [gsv-versus-ai.gguf + manifest]
                                                    |
                                                    v
                                    [mirror publish + WGPU cluster smoke eval]
                                                    |
                                                    v
                                        [NanoJev/oracle eval scorecard]
```

**NanoJev role in diagram:** oracle/eval/fitness, not the binary weight copier.

## 6. Ticket plan (create after owner review)

### Phase A — Research/docs (no weight writes)
- **GASV-A1 (gsv):** ecosystem verification matrix + doc/plan sync — ✅ done 2026-11-21 (`t-1795270402999084900`, `6088f45`).
- **GASV-A2 (rusty-engine):** GGUF writer/skeleton feasibility doc + API sketch — ✅ done 2026-11-21 (`t-1795270403090130500`, rusty-engine `d555900`).
- **GASV-A3 (gsv):** weight transplant matrix schema + source model tensor inventory script/spec (Rust xtask, read-only) — ✅ done 2026-11-21 (`t-1795270403148226800`).
- **GASV-A4 (gsv/poolAI docs):** NanoJev role clarification + WSP/K8s boundary update — ✅ done 2026-11-21 (`t-1795270403204663600`).
- **GASV-A5 (gsv):** cluster set spec min/med/max + WGPU smoke criteria (bifrost minimal) — ✅ done 2026-11-21 (`t-1795270403265695800`).

### Phase B — Tooling (read-only / manifest)
- **GASV-B1 (rusty-engine):** GGUF metadata/manifest read-only export — ✅ done 2026-11-21 (`t-1795270985412515600`, rusty-engine `8c4dcc8`).
- **GASV-B2 (gsv xtask):** mirror inventory + checksum manifest generator — ✅ done 2026-11-21 (`t-1795270985512066300`, GSV `f72eb4c`).
- **GASV-B3 (gsv MCP):** read-only MCP tools/resources for model/mirror/cluster skeleton status — ✅ done 2026-11-21 (`t-1795270985609581700`, GSV `d27afa3`).

### Phase C — Skeleton builder (owner-approved engineering)
- **GASV-C1 (rusty-engine):** GGUF writer minimal viable (header/kv/tensors/alignment) — ✅ done 2026-11-21 (`t-1795271801803502800`, rusty-engine `dd0d2a3`).
- **GASV-C2 (rusty-engine):** deterministic random-init skeleton generator from spec — ✅ done 2026-11-21 (`t-1795271801906749600`, rusty-engine `931e13a`).
- **GASV-C3 (rusty-engine):** validation: inspect/load/smoke infer — ✅ done 2026-11-21 (`t-1795271802006964200`, rusty-engine `52cb0bd`).

### Phase D — Weight transplant / quant (strict gates)
- **GASV-D1:** compatible tensor transplant engine (manifest-driven) — ✅ 2026-11-21 (`t-1795272283289364000`, `6fa1bd4`).
- **GASV-D2:** incompatible tensor policy (skip/project/research flag; no silent corruption) — ✅ 2026-11-21 (`t-1795272283395485900`, `b9aabda`).
- **GASV-D3:** Q4 quant/export pipeline + checksums/provenance — ✅ 2026-11-21 (`t-1795272283500474100`, `90a2a02`; supported Q4_0/Q8_0, Q4_K_M parity-gated deferred).
- **GASV-D4:** publish `gsv-versus-ai.gguf` to mirror after eval gate — ✅ 2026-11-21 (`t-1795272283607411700`): owner confirmed publish after eval-gate smoke (inspect/manifest/publish-check dry-run OK; validate/infer N/A for minimal gsv skeleton; no quality/parity claim). Republished as `arch:"gsv"` (owner-confirmed): `L:\gsv_mirror\models\gsv-versus-ai\` (`gsv-versus-ai.gguf` + manifest, sha256 `17aa63ef…` verified). No git weights.

### Phase E — Cluster eval
- **GASV-E1:** min set (hub+bifrost) smoke: load/dispatch/proof/latency — ✅ 2026-11-21 (`t-1795272283721764300`): hub healthy (`0.248.0`), bifrost **peer 39 alive=true** (38 present/alive=false). Owner-go canary `mt-641` `weights_matvec blk.7.ffn_gate_shexp.weight` shards=1 → **done** on peer 39, `max_err_model=5.55e-17`, `elapsed_ms~261`, `8.03 MFLOPS`. Fail-open/proof-gate unchanged.
- **GASV-E2:** med set (+valhall) opportunistic — ✅ 2026-11-21 (`t-1795272283847007500`): valhall rejoined as peer **#40** (`arm/valhall`, phone) `alive/ready`. Probe `mt-644` `matmul64` done `65.5ms/op · 8.0 MFLOPS · max_err=0`. Smoke `mt-646` `weights_matvec blk.7.ffn_up_shexp.weight` shards=1 routed to peer 40, status `done`. No failover incident induced.
- **GASV-E3:** max set spec only (Pi deferred) — ✅ spec-only 2026-11-21 (`t-1795272283960030800`): Pi4 max set is documentation-only until hardware restored; no tests/dispatch; requires owner gate, dedicated node/RAM, restored mesh-node remote, and reproducible parity evidence before any activation.

### Phase F — Proper shexp-pack follow-up (real transplant candidate)

Research: [`docs/gsv/research/GASV_F_PROPER_NEXT_STEPS_2026-11-21.md`](../../gsv/research/GASV_F_PROPER_NEXT_STEPS_2026-11-21.md).

**Candidate fact (2026-11-21):** `rusty skeleton` + `rusty transplant-mirror` produced temp `gsv-versus-ai.gguf` from mirror shexp tensors: 518,004,896 B, 121 tensors, 121/121 direct-copy, `kept_skeleton 0`, sha256 `591b6310833e873159f48179ff873b6c8bad7c2da642181452b93214fa8a7669`. Inspect/manifest/publish-check dry-run OK. **Not a runnable full LLM** (partial shexp pack; validate/infer N/A; no quality/parity claim).

- **GASV-F1 (rusty-engine+gsv):** artifact contract/provenance — ✅ 2026-11-21 (`t-1795272284060030800`, rusty-engine `ebb7b79`): `gasv.artifact-manifest.v1` + `rusty artifact-report`; transplant report sha256 `12096faf9…`; candidate 518004896B/121/F32/seed20261121 sha256 `591b6310…`; NanoJev eval-oracle-only, no quality/parity claim.
- **GASV-F2 (rusty-engine):** Q8_0/Q4_0 export evaluation — ✅ 2026-11-21 (`t-1795272284170030800`, rusty-engine `ebb7b79`): `rusty quantize-export` + `gasv.quant-export-report.v1`. Q8_0 137601184B sha256 `99938926…` ratio 3.7647 max_abs_err_max=0.002277 mean_abs_err_avg=0.00005008; Q4_0 72851616B sha256 `cf0a5e40…` ratio 7.1111 max_abs_err_max=0.049953 mean_abs_err_avg=0.00091565. Q4_K_M parity-gated/deferred. No publish.
- **GASV-F3 (gsv):** proper eval gate — ✅ 2026-11-21 (`t-1795272284280030800`): structural smoke OK (inspect/manifest/publish-check; candidate 518004896B/121/F32 sha256 591b6310…, transplant 12096faf9…); parity CONFIRMED `ok=121 bad=0 total=121` (aligned data_offset 8352, 121 tensors vs mirror chunks/tensors.json). Med weights_matvec canaries (<1e-4) across #37/#39/#40: `mt-685` blk.7.ffn_gate_shexp peer 40 valhall `max_err_model=1.2815e-8`; `mt-686` blk.7.ffn_up_shexp peer 39 bifrost `1.9334e-8`; `mt-699` matmul64 probe peer 37 pc-wgpu `max_err=0` (readiness bootstrap); `mt-701` blk.13.ffn_gate_shexp peer 37 pc-wgpu `max_err_model=1.3709e-8`. No infer/quality claim (partial shexp pack).
- **GASV-F4 (gsv, owner-gated):** publish decision — ✅ 2026-11-21 (`t-1795272284390030800`, owner confirmed variant 1, canonical=Q8_0): published `L:\gsv_mirror\models\gsv-versus-ai\` — canonical `gsv-versus-ai.gguf`=Q8_0 sha256 `99938926…` (137601184B), `gsv-versus-ai-f32.gguf` `591b6310…` (518004896B), `gsv-versus-ai-q4_0.gguf` `cf0a5e40…` (72851616B), prior 16KB skeleton preserved as `gsv-versus-ai-skeleton-16kb.gguf` `17aa63ef…`; manifest + publish set + transplant/quant/artifact reports written. No git weights.
- **GASV-F5 (gsv):** med cluster regression record + docs — ✅ 2026-11-21 (`t-1795272284500030800`): post-F4 med regression across #37 pc-wgpu (mt-699 probe max_err=0 ready bootstrap, mt-701 blk.13 gate max_err_model=1.3709e-8), #39 bifrost (mt-686 1.9334e-8, mt-690 1.0673e-8, mt-691 1.5944e-8, mt-692 1.8757e-8, mt-693 2.1420e-8), #40 valhall (mt-685 1.2815e-8); all <1e-4. Firefox LAN HTTP lacks navigator.gpu; localhost SSH port-forward workaround documented. No infer/quality claim.

## 7. Gates / Definition of Done

- **Research DoD:** cited facts, file paths, feasibility class, risks, explicit non-goals.
- **Code DoD:** `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, relevant tests; one commit per repo.
- **Artifact DoD:** GGUF inspectable, checksumed, manifest present, no git weight blobs, smoke eval recorded.
- **Cluster DoD:** min set smoke reproducible; med set opportunistic; max set documented only.

## 8. Risks / anti-goals

- Treating NanoJev safetensors as universal GGUF weight source — **no**.
- Silent tensor shape coercion/transplant — **no**.
- Kubernetes-first detour for current phone fleet — **no**.
- Committing model weights/mirror binaries — **no**.
- Big-bang “all models merged” claim before compatibility matrix — **no**.

## 9. Phase F outcome (2026-11-21)

1. **F1–F5 done:** artifact contract/provenance, Q8_0/Q4_0 eval, parity 121/121, med canaries #37/#39/#40 <1e-4, owner-gated publish (variant 1, canonical Q8_0), regression record.
2. **Published:** `L:\gsv_mirror\models\gsv-versus-ai\` — canonical `gsv-versus-ai.gguf` (Q8_0, 137601184B, `99938926…`), `gsv-versus-ai-f32.gguf` (`591b6310…`), `gsv-versus-ai-q4_0.gguf` (`cf0a5e40…`), prior 16KB skeleton preserved, manifests/reports included.
3. **Caveat:** partial shexp-pack only; not runnable full LLM; no infer/quality claim. Next real step beyond this plan requires full tensor coverage/architecture compatibility matrix and owner-approved inference target.

## 10. Reference files

- `docs/gsv/GSV_ECOSYSTEM_SYMBIOSIS.md`, `docs/gsv/GSV_FUNCTION_MANAGEMENT.md`, `docs/gsv/LOCAL_MODEL_ROLES_2026-11-21.md`
- `docs/gsv/MESH_ROADMAP_TASKS_2026-11-19.md`, `docs/gsv/MESH_DISPATCH_RESEARCH_2026-11-18.md`, `docs/gsv/K8S_WGPU_SYNTHESIS_2026-11-18.md`
- `docs/gsv/NANOJEV_PORT_FEASIBILITY_2026-11-18.md`, `docs/gsv/GSV_MCP_OPENBOT.md`
- `rusty-engine/docs/ARCHITECTURE.md`, `rusty-engine/src/gguf.rs`, `rusty-engine/src/nanojev/*`
- `poolAI/docs/development/WSP_WEIGHT_SPACE_PROGRAMMING_BRIEF.md`, `poolAI/docs/development/WSP_HYBRID_ARCHITECTURE_SKETCH.md`
- `L:\gsv_mirror\wgpu_weights\` (local mirror, not git)