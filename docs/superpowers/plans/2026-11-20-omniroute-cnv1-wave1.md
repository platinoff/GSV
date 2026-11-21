# CNV-1 — omniroute TS→Rust (Wave 1–4 progress)

**Status:** Wave 1–4 done (bounded slices) · **Date:** 2026-11-20  
**Ticket:** CNV-1 (owner: exempt-till-epic → epic started)  
**Repo:** `S:\rust\omniroute` (origin `https://github.com/platinoff/OmniRoute` fork in use, branch `release/v3.8.50`)  
**Scope reality:** 8,263 tracked TS/JS-family files deferred under CNV-1 (Next.js 16 app + `open-sse/` streaming engine + Electron + CLI + tests + scripts). Full migration is a multi-wave epic; waves deliver additive, bounded, testable ports without breaking existing npm/Node runtime.

## Goals

1. Establish Rust build foundation inside omniroute without breaking existing npm/Node flows.
2. Port pure, deterministic modules to Rust with tests as reference migration slices.
3. Document strategy, boundaries, parity evidence, and next waves.

## Non-goals (current waves)

- No mass TS→Rust conversion of app/routes/providers/UI.
- No N-API/WASM/FFI integration into the running Node app yet (follow-up wave).
- No changes to omniroute runtime behavior, CI semantics, or build outputs beyond additive Rust workspace files.
- No touching `.env`, secrets, `node_modules`, `target/` artifacts, rebook, model weights.

## Strategy

- **Additive Rust workspace:** `crates/` + root `Cargo.toml` workspace (resolver 2). Existing TS/JS remains source of truth for runtime until a port is wired.
- **Pick pure, dependency-light modules first:** deterministic transforms, parsers/encoders, config/validation logic, utilities. Avoid DB/network/UI/framework-coupled code early.
- **Parity via tests:** Rust ports mirror TS semantics and include unit/integration tests (golden/parity cases; later shared vectors with TS).
- **Integration later:** after parity crates stabilize, expose via N-API (`napi-rs`) or WASM or sidecar/CLI, then swap call sites wave-by-wave behind tests.

## Wave 1 (bootstrap + scalar) — done

| ID | Deliverable | Status |
|----|-------------|--------|
| W1-PLAN | This plan (scope, waves, risks) | done |
| W1-BOOT | Rust workspace bootstrap in omniroute (`Cargo.toml`, `crates/omniroute-gcf/`, `.gitignore` Rust entries) | done (`8e6a304`) |
| W1-PORT-GCF-SCALAR | Port `open-sse/services/compression/engines/headroom/gcf/scalar.ts` → `crates/omniroute-gcf` | done (`8e6a304`) |
| W1-GATES | `cargo fmt`/`clippy`/`cargo test -p omniroute-gcf` pass for crate | done |
| W1-COMMIT | Commit in omniroute + push to `origin/release/v3.8.50` | done (`8e6a304`) |

**W1 scope:** `scalar` grammar (quoting/escaping, scalar format/parse, key formatting, delimiter-aware split, field decl parsing, tabular sentinels), 9 unit tests.

## Wave 2 (GCF generic encode/decode parity) — done

| ID | Deliverable | Status |
|----|-------------|--------|
| W2-PORT-GENERIC | Port `generic.ts` (`encode_generic`) and `decode_generic.ts` (`decode_generic`) to Rust | done (`fcdfe68`) |
| W2-PARITY-TESTS | Round-trip/parity integration tests incl. B-GCF-QUOTE inline-array quoting cases | done |
| W2-GATES | `cargo fmt --all --check`, `cargo clippy -p omniroute-gcf --all-targets` clean, `cargo test -p omniroute-gcf` pass | done |
| W2-COMMIT | Conventional commit in omniroute + push | done (`fcdfe68`) |

**W2 implementation:** `crates/omniroute-gcf/src/generic.rs`, `decode_generic.rs`, `tests/generic.rs`. **Parity:** 16 tests green, clippy clean.

## Wave 3 (compression pure utilities: validators/helpers) — done

