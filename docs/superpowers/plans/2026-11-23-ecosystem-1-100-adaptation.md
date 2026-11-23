# Ecosystem 1.100 Standardization & Docs/FM Adaptation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Adapt GSV docs/plans/FM to the 2026-11-23 steady state and add a single Rust-native gate that verifies Rust 1.100 pins, plugin wiring, FM/ticket links and vision sync across the ecosystem.

**Architecture:** Research spec travels with the plan (`docs/gsv/research/ECOSYSTEM_1_100_STEADY_STATE_2026-11-23.md`). Docs are updated in-place (FM, roadmap, NEXT) with facts + test references. Automation is added as a read-only `cargo xtask eco-check` that composes existing xtask surfaces (`products --state`, `rules-check`, `fm-xlink --strict`, `mirrors --check`, `sync --check`) and a new pure toolchain-pin scanner; F1-only, no mutation of other repos.

**Tech Stack:** Rust stable 1.100.0 (`stable-x86_64-pc-windows-gnu` / host-agnostic `stable`), `cargo xtask`, MSYS2 bash, GSV MCP ticket tools, serde_json, std fs.

**Spec:** `docs/gsv/research/ECOSYSTEM_1_100_STEADY_STATE_2026-11-23.md`

## Global Constraints

- Rust-first; MSYS2 bash only; no Python/PowerShell/Perl for project work.
- Allowed formats `.rs/.md/.mdc/.json/.js/.wasm`; no weights/secrets in git; no `git add -A`.
- One meaningful commit per repo; push + summary; `git fetch` before drain.
- Global rule file is only `S:/rust/GSV/AGENTS.md`; plugin rules stay in plugin trees.
- Do not kill `target/live/gsv-server.exe` before tests/builds; S0 disk guard first.
- poolAI = dad/FM plan, GSV = mom/ops hub; GASV artifacts are versioned companions, canonical F32 unchanged.
- Registered plugins: gsv, poolai, linfs, telenetis, llama-rs, rebook, orr_desktop, omniroute (policy-down), rusty-engine, gsv-wgpu, sport-forcast, coreboard_labs_shop, adoreble_wifi, mesh-node (14). Host = GSV.
- `RUSTUP_TOOLCHAIN` must stay unset; toolchain pins float with stable 1.100.0.

---

## File Structure

| File | Responsibility |
|---|---|
| `docs/gsv/research/ECOSYSTEM_1_100_STEADY_STATE_2026-11-23.md` | Research spec (facts + citations) — already created |
| `docs/gsv/FUNCTION_MANAGEMENT.md` | GSV function registry: new rows 43–46, refreshed gates/HEAD/owner queue |
| `docs/gsv/GSV_TECH_ROADMAP.md` | Add band addendum for GASV G15–G24 + 1.100 standardization close |
| `docs/gsv/NEXT_SESSION_PROMPT.md` / `docs/gsv/NEXT_GASV_SESSION_2026-11-22.md` | Steady-state handoff refresh |
| `src/boxes/eco_check.rs` | New pure read-only scanner: toolchain pins + plugin clean/dirty + wiring + FM rows + vision rev |
| `src/boxes/mod.rs` | Register `pub mod eco_check;` |
| `src/bin/gsv_xtask.rs` | Add `eco-check` CLI arm |
| `tests/gsv_eco_check_contracts.rs` | Contract tests for scanner (fixture-based, no live env dependency) |

---

## Task 1: Refresh GSV FUNCTION_MANAGEMENT.md to steady state

**Files:**
- Modify: `docs/gsv/FUNCTION_MANAGEMENT.md:1-92`

**Interfaces:**
- Consumes: spec §1–§5 (toolchain, products, GASV delta, automation, devices).
- Produces: FM rows 43–46 referenced by Ticket G27 and Task 3's `fm-xlink` gate.

- [ ] **Step 1: Update header + §1 rows 43–46**
  - Header: `Status: Canon, adapted 2026-11-23 @ HEAD 42b74c6 (rate 1.100.0)`.
  - Add rows for: artifact-backed WGPU weight lane (`boxes/wgpu.rs`, `boxes/wgpu_dequant.rs`, `boxes/weight_store.rs`, test `tests/gsv_wgpu_contracts.rs` + 7 dequant unit tests); Q4_K/Q6_K streaming dequant + golden (same files); durable peer identity `wgpu_peer_ids.json` (`boxes/wgpu.rs`, test `peer_rejoin_after_prune_keeps_durable_id`); versioned 4-bit MoE companion (`docs/gsv/research/GASV_G20_ARTIFACT_WEIGHT_LANE_2026-11-22.md`, mirror publish.json).
