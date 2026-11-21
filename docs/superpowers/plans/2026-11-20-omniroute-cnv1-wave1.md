# CNV-1 — omniroute TS→Rust (Wave 1–5 progress)

**Status:** Wave 1–5 done (bounded slices) · **Date:** 2026-11-20  
**Ticket:** CNV-1 (owner: exempt-till-epic → epic started)  
**Repo:** `S:\rust\omniroute` (origin `https://github.com/platinoff/OmniRoute` fork in use, branch `release/v3.8.50`)  
**Scope reality:** 8,263 tracked TS/JS-family files deferred under CNV-1 (Next.js 16 app + `open-sse/` streaming engine + Electron + CLI + tests + scripts). Full migration is a multi-wave epic; waves deliver additive, bounded, testable ports without breaking existing npm/Node runtime.

## Goals

1. Establish Rust build foundation inside omniroute without breaking existing npm/Node flows.
2. Port pure, deterministic modules to Rust with tests as reference migration slices.
3. Document strategy, boundaries, parity evidence, and next waves.

## Non-goals (current waves)

- No mass TS→Rust conversion of app/routes/providers/UI.
- No changes to omniroute runtime behavior, CI semantics, or build outputs beyond additive Rust workspace files (bindings are opt-in, not wired into default build/test).
- No touching `.env`, secrets, `node_modules`, `target/` artifacts, rebook, model weights.

## Strategy

- **Additive Rust workspace:** `crates/` + root `Cargo.toml` workspace (resolver 2). Existing TS/JS remains source of truth for runtime until a port is wired.
- **Pick pure, dependency-light modules first:** deterministic transforms, parsers/encoders, config/validation logic, utilities.
- **Parity via tests:** Rust ports mirror TS semantics and include unit/integration tests (golden/parity cases; later shared vectors with TS).
- **Integration later (incremental):** expose crates via N-API (`napi-rs`) or WASM + adapter, then swap call sites wave-by-wave behind tests.

## Wave 1 (bootstrap + scalar) — done

| ID | Deliverable | Status |
|----|-------------|--------|
| W1-BOOT | Rust workspace bootstrap (`Cargo.toml`, `crates/omniroute-gcf/`, `.gitignore` Rust entries) | done (`8e6a304`) |
| W1-PORT-GCF-SCALAR | Port `gcf/scalar.ts` → `crates/omniroute-gcf` | done |
| W1-GATES/COMMIT | fmt/clippy/test green, commit+push | done (`8e6a304`) |

## Wave 2 (GCF generic encode/decode parity) — done

| ID | Deliverable | Status |
|----|-------------|--------|
| W2-PORT-GENERIC | Port `generic.ts`/`decode_generic.ts` to Rust | done (`fcdfe68`) |
| W2-PARITY/GATES/COMMIT | round-trip parity tests, gates green, commit+push | done (`fcdfe68`) |

**Parity:** 16 tests green in `omniroute-gcf`.

## Wave 3 (compression pure utilities: validators/helpers) — done

| ID | Deliverable | Status |
|----|-------------|--------|
| W3-COMPRESSION-CRATE | `crates/omniroute-compression` | done (`ab297a0`) |
| W3-EXCLUSIONS | Port `exclusions.ts` (normalize/match, wildcard/metachar) | done |
| W3-PRESERVATION-HELPERS | Fenced-code extraction helpers from `preservation.ts` | done |
| W3-VALIDATION | Port `validation.ts` (`validate_compression`, collectors, previews) | done |
| W3-GATES/COMMIT | gates green, commit+push | done (`ab297a0`) |

## Wave 4 (caveman rule pack validation + metadata/selectors) — done

| ID | Deliverable | Status |
|----|-------------|--------|
| W4-RULE-PACK-VALIDATION | Port `ruleLoader.ts` `validateRulePack` (structural + best-effort pattern compile via `fancy-regex`) | done (`a94e478`) |
| W4-RULE-META-SELECTORS | Rule types/metadata, `filter_rules_for_context`, intensity ranking, metadata projection | done |
| W4-GATES/COMMIT | gates green, commit+push | done (`a94e478`) |

**Parity:** 18 tests green in `omniroute-compression` (+ gcf 16 green).

## Wave 5 (N-API bindings foundation, opt-in) — done

| ID | Deliverable | Status |
|----|-------------|--------|
| W5-NAPI-CRATE | `crates/omniroute-napi` (cdylib, `napi-rs` v2) exposing Rust primitives | done |
| W5-BINDING-API | `gcfEncodeJson`, `gcfDecodeToJson`, `validateRulePackJson`, `validateCompressionJson`, `isCompressionExcluded` (JSON/string boundary) | done |
| W5-PKG-SCAFFOLD | Opt-in `crates/omniroute-napi/package.json` (`@napi-rs/cli`), README, local `.gitignore` (not in root workspaces/build) | done |
| W5-GATES | `cargo fmt`, `cargo check/clippy -p omniroute-napi` green; existing crates clippy/tests green | done |
| W5-COMMIT | Conventional commit in omniroute + push | pending (this wave) |

**W5 implementation:**
- `crates/omniroute-napi/src/lib.rs` — napi exports bridging `omniroute-gcf` and `omniroute-compression` over JSON/string inputs/outputs (additive, no runtime wiring).
- `build.rs`, `Cargo.toml`, `package.json`, `README.md`, `.gitignore` for opt-in local builds.

**Notes:** binding is **not** wired into omniroute npm build/test/CI and not added to root workspaces; Node adapter/call-site swaps and CI artifact strategy are follow-ups.

## Proposed next waves (owner-prioritized, in order)

| Wave | Candidate | Rationale | Risk |
|------|-----------|-----------|------|
| W6 | Node adapter + opt-in wiring for napi bindings (internal package/import boundary, smoke tests) without changing default runtime behavior | make W5 consumable safely | M |
| W7 | Pure compression/rules core continued (deterministic helpers, regex-semantics-safe transforms, pack/data abstractions) | extend portable core | M |
| W8 | `open-sse` hot-path components (translator/chunk/protocol primitives) behind adapters | core engine leverage | L |
| W9+ | Services/handlers/routes/domain slices, incremental strangler-fig | broad app impact | L/XL |

> W2 hardening option open: TS-shared golden vectors/fixture harness for GCF.

## Risks / constraints

- omniroute CI is Node/npm-centric; native addon build/distribution (platform binaries, CI matrix, Electron) must be added carefully and kept opt-in until stable.
- JS regex features/semantics differ from Rust engines; pattern validation/execution requires care (`fancy-regex` best-effort where used).
- License/provenance: preserve upstream notices for vendored logic ports (GCF MIT noted in TS originals).
- Keep omniroute `.env`/keys out of commits. npm/npx hooks may be skipped in constrained envs; TS/JS untouched in these waves.

## Gates

- Rust: `cargo fmt --all -- --check`, `cargo check/clippy -p omniroute-napi`, `cargo clippy -p omniroute-compression -p omniroute-gcf --all-targets`, `cargo test -p omniroute-compression -p omniroute-gcf`.
- No changes to tracked TS runtime behavior/default build in these waves.
- Commit: one conventional commit per wave/slice in omniroute, push with summary. Plan doc maintained in GSV.
