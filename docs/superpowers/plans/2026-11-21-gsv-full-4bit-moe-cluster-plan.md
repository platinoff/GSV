# GSV-VERSUS-AI Full 4-bit MoE Cluster Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Produce a full-fledged 4-bit (Q4_K/Q4_K_M-fallback) GSV-VERSUS-AI model artifact for the 35B-A3B MoE family, NanoJev-eval-gated, tuned for GSV ecosystem and Wi-Fi WGPU cluster MoE distribution across available peers, with honest no-fake-full-LLM boundaries until runtime infer/quality gates pass.

**Architecture:** Source-of-truth is `S:/rust/rusty-engine/data/models/MiniCPM-V-4_7-35B-A3B-Q4_K_M.gguf` (qwen35moe, 733 tensors, Q4_K_M, 19.7GB). Build pipeline: (1) full tensor coverage matrix + feasibility classes, (2) quantized extraction without full F32 materialization (stream/chunk direct-quant or dequant->Q4_K per tensor with staged disk budget), (3) NanoJev eval-oracle fitness gate (oracle only, no weight transplant/generation), (4) MoE expert placement/sharding plan for alive peers, (5) WGPU cluster canary/regression, (6) versioned mirror publish with manifest/checksums, (7) runtime infer/quality gate before any "runnable full LLM" claim.

**Tech Stack:** Rust stable (`rusty-engine` quantize-export/manifest/dump-f32, `GSV` wgpu mesh/tasks/tickets/xtask), MSYS2 bash, GGUF Q4_K type 12, NanoJev safetensors eval-oracle (no GGUF gen), GSV WGPU mesh peers, mirror `L:\gsv_mirror`.

**Spec:** This plan is grounded in steady states 2026-11-21: `docs/gsv/research/GASV_G5_FULL_TENSOR_MATRIX_2026-11-21.md`, `docs/gsv/research/GASV_G6_Q4_NANOJEV_WGPU_ADAPTATION_2026-11-21.md`, `docs/superpowers/plans/2026-11-21-gsv-versus-ai-wgpu-cluster-skeleton.md`, `L:\gsv_mirror\models\gsv-versus-ai\gsv-versus-ai.publish.json`.

## Global Constraints

- **Rust-first / MSYS2 bash + GSV MCP tools.** **No Python, no Perl, no PowerShell** for project work — including "throwaway"/scratch scratch scripts and JSONL edits (use MCP `gsv_tickets_*` or Rust/`cargo xtask` utilities instead). Allowed formats `.rs/.md/.mdc/.json/.js/.wasm`; no weights/secrets in git.
- **One meaningful commit per repo touched.** No `git add -A`. Push after green gates.
- **NanoJev = eval/oracle/fitness only.** No GGUF generation, no weight transplant, no silent coercion.
- **No "full working LLM" claim** until full tensor graph coverage + runtime infer/quality gate pass on owner-approved target.
- **Owner-gated publish/canonical change.** Mirror publish requires checksums/manifests/reports; no git weight blobs.
- **Disk/RAM budget:** S: ~44G free, L: ~40G free, host RAM ~7.7GB. Never materialize 123.79GB F32 MoE experts at once. Stage/chunk/quantize and delete temp F32.
- **Cluster steady state (2026-11-22):** owner withdrew **valhall #40** → active **Min = hub RDNA2 WGPU (#37) + bifrost #39**; Med = Min + #40 (spec, owner returns phone); Max = Med + Pi4 #36 (spec/gated). #38/#41 dead. Gates run in Min; Med/Max only add throughput. Canaries must respect proof/readiness/fail-open.
- **Q4 target:** Q4_K (type 12) supported; Q4_K_M is `q4_k_m-no-imatrix-fallback=q4_k` unless Q6_K encoder+imatrix implemented. Report policy explicitly.

## Steady State (2026-11-21)