| ID | Deliverable | Status |
|----|-------------|--------|
| W3-COMPRESSION-CRATE | `crates/omniroute-compression` (additive workspace member) | done (`ab297a0`) |
| W3-EXCLUSIONS | Port `open-sse/services/compression/exclusions.ts` (normalize/match, wildcard `*`, metachar escaping) | done |
| W3-PRESERVATION-HELPERS | Port fenced-code extraction helpers from `preservation.ts` (`find/extract_fenced_code_blocks`) | done |
| W3-VALIDATION | Port `open-sse/services/compression/validation.ts` (`validate_compression`, collectors, preview formatting) | done |
| W3-GATES | fmt/clippy/tests green for `omniroute-compression` + `omniroute-gcf` | done |
| W3-COMMIT | Conventional commit in omniroute + push | done (`ab297a0`) |

**W3 implementation:** `exclusions.rs`, `preservation.rs`, `validation.rs`. **Parity:** 13 tests green in `omniroute-compression`.

## Wave 4 (caveman rule pack validation + metadata/selectors) — done

| ID | Deliverable | Status |
|----|-------------|--------|
| W4-RULE-PACK-VALIDATION | Port `ruleLoader.ts` `validateRulePack` (structural checks + best-effort pattern compilation) | done |
| W4-RULE-META-SELECTORS | Rule types/metadata, `filter_rules_for_context` / intensity ranking, metadata projection (`get_caveman_rule_metadata`, `file_rules_to_meta`) | done |
| W4-GATES | `cargo fmt --all --check`, `cargo clippy -p omniroute-compression -p omniroute-gcf --all-targets` clean, `cargo test -p omniroute-compression -p omniroute-gcf` pass | done |
| W4-COMMIT | Conventional commit in omniroute + push | pending (this wave) |

**W4 implementation:**
- `crates/omniroute-compression/src/caveman_rules.rs` — `validate_rule_pack` (object/fields/enums/flags/replacementMap checks mirroring TS; pattern compile check via `fancy-regex` best-effort with `i/m` mapping to support real lookaround packs), rule pack/file-rule/metadata types, `intensity_rank`, `filter_rules_for_context`, `file_rules_to_meta`, `get_caveman_rule_metadata`.
- `Cargo.toml` deps: `serde`, `serde_json`, `fancy-regex`.

**Parity evidence (Rust):** 18 tests green in `omniroute-compression` (incl. structural validation, flags/replacementMap, lookaround pack pattern, selector/metadata/defaults) + gcf 16 still green.

**Not in this slice (future):** FS rule-pack loading/caching/path resolution, JS-regex rule execution/apply engine and full built-in rule transform table, caveman pipeline integration, preservation sentinel/random-seed extraction, stats/language detection.

## Proposed next waves (owner-prioritized, in order)

| Wave | Candidate | Rationale | Risk |
|------|-----------|-----------|------|
| W5 | N-API or WASM bindings for selected crate(s) + Node adapter | integration boundary (expose Rust primitives to omniroute runtime) | M/L |
| W6 | Pure compression helpers continued (e.g. `cavemanRules` metadata parity from JSON packs at build/load boundary, deterministic text helpers where regex-semantics-safe) and/or `ruleLoader` FS-safe abstractions | extend pure core before deeper coupling | M |
| W7 | `open-sse` hot-path components (translator/chunk/protocol primitives) behind adapters | core engine leverage | L |
| W8+ | Services/handlers/routes/domain slices, incremental strangler-fig | broad app impact | L/XL |

> W2 hardening option still open: TS-shared golden vectors/fixture harness for GCF.

## Risks / constraints

- omniroute CI is Node/npm-centric (many workflows); Rust jobs/toolchain may need to be added in a later wave (not changing existing workflows except additive files).
- Some TS modules depend on Node/browser APIs; not all are portable to pure Rust lib. JS regex features (lookarounds/backrefs/Unicode semantics) require care (`regex` vs `fancy-regex` vs redesign); pattern-compile validation is best-effort across engines.
- License/provenance: preserve upstream notices for vendored logic ports (GCF MIT noted in TS originals).
- Keep omniroute `.env`/keys out of commits (SEC rules still apply).
- npm/npx hooks may be skipped in constrained envs; TS/JS untouched in these waves, Rust gates authoritative for changed Rust code.

## Gates

- Rust: `cargo fmt --all -- --check`, `cargo clippy -p omniroute-compression -p omniroute-gcf --all-targets` (clean), `cargo test -p omniroute-compression -p omniroute-gcf` in omniroute workspace.
- No changes to tracked TS runtime behavior in these waves.
- Commit: one conventional commit per wave/slice in omniroute, push with summary. Plan doc maintained in GSV.
