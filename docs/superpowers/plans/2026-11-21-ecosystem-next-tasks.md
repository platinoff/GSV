> **SUPERSEDED 2026-11-27:** rows N12, N15 and O7 below are reconciled by
> `docs/superpowers/plans/2026-11-27-ticket-board-logic-reconciliation.md`
> and `docs/gsv/reports/TICKET_BOARD_LOGIC_RECONCILIATION_2026-11-27.md`.
> The AGI scaffold rows `t-17931031167*` are procedural, not product backlog.

# Ecosystem next tasks — logical execution order (2026-11-21)

**Source review:** `docs/HANDOFF_NEW_SESSION.md`, `docs/NEXT_SESSION_PROMPT.md`, `docs/gsv/MESH_ROADMAP_TASKS_2026-11-19.md`, `docs/gsv/ECOSYSTEM_2026_11_18_PLAN.md`, `docs/gsv/FORMAT_INVENTORY_2026-11-20.md`, `docs/superpowers/plans/2026-11-20-format-hygiene-tickets.md`, `docs/superpowers/plans/2026-11-20-omniroute-cnv1-wave1.md`, `docs/gsv/tickets.jsonl` (open set), `docs/gsv/SESSION_PRESET.md`, `docs/gsv/ECOSYSTEM_CI_REGISTRY_2026-11-18.md`, `docs/superpowers/plans/2026-11-16-next-priority-queue.md` (historical, reconciled).

**Rules:** Rust-first, MSYS2 bash only, no `git add -A`, one meaningful commit per repo, push+summary after each, no secrets/`rebook/**`/weights/`.env`, poolAI full tests CI-authority (local `fmt+clippy+check`), CNV-1 additive/opt-in (no default runtime wiring). Do not kill `target/live/gsv-server.exe` before tests/build in GSV.

**Legend:** `GATE` = must-pass checks; `OWNER` = explicit owner decision/go required; `BLOCKED` = cannot proceed now.

## Snapshot (facts)