| Item | Value |
|---|---|
| Source 35B | `MiniCPM-V-4_7-35B-A3B-Q4_K_M.gguf`, qwen35moe, 40 layers, 256 experts, 8 used, 733 tensors, 19.7GB, Q4_K_M |
| GSV-VERSUS-AI canonical | G2b F32 706,495,136B sha256 `25fadfbb...`, 502 tensors, partial pack |
| Live wgpu index | 612 tensors (501 prior + 110 large + G2b coverage) |
| Missing for full 35B coverage | 122 huge MoE expert tensors, 123.79GB F32 (ffn_{gate,up,down}_exps.weight, 3D 256x512x2048 class) |
| Q4_K export | rusty-engine `692b8ce,bf60c90` supports `q4_k|q4_k_m` fallback; smoke report `883f2c03` |
| NanoJev | eval/oracle only, checkpoint `S:/rust/rusty-engine/data/models/nanojev/best.safetensors` |
| Mirror | L: 40G free; staging_g5 4.7G; gsv-versus-ai 2.4G |

## Ticket Map (GSV rules)

| Ticket | Title | Status | Depends | Owner gate |
|---|---|---|---|---|
| `t-1795273000000000010` | G8 Research: full 35B MoE 4-bit GSV/cluster plan synthesis | done | - | no |
| `t-1795273000000000011` | G9: Q4_K direct-copy full 733-tensor assembly (Strategy C) | done (G9 artifact `2f37d609…`) | G8 | no publish |
| `t-1795273000000000012` | G10: Full-tensor manifest/publish-prep reconcile | todo (artifact-report exists; ticket stale) | G9 | publish owner-gated |
| `t-1795273000000000013` | G11: NanoJev eval-oracle fitness + WGPU regression | open (canaries g-711/g-712 PASS `6.94e-18`; reclaimed lease — finalize scorecard) | G10 | no (eval only) |
| `t-1795273000000000014` | G12: MoE cluster placement/tuning — **Min active** (#37+#39) | todo | G10,G11 | canary dispatch reasonable |
| `t-1795273000000000015` | G13: Runtime infer/quality gate (full LLM claim gate) | blocked | G12 | owner-approved runtime target + RAM/node |

> 2026-11-22 adaptation: G14 research/adaptation doc + rusty-engine NanoJev commit `3bb3397`; tickets G15–G19 created via GSV MCP for fitness finalize, Min placement, Astra-style candidate audit, vision/mmproj feasibility, staged patch candidate. See `docs/gsv/research/GASV_G14_WEIGHT_FIX_CLUSTER_ADAPTATION_2026-11-22.md`.

## Phase G8 — Research (this document)

### R1. Tensor coverage reality

- 733 total tensors in source 35B Q4_K_M.
- 501 already in mirror live index, 110 large-chunked extracted, 122 huge MoE experts blocked by F32 size.
- Huge experts are 3D MoE tensors (`ffn_gate_exps.weight`, `ffn_up_exps.weight`, `ffn_down_exps.weight`, 256 experts x 512 x 2048 or inverse) — ~1GB F32 each class; Q4_K ~151MB/tensor.

### R2. No-full-F32 strategy (UPDATED 2026-11-21 — key finding)

- **G5 "123.79GB F32 infeasible" blocker is bypassed for 122 huge tensors by Strategy C.**
- Manifest check (`35b_manifest.json`): 122 huge tensors = `output.weight` (Q6_K, 417MB src), `token_embd.weight` (Q4_K, 286MB src), 120 MoE `ffn_*_exps.weight` (100x Q4_K, 20x Q6_K, ~151-220MB src each). **All already quantized** (no F32 in source). Total src bytes ~20.2GB, peak RAM one tensor.
- **Strategy C (selected):** direct-copy raw quantized tensor bytes from source GGUF into target GGUF. No dequant/requant, no F32 materialization. Requires ggml type/dims/numel/checksum validation and provenance.
- Strategy B fallback: only for tensors needing type conversion (none in the 122 huge set).
- Strategy A: forbidden.

### R3. NanoJev fitness role (correct)

- NanoJev checkpoint present; eval/oracle/fitness only.
- Use for: candidate accept/reject, quant-policy selection, regression signal, routing/placement fitness scoring where applicable.
- Not for: GGUF generation, weight transplant, direct weight repair.

### R4. MoE cluster placement

- Alive peers: #37 pc-wgpu 30.84 mflops, #39 bifrost 137.97, #40 valhall 328.0. #36 pi4 dead, #38/#41 dead.
- Expert placement objective: minimize Wi-Fi transfer and latency; keep hot/shared experts near fast peers; use proof/readiness gate.
- Sharding unit: per-expert tensor chunk or per-(layer,expert,proj) tensor. 256 experts/layer x 3 proj x 40 layers = 30720 expert tensors total - too many objects; group by layer/expert shard packs.
- Need placement manifest: tensor -> preferred peers -> fallback -> checksum -> chunk ids.

### R5. Runtime infer/quality gate

- Current artifact is partial pack; cannot claim runnable full LLM.
- Gate requires: full tensor coverage, compatible runtime (`rusty-engine` qwen35moe or approved runtime), load/infer smoke, quality eval, no NaN/Inf, WGPU/CPU parity on selected ops.

## Phase G9 — Staged MoE expert extraction (ticket `t-1795273000000000011`)

- [x] **Step 1: Inventory 122 huge tensors from G5 matrix** — done via `rusty manifest` + GSV docs (no Perl/Python).
  - Tooling rule: use `cargo xtask` / `rusty manifest` / GSV MCP tools only; **never Perl/Python**, even for "scratch" JSON parsing.
  - Expected: 122 tensors, ~123.79GB F32.
- [ ] **Step 2: Pick strategy C/B per tensor**
  - If source tensor already Q4_K/Q6_K/Q4_0/Q8_0 and acceptable, direct-copy bytes with provenance.
  - If F32/BF16/F16 or incompatible, dequant->Q4_K one tensor at a time, delete temp F32.
- [ ] **Step 3: Implement staged extractor**
  - Files: `rusty-engine/src/bin/rusty.rs` or `GSV/xtask` helper; output manifest JSONL in mirror staging (gitignored).
  - Test: one tensor dry-run, checksum, size, ggml type.
- [ ] **Step 4: Run serialized extraction with disk budget checks**
  - Stop if L: free < 15GB. Keep only Q4_K output + manifest.
- [ ] **Step 5: Commit code + report, no weights**

## Phase G10 — Full 4-bit assembly (ticket `t-1795273000000000012`)

- [ ] Build `gsv-versus-ai-full-q4k.gguf` (or agreed name) from staged tensors.
- [ ] Generate manifest, sha256, tensor count, quant policy, provenance, NanoJev role.
- [ ] Verify with `rusty inspect/manifest/artifact-report`.
- [ ] Owner-gated mirror publish only.

## Phase G11 — NanoJev eval-oracle fitness (ticket `t-1795273000000000013`)

- [ ] Run NanoJev oracle checks applicable to candidate (checkpoint present; no weight mod).
- [ ] WGPU canary set across alive peers (#37/#39/#40) on representative tensors.
- [ ] Record pass/fail; reject on gate failure.

## Phase G12 — MoE cluster placement/tuning (ticket `t-1795273000000000014`)

- [ ] Build placement manifest by peer capability/latency/readiness.
- [ ] Assign expert shards/packs; prefer #40/#39 for hot paths, #37 for pc-wgpu tasks.
- [ ] Run WGPU regression canaries post-placement.
- [ ] Document reconnect/fail-open behavior.

## Phase G13 — Runtime infer/quality gate (ticket `t-1795273000000000015`, blocked)

- [ ] Owner-approved runtime target and RAM/node.
- [ ] Load/infer smoke, NaN/Inf check, quality eval.
- [ ] Only then consider "runnable full LLM" claim.

## Exit Criteria

- Research + plan + tickets created: **done by G8**.
- Full 4-bit artifact built with manifest/checksums: G10.
- NanoJev fitness pass: G11.
- Cluster placement manifest + WGPU regression: G12.
- Runnable full LLM claim: only after G13 green. No fake claim.