- [ ] **Step 2: Refresh §2 gates**
  - Replace `567s`/`99.61%`/HEAD `28753cf` with: `record-rust` warnings=0 errors=0 @ `42b74c6`; `record-speed --skip-run` ok (last full 518s); `sync` rev 516 next PH-S3129; clippy `-D warnings` green; 65 wgpu + 7 dequant tests green.
- [ ] **Step 3: Refresh §4 owner queue**
  - Keep RAM/EDR, vision rev mirror, P4 federation; add G13 (runnable full LLM — owner runtime/RAM + infer/quality), gsv-wgpu & mesh-node remote creation (no remote), omniroute policy-down unchanged.
- [ ] **Step 4: Verify + commit**
  - Run: `cd /s/rust/GSV && git diff --check docs/gsv/FUNCTION_MANAGEMENT.md`
  - Run: `cargo xtask fm-xlink` (advisory — must not show new rows as orphans).
  - Commit: `docs: adapt GSV FUNCTION_MANAGEMENT to 2026-11-23 steady state`.

## Task 2: Add band addendum to GSV_TECH_ROADMAP.md

**Files:**
- Modify: `docs/gsv/GSV_TECH_ROADMAP.md` (append addendum at end, do not rewrite 2347 lines)

**Interfaces:**
- Consumes: Task 1 FM rows, spec §2–§5.
- Produces: roadmap entry for the 4-bit MoE track and the 1.100 standardization band.

- [ ] **Step 1: Append addendum section**
  - Heading: `## Addendum 2026-11-23 — GASV G15–G24 + 1.100 standardization`.
  - Table with: G15 fitness, G16 placement, G17 audit verdict, G18 vision verdict, G20 lane, G21 Q4_K canary, G22 huge Q4_K 8/8, G23 Q6_K 8/8, G24 durable peer ID, kit gates (rev 516).
  - Next: PH-S3129; G13 blocked; G19 not started.
- [ ] **Step 2: Verify + commit**
  - Run: `cargo xtask sync` (vision snapshot must stay rev 516 or advance cleanly).
  - Commit: `docs: roadmap addendum for 4-bit MoE + 1.100 band`.

## Task 3: Implement `eco-check` scanner (pure, read-only)

**Files:**
- Create: `src/boxes/eco_check.rs`
- Modify: `src/boxes/mod.rs` (add `pub mod eco_check;`)
- Test: `tests/gsv_eco_check_contracts.rs`

**Interfaces:**
- Consumes: `boxes/products.rs` discover/state, std fs, serde_json.
- Produces: `pub fn report(root: &Path) -> Value` with fields `toolchain` (per product id: `pin`, `ok_1_100`), `products` (count/registered/clean), `fm` (row_count, last_id), `vision` (rev, next), `gates` (record_rust/rules/fm_xlink advisory), `ok: bool`.

- [ ] **Step 1: Write failing contract test** in `tests/gsv_eco_check_contracts.rs` with a temp fixture root containing a fake `rust-toolchain.toml` with `channel = "stable-x86_64-pc-windows-gnu"` and a fake `docs/gsv/FUNCTION_MANAGEMENT.md`; assert `report()["toolchain"]["ok_1_100"] == true` and `["fm"]["row_count"] >= 40`.
- [ ] **Step 2: Run it to verify it fails**
  - Run: `cargo test --test gsv_eco_check_contracts`
  - Expected: FAIL — module/file not found.
- [ ] **Step 3: Write minimal implementation**
  - `eco_check::report(root)`: scan root parent for product dirs (`Cargo.toml`), read `rust-toolchain.toml` channel string; accept `stable` or `stable-x86_64-pc-windows-gnu`; count registered via `boxes/products::discover`; parse FM rows with a regex-free line counter (`lines().filter(|l| l.starts_with("| "))` minus header/separator); read `docs/vision/manifest.json` rev if present; never mutate.
- [ ] **Step 4: Run test to verify PASS**
  - Run: `cargo test --test gsv_eco_check_contracts`
  - Expected: PASS.
