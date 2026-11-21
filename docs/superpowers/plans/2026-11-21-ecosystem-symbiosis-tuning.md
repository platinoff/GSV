# Ecosystem symbiosis tuning plan — nanojev verify, FM/rules/MCP/toolchain audit

**Date:** 2026-11-21  
**Status:** Approved-for-execution-by-tickets (analysis + plan recorded)  
**Scope:** GSV hub + registered ecosystem projects (local hybrid models, nanojev, function management, rules/MCP/toolchain compatibility)  
**Canon refs:** `docs/gsv/GSV_ECOSYSTEM_SYMBIOSIS.md`, `docs/gsv/GSV_FUNCTION_MANAGEMENT.md`, `docs/gsv/GSV_OMNI_CATALOG.md`, `docs/gsv/GSV_MCP_OPENBOT.md`, `docs/gsv/GSV_AGI_PATH.md`, `AGENTS.md`

## 1. Objective

Continue ecosystem tuning with a ticketed workflow:
1. **Verify NanoJev** artifacts/integration and define its role in the ecosystem.
2. **Regrep function management (FM)** coverage across all projects.
3. **Audit rules + MCP bot/toolchain compatibility** (GSV kit wiring, Cursor/OpenCode/Grok configs, toolchain pins).
4. **Produce a symbiosis plan** for follow-up tuning/functions and execute via tickets.

## 2. NanoJev verification (done, static/runtime-tooling check)