- **Hygiene:** actionable **0 / 0** (tracked/all) per recheck 2026-11-20; remaining non-allowed files are exempt/deferred only. mesh-node local commits ready, **no remote** (owner-gate).
- **CNV-1 (omniroute, `origin/release/v3.8.50`):** W1–W21 done (`c9bceeb`), plan in GSV `15a84a0`. Next proposed: W22 (bounded protocol/primitives/adapter ergonomics; optionally expose W21 via napi/wasm if useful), then W23.
- **Mesh (hub + bifrost #38/#39 context evolved):** A1–A4, B4, B7 (live telemetry path), C1–C6, D1/D2/D3/D5/D6/D7 done per HANDOFF/tasks doc; B1/B2/B3(client-fix flank)/B5/B6, D1-periodic/D2-periodic/D4, A5 remain relevant/open in roadmap framing.
- **Open tickets (board, selected actionable/blocked):** ECO-8.2 (`llama-rs`, open), ECO-8.4 (`poolai`, owner-gated), ECO-7.7 (`rebook`, owner-gated), WSP-G1 (`gsv`, open), WSP-3/MC-1/MC-5/PH-1202..1206 (`rusty-engine`/`poolai`, open, dependency-heavy), P6.2 (`poolai`, open, CI/node only), telenetis token/group/webhook split issues (owner/security-gated). AGI scaffold tickets `t-17931031167*` are process/session tickets (treat as procedural, not product backlog).
- **Critical environment note:** hub/RAM instability historically P0 in ecosystem plan; treat RAM/free-disk (`gsv_disk`) as preflight before heavy gates.

## Prioritized backlog

### P0 — Active continuity, unblocked (do first)

| ID | Task | Repo/Product | Why now | Depends on | Gate | Owner |
|---:|---|---|---|---|---|---|
| N1 | **CNV-1 Wave 22 (omniroute):** bounded protocol/primitives continued (chunk/value utilities, adapter ergonomics, e.g. reasoning-field helpers) behind opt-in boundaries; **optionally** expose W21 lite-body helpers via napi/wasm + adapter/smoke if parity/useful. No default runtime wiring. | `omniroute` | Direct continuation of active epic (W21 done, plan §“Proposed next waves”); highest momentum, additive/low-blast if bounded. | — | `cargo fmt --all --check`; clippy clean for touched crates (`omniroute-compression/gcf/sse` + `napi`/`wasm` if touched); `cargo test -p omniroute-compression -p omniroute-sse -p omniroute-gcf`; `cargo build -p omniroute-wasm --target wasm32-unknown-unknown --release` if wasm touched; napi addon build/smoke only if env supports (probe exit 2 acceptable when addon absent). One commit + push. | no |
| N2 | **Sync CNV-1 plan in GSV after N1:** update `docs/superpowers/plans/2026-11-20-omniroute-cnv1-wave1.md` (W22 status/evidence, next = W23). | `GSV` | Keep plan/canon truthful per wave. | N1 | doc consistency; `cargo xtask sync` only if required by changed canon scope; commit + push. | no |
| N3 | **Mesh A5 — sync `docs/NEXT_SESSION_PROMPT.md` with reality:** reflect Pi dead, current peer reality/hot path (AP 2.4 GHz), mesh roadmap done items (A1–A4,B4,B7,C1–C6,D1–D3,D5–D7), and point to `SESSION_PRESET.md`/`MESH_ROADMAP_TASKS_2026-11-19.md`/failover+PWA docs as operator canon. | `GSV` | Removes operator drift; explicitly open in mesh roadmap (A5). Small, no code risk. | — | factual review vs HANDOFF/roadmap; commit + push. | no |

### P1 — Unblocked, evidence-gathering / small hardening (enables next decisions)

| ID | Task | Repo/Product | Why now | Depends on | Gate | Owner |
|---:|---|---|---|---|---|---|
| N4 | **Mesh B1 — 30-min passive reconnect log capture:** sample `/api/wgpu/console` + `/api/sys/metrics` (age/proofs/battery/alive/ready) on live hub, produce trigger/break/restore table for bifrost reconnect-thrash symptoms. | `GSV` (ops artifact/doc note, e.g. append evidence in roadmap report or short note under `docs/gsv/reports/`) | Prerequisite evidence for B2/B3(client) tuning; roadmap key pain. | live hub + peer present | evidence recorded (timestamps/counts/gaps), no hub overload; commit doc if added. | no (requires peer online window) |
| N5 | **Mesh B5 — auto re-register check:** verify `device_key` recovery and stable peer identity after page reload/reconnect (no unintended peer-id migration) using live telemetry/mesh state. | `GSV` (verify + doc/test note) | Cheap correctness check; listed B5. | live hub + peer | observed before/after evidence; if code fix needed → `cargo fmt && cargo clippy --all-targets && cargo test` (no kill `target/live`). | no |
| N6 | **Mesh B6 — IndexedDB cache hit-rate check:** inspect/estimate chunk-cache hit behavior after soak (R3 persistent cache); document hit/miss/eviction signals available. Feeds D4. | `GSV` / ops doc | Validates R3 benefit before raising limits. | peer activity + telemetry access | evidence note; no storage writes to peers. | no |
| N7 | **Mesh D4 — IndexedDB eviction audit:** when cache >128 MiB LRU, confirm eviction behavior/count visibility; if high churn, write docs note/recommendation (do not change peer disk). | `GSV` (docs; code only if hub-side observability gap and bounded) | Closes D4; complements B6. | N6 (preferred) | doc committed; any code change gated by fmt/clippy/test. | no |

### P2 — Bounded follow-ups (after evidence or when convenient)

| ID | Task | Repo/Product | Why now | Depends on | Gate | Owner |
|---:|---|---|---|---|---|---|
| N8 | **CNV-1 Wave 23 (omniroute):** portable compression-core extensions where dependency-light (e.g. stats/token-estimation or diff/heatmap helpers) with parity tests; keep additive/opt-in. | `omniroute` | Planned after W22; continues epic without default wiring. | N1 | fmt/clippy/tests (+ wasm/napi if exposed) per wave pattern; commit+push; GSV plan update. | no |
| N9 | **Mesh B2 — push vs poll A/B assessment:** verify WS reconnect/backoff behavior and whether 5s SSE/poll fallback helps during push breaks; implement only minimal fix if evidence warrants. | `GSV` | Reduces thrash if B1 shows transport instability. | N4 | repro/evidence; if code: `cargo fmt`, `cargo clippy --all-targets`, `cargo test`; no `target/live` kill. | no |
| N10 | **Mesh B3 (client flank, optional):** reconnect debounce/backoff tuning in `/wgpu` PWA client (e.g. 5s → 20s debounce x2..4) only if B1/B2 evidence supports and UX acceptable. | `GSV` | Listed as 2nd flank in roadmap/keepalive doc. | N4, N9 | targeted UI/client verification; fmt/clippy/test as applicable; **get owner go before behavior-changing client reconnect policy**. | **owner go recommended** |
| N11 | **Mesh C7 — stand-smoke in CI:** add/adjust GitHub workflow to run `gsv-http-stand-smoke` against temporary hub if CI can bind `:9999`; otherwise document skip condition. | `GSV` | Closes roadmap C7; improves regression signal. | — | workflow lint/review; CI green on PR/push path. | no (may need repo CI constraints check) |

### P3 — Cross-ecosystem / heavier (separate sessions, respect gates)

| ID | Task | Repo/Product | Why | Depends on | Gate | Owner |
|---:|---|---|---|---|---|---|
| N12 | **ECO-8.2 — llama-rs `llama-cpp-sys-2` lockstep bump band:** bump sys/llama-cpp-2 in lockstep, re-verify patches/bindings, rebuild, gates + heartbeat probe. | `llama-rs` | Open board ticket; Pi build unblocked noted in HANDOFF but bump itself still pending/careful. | clean scope, no parallel cargo | repo gates per ticket; one commit. Do not mix with unrelated tracks. | confirm scope/timing |
| N13 | **ECO-8.3 dep-sweep (remaining plugins), one product per session:** LinFS, ORR, rusty-engine, sport-forcast, CoreBoard etc., minimal patch-level updates. | per plugin | Board/open plan item; reduces drift incrementally. | per-repo clean state | per-repo fmt/clippy/tests/check as applicable; one commit per repo. | no |
| N14 | **poolAI P6.2 tests rewrite (module-by-module):** execute only on CI/separate runner (orchestrator→raid→discovery→ui-admin) per `2026-11-18-poolai-p6-2-rewrite-plan.md`. | `poolai` | Open ticket; large but planned. | CI/node capacity | `cargo test-ci` green in poolAI CI, FM row updates, ratio/stretch gates per plan. **Do not run full suite on this box.** | CI/runner |
| N15 | **WSP-G1 / MC-* / PH-120x tracks (rusty-engine/gsv/poolAI):** advance only when prerequisites and compute/RAM/peer conditions are satisfied (envelope bridge, real-weight dispatch, hybrid split, vision/MoE milestones). | multi | Open tickets exist but dependency-heavy and resource-sensitive. | per ticket deps (MC-2/WSP-G1 chain, dedicated node/RAM for 35B where stated) | per-ticket technical gates + evidence; no weight patching without owner go where required. | often owner/go+env |

## Owner-gated / blocked (track, do not execute implicitly)

| ID | Item | Status | Notes |
|---:|---|---|---|
| O1 | **mesh-node remote + push** | BLOCKED on owner | local `e4d2793`,`bae1008` ready; add remote then `git push -u origin main`. |
| O2 | **GEN-4 / CLN-4 (mesh-node scripts/images)** | owner decision | keep/archive/delete per format-hygiene plan. |
| O3 | **CLN-2 (llama-rs `.cmake/`)** | owner/verify | only remove if build.rs/CI unaffected. |
| O4 | **CNV-1 epic scope beyond W23 / runtime wiring** | owner-prioritized | strangler-fig deeper waves (W24+) and any TS-runtime wiring decisions. |
| O5 | **ECO-8.4 poolAI coordinator `:8091` keep-live** | owner-gated | infra mutation; no off-box exposure. |
| O6 | **ECO-7.7 rebook workspace/books scripts tracking** | owner-gated | hygiene decision pending. |
| O7 | **telenetis bot token rotate / group reply ownership / webhook-vs-poller split** | owner/security-gated | open tickets; do not deleteWebhook without owner. |
| O8 | **WSP Phase 2, Granite serve retry, NanoJev deploy** | owner-gated | per HANDOFF/research constraints. |
| O9 | **E2 poolAI CI rerun/dispatch**, **F1/F4 adoreble** | owner-gated | as listed in mesh/infra plans. |
| O10 | **RAM/EDR stabilization for hub-heavy work** | environment/owner action | recurring P0 risk in ecosystem history; preflight with `gsv_disk`/health and avoid parallel cargo storms. |

## Explicitly excluded (do not schedule)

- Pi SD recovery/reflash, disk writes to peers, ADB/system setting changes without per-action owner go.
- k3s/k8s/Docker on Pi (Pi dead; policy), 35B full forward on this hub box, tunnels/User-scope MCP, `:8091` bring-up without owner go.
- Rebook book-content/RB ticket claims from GSV free lane.

## Suggested execution sequence (next 1–3 sessions)

1. **Session A (code, additive):** N1 → push omniroute → N2 → push GSV. *(CNV-1 continuity)*
2. **Session B (ops/docs, when peer available):** N3 (can be done anytime) + N4 → N5 → N6 → N7 as evidence allows. *(mesh stabilization evidence)*
3. **Session C:** N8 or N9/N11 depending on evidence and appetite; pick **one** cross-ecosystem item from P3 only with clean scope. *(bounded follow-up)*

## Maintenance

- After each completed item: update this plan's status markers or the source roadmap/plan doc, and keep HANDOFF/NEXT in sync when stable state changes (not every micro-slice).
- Create/claim formal board tickets only when entering a product drain band where ticket workflow is expected; avoid duplicating existing open tickets.