- [ ] **Step 5: Run clippy + commit**
  - Run: `cargo clippy --all-targets -- -D warnings`
  - Commit: `feat: add read-only ecosystem 1.100 eco-check scanner`.

## Task 4: Wire `eco-check` into `cargo xtask`

**Files:**
- Modify: `src/bin/gsv_xtask.rs` (CLI arm), `src/boxes/sli.rs` (catalog entry via existing pattern)

**Interfaces:**
- Consumes: Task 3 `eco_check::report(&Path)`.
- Produces: `cargo xtask eco-check [--json]` exit 0 (all green) / 1 (drift).

- [ ] **Step 1: Add CLI arm**
  - Match `"eco-check"` → call `xtask::run_eco_check(&root, json_flag)`; print TSV default, JSON with `--json`.
- [ ] **Step 2: Add SLI catalog entry** with `name: "eco-check"`, `kind: "xtask"`, `path: "src/boxes/xtask.rs"`, `example: "cargo xtask eco-check"`, description reflecting read-only verification.
- [ ] **Step 3: Verify**
  - Run: `cargo xtask eco-check --json | head -c 500`
  - Expected: JSON with `ok: true`, 14 registered products, `toolchain.ok_1_100: true`.
  - Run: `cargo test --lib boxes::sli` (catalog shape contract green).
- [ ] **Step 4: Commit** `feat: wire eco-check into xtask + SLI catalog`.

## Task 5: Ticket creation + close per GSV rules

**Files:**
- Modify: `docs/gsv/tickets.jsonl` via GSV MCP only (`gsv_tickets_create` / `claim` / `done`).

**Interfaces:**
- Consumes: Tasks 1–4 evidence.
- Produces: tickets G25 (FM/roadmap adaptation), G26 (eco-check), G27 (automation wiring) — created, claimed, done with evidence.

- [ ] **Step 1: Create tickets via `gsv_tickets_create`** (product `gsv`, titles as above, bodies cite spec path + Task evidence).
- [ ] **Step 2: Claim + done each** with `evidence` = exact commands + green output (FM diff check, sync rev, eco-check JSON, clippy/test lines).
- [ ] **Step 3: Verify board** via `gsv_tickets` — G25–G27 status `done`, no orphan `fm_ref`.
- [ ] **Step 4: Commit tickets.jsonl** `tickets: G25-G27 close (ecosystem 1.100 adaptation)`.

## Task 6: Kit close-out

**Files:**
- Modify: `docs/vision/rust_diagnostics.json` (via `record-rust`), `docs/vision/speed_index.json` (via `record-speed --skip-run`), vision snapshots (via `sync`).

**Interfaces:**
- Consumes: Tasks 1–5 all green.
- Produces: stable vision rev + next PH-S* + fingerprint.

- [ ] **Step 1: Run gates**
  - `cargo fmt --all -- --check`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo test --lib`
  - `cargo xtask record-rust && cargo xtask record-speed --skip-run && cargo xtask sync`
  - `cargo xtask rules-check` (must be green or documented advisory)
- [ ] **Step 2: Fingerprint + commit + push**
  - `cargo xtask fingerprint --model <current> --band N` (N = next band from sync)
  - One GSV commit with all docs/code, trailers `Gsv-Actor`/`Gsv-Ide`/`Gsv-Model`
  - `git push origin main` + summary in chat.

## Exit Criteria

- FM/roadmap/NEXT reflect 2026-11-23 steady state with test references.
- `cargo xtask eco-check --json` returns `ok:true` for the registered ecosystem.
- Tickets G25–G27 done via GSV MCP with command evidence.
- Kit gates green; one commit + push; no plugin repo modified in this plan.
- G13 remains blocked; G19 not started; canonical F32 unchanged.

## Self-Review Notes

- Spec §1 toolchain → Task 3/4 verify; §2 products → Task 3 scanner; §3 FM gap → Task 1; §4 automation gap → Task 3/4; §5 devices → Task 1/2 docs; §6 constraints → Global Constraints.
- No placeholders: every task has exact file, command, expected output, commit message.
- Type consistency: `eco_check::report(root: &Path) -> Value`; fields `toolchain`, `products`, `fm`, `vision`, `gates`, `ok` used identically in Tasks 3/4/5.