### Artifacts (kept)
- Path: `S:\rust\rusty-engine\data\models\nanojev\` (~2.3 GB)
- Key files: `best.safetensors` (2,385,039,280 B), `config.json`, `summary.json`, `tokenizer/` (tokenizer.json/config), `predictions_test.jsonl`, `unified_hard_test.jsonl`, `unified_hard_manifest.json`, `TRAINING_RECIPE.md`, `backbone_config/`, request samples.

### Verification evidence
- `rusty-engine\target\release\nanojev.exe inspect --checkpoint S:/rust/rusty-engine/data/models/nanojev`  
  → **322 tensors**, readable safetensors checkpoint (Qwen3-0.6B-style backbone: `embed_tokens [151936,1024]`, 28-layer transformer blocks incl. q/k/v/o/proj, norms, MLP gate/up/down, etc.).
- `config.json`: `schema_version=nanojev-unified-games-v1`, `model=Qwen/Qwen3-0.6B`, `stage=sft`, `best_step=400` context in `summary.json`, tasks `maze/policy`, `snake/policy`, `shooting/policy/{basic,predict_position}`.
- rusty-engine integration present: `src/bin/nanojev.rs`, `src/nanojev/*` (`predict`, `replay`, `inspect` CLI).

### Compatibility / symbiosis role
- **Format:** safetensors DecisionModel checkpoint (not GGUF). **Not** a drop-in for current GGUF-based local omni serve lanes (`bunke-rock`/`rusty-serve` GGUF gates).
- **Use:** decision/eval/replay experiments in `rusty-engine` (nanojev CLI, WSP/Astra/NanoJev research context). Keep as local research/eval asset; do not route through omni GGUF catalog.
- **Next:** optional ticketed integration checks (replay smoke on sample cohort subset) when cheap; document runbook in rusty-engine docs if needed.

## 3. Local model inventory (post-tuning, current)

All in Rust workspace paths (model weights not committed):
- `S:\rust\rusty-engine\data\models\granite-3.1-3b-a800m-instruct-Q4_K_M.gguf` (1.9 GB)
- `S:\rust\rusty-engine\data\models\MiniCPM-V-4_7-1B-Q4_K_M.gguf` (641 MB), `mmproj-model-f16.gguf` (1.1 GB)
- `S:\rust\rusty-engine\data\models\MiniCPM-V-4_7-35B-A3B-Q4_K_M.gguf` (20 GB), `mmproj-model-f16-35b.gguf` (1.1 GB)
- `S:\rust\rusty-engine\data\models\Inkling-2-Mini-UD-Q4_K_M.gguf` (~15.47 GiB), `mmproj-F16.gguf` (610 MB)
- `S:\rust\rusty-engine\data\models\nanojev\` (2.3 GB, safetensors, eval/research)
- `L:\gsv_mirror\` (184 MB, untouched mesh shards/cache)

GSV omni catalog adapted for local hybrid bunke-rock/rusty lanes (see commit `f1a8e1a`): `lama-2.8/granite-a800m`, `lama-1.5/minicpm-v47-1b-local`, `rusty-05/inkling-2-mini`, `rusty-mcp-v47`, `rusty-mesh-v47`, `minicpm-v47-35b`.

## 4. Function management (FM) regrep

### Canonical FM present
| Project | FM artifacts | Status |
|---|---|---|
| **GSV** (mama) | `docs/gsv/GSV_FUNCTION_MANAGEMENT.md`, `docs/gsv/FUNCTION_MANAGEMENT.md` | **present (canonical)** |
| **poolAI** (тато) | `docs/catalog/FUNCTION_MANAGEMENT.md`, `.cursor/rules/functionality-management.mdc` | **present (canonical, per-project)** |

### Registered plugins/products — FM gap (no dedicated FUNCTION_MANAGEMENT doc found)
| Project | Notes / priority |
|---|---|
| `rusty-engine` | High — inference engine (GGUF + nanojev eval), local model serving, symbiosis-critical |
| `llama-rs` | High — bunke-rock backend/serve lane, Pi build context |
| `mesh-node` | High — Pi mesh agent, remote owner-gated but operationally important |
| `GSV-WGPU` | High — browser/mesh lane |
| `telenetis` | High — commutation/Telegram plugin |
| `omniroute` | High — external gateway lane (Node+Rust crates CNV waves) |
| `LinFS` | Medium — FS plugin |
| `rebook` | Medium — EPUB pipeline (book work local-only per policy) |
| `ORR_DESKTOP` | Medium — capture plugin (private) |
| `CoreBoard_Labs_shop` | Medium — private |
| `sport-forcast` | Medium — private |
| `adoreble_wifi` | Medium — Wi-Fi/AP plugin, recently scaffolded |

**Non-registered/outside:** `freebeat.ai` (scaffold), `platinoff-profile` (static) — FM optional/outside per symbiosis canon.

### FM symbiosis actions (planned)
- Keep full FM canon in **GSV + poolAI**; add **FM skeletons** for high-priority plugins (minimum: purpose, key functions/modules, interfaces/ports, dependencies, test/gate status, model/runtime assets where relevant, links to HANDOFF/NEXT).
- Avoid duplicating poolAI FM journal into plugins; reference GSV tickets/FM canon.

## 5. Rules, MCP bot/toolchain compatibility audit

### MCP kit wiring (sampled: GSV, poolAI, telenetis, llama-rs, rusty-engine, mesh-node, omniroute, adoreble_wifi)
- **Cursor:** folder-scoped `.cursor/mcp.json` → `http://127.0.0.1:9999/mcp` (`gsv_mcp_openbot`) ✅ (no User-scope leak pattern observed in sampled plugin configs).
- **OpenCode:** `opencode.json` `mcp.gsv_mcp_openbot` stdio → `S:/rust/GSV/target/live/gsv-mcp.exe --repo-root S:/rust/GSV`, cwd `S:/rust/GSV` ✅; instructions include `S:/rust/GSV/AGENTS.md` (+ local AGENTS/docs) ✅; skills path typically includes `S:/rust/GSV/.agents/skills` (and/or local `.agents/skills`) ✅.
- **Grok:** `.grok/config.toml` present in some projects (not universal in samples); stdio live `gsv-mcp.exe` per GSV canon where used.
- **MCP server/toolchain:** single hub `gsv_mcp_openbot` (GSV live); clients must depend on `target/live/gsv-mcp.exe` being built/copied (`cargo xtask live`).

### Rules/toolchain
- **Rust toolchain:** GSV/poolAI/rusty-engine and most Rust plugins pin `stable-x86_64-pc-windows-gnu` with `rustfmt, clippy` (`rust-toolchain.toml`) ✅ (sampled). `omniroute` root has no `rust-toolchain.toml` (Node-primary repo with `crates/*` Rust components) — verify crate/tooling expectations separately.
- **AGENTS/rules:** all sampled registered projects have `AGENTS.md`; GSV canon is global source of truth. Plugin AGENTS should stay product-specific and reference GSV kit where applicable.
- **Policy:** no PowerShell, no `git add -A`, no secrets in logs/commits, no User-scope MCP, CNV/runtime changes additive/opt-in unless explicitly approved.

### Compatibility risks / follow-ups
1. **Config drift:** some plugins may miss `.grok/config.toml`, local `.agents/skills` links, or updated instruction files — normalize via `cargo xtask mirrors`/kit wiring checks where appropriate.
2. **Live dependency:** MCP stdio clients fail if `S:/rust/GSV/target/live/gsv-mcp.exe` is missing/stale — operational prerequisite (not code drift).
3. **NanoJev vs GGUF routing:** nanojev safetensors is eval/research, not omni GGUF serve; keep explicit in docs/FM to avoid misuse.
4. **Omniroute dual-stack:** Rust crates follow Rust gates; JS/TS remains source-of-truth for runtime per CNV policy — compatibility tracked by CNV waves.

## 6. Symbiosis tuning roadmap (prioritized, ticketed)

### P0 — Canon/plan/tickets (this session)
- [x] NanoJev static verification via rusty-engine (`inspect`, 322 tensors)
- [x] FM/rules/MCP/toolchain regrep and audit summary
- [x] Create this plan
- [x] Create execution tickets (board)

### P1 — Documentation & FM symbiosis
- **SYM-1 (GSV, done):** Record nanojev verification + model-role notes in GSV docs. Evidence: `docs/gsv/LOCAL_MODEL_ROLES_2026-11-21.md`, `docs/NEXT_SESSION_PROMPT.md` updated.
- **SYM-2 (done):** Add FM skeleton docs for high-priority plugins (`rusty-engine`, `llama-rs`, `mesh-node`, `GSV-WGPU`, `telenetis`) — one commit per repo. Evidence: rusty-engine `bf20c64`, llama-rs `2761710`, telenetis `bc81a69` pushed; mesh-node `bafca35`, GSV-WGPU `9c65f59` local (no remote, owner-gate).
- **SYM-3:** Extend FM skeletons to remaining registered plugins (`omniroute`, `LinFS`, `rebook`, `ORR_DESKTOP`, `CoreBoard_Labs_shop`, `sport-forcast`, `adoreble_wifi`) as follow-up.

### P2 — MCP/rules/toolchain normalization
- **SYM-4 (done):** Kit wiring audit/fix across registered projects. Evidence: cursor/opencode/grok/skills refs ok; added rust-toolchain.toml to adoreble_wifi `d1eba73` (pushed) and mesh-node `e00537e` (local, no remote). GSV ticket close `051e5fb`.
- **SYM-5:** Verify rust-toolchain alignment across all Rust repos/crates (including omniroute `crates/*` if required) and document exceptions.
- **SYM-6 (done):** NanoJev operational runbook in `rusty-engine` (`docs/NANOJEV_RUNBOOK.md`, `31b84f6` pushed). Inspect smoke 322 tensors OK; replay `--limit 2` attempted but terminated on constrained box (documented).

### P3 — Functional symbiosis tuning
- **SYM-7:** Validate local hybrid model routing end-to-end when services/heartbeats are available (bunke-rock/rusty-serve/rusty-hybrid/mesh/35b lanes), capture evidence.
- **SYM-8:** Mesh evidence backlog continuation per `docs/superpowers/plans/2026-11-21-ecosystem-next-tasks.md` (B1/B5/B6/D4) when peer online.
- **SYM-9:** CNV-1 W23+ continuation (separate omniroute tickets/waves, opt-in).

## 7. Ticket map (created for this plan)

- `t-1795243252280126900` — **Ecosystem symbiosis: audit nanojev/FM/rules/MCP toolchain and create tuning plan+tickets** (this audit/plan) — **done**
- `t-1795243604178088100` — **SYM-1:** document nanojev verification and local model roles in GSV canon (`gsv`, **done**)
- `t-1795243604268301300` — **SYM-2:** add FM skeletons for high-priority plugins (`rusty-engine,llama-rs,mesh-node,GSV-WGPU,telenetis`) (`gsv`, **done**)
- `t-1795243604357865800` — **SYM-4:** normalize kit wiring (MCP/rules/skills/instructions) across registered projects (`gsv`, **done**)
- `t-1795243604437440700` — **SYM-6:** rusty-engine nanojev runbook and replay smoke verification (`rusty-engine`, **done**)

(Additional tickets SYM-3/SYM-5/SYM-7/SYM-8/SYM-9 to be created/scheduled as dependencies clear; SYM-8/SYM-9 tracked in existing backlog plans.)

## 8. Gates / execution rules

- MSYS2 bash only; Rust-first/toolchain per repo; no PowerShell.
- One meaningful commit per repo, no `git add -A`; push + summary after green gates.
- Do not commit model weights, secrets, `rebook/**` book content, or transient artifacts.
- CNV/runtime changes additive/opt-in; poolAI full tests remain CI-authority.
- If a follow-up requires live peers/services/owner gates, record evidence and keep blocked items explicit.
