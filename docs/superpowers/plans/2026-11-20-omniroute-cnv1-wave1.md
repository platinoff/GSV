# CNV-1 — omniroute TS→Rust (Wave 1–11 progress)

**Status:** Wave 1–11 done (bounded slices) · **Date:** 2026-11-20  
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
| W5-NAPI-CRATE | `crates/omniroute-napi` (cdylib, `napi-rs` v2) exposing Rust primitives | done (`2b83c1b`) |
| W5-BINDING-API | `gcfEncodeJson`, `gcfDecodeToJson`, `validateRulePackJson`, `validateCompressionJson`, `isCompressionExcluded` (JSON/string boundary) | done |
| W5-PKG-SCAFFOLD | Opt-in `crates/omniroute-napi/package.json` (`@napi-rs/cli`), README, local `.gitignore` (not in root workspaces/build) | done |
| W5-GATES | `cargo fmt`, `cargo check/clippy -p omniroute-napi` green; existing crates clippy/tests green | done |
| W5-COMMIT | Conventional commit in omniroute + push | done (`2b83c1b`) |

**W5 implementation:**
- `crates/omniroute-napi/src/lib.rs` — napi exports bridging `omniroute-gcf` and `omniroute-compression` over JSON/string inputs/outputs (additive, no runtime wiring).
- `build.rs`, `Cargo.toml`, `package.json`, `README.md`, `.gitignore` for opt-in local builds.

**Notes:** binding is **not** wired into omniroute npm build/test/CI and not added to root workspaces.

## Wave 6 (Node adapter + opt-in smoke boundary) — done

| ID | Deliverable | Status |
|----|-------------|--------|
| W6-ADAPTER | Opt-in bridge module `open-sse/services/compression/nativeRust.ts` (lazy-load napi addon, candidate path resolution incl. `OMNIROUTE_RUST_NAPI_PATH`, typed wrappers mirroring TS-level result shapes) | done (`8643034`) |
| W6-BOUNDARY | **Not** exported from `compression/index.ts` and not imported by default runtime paths (explicit opt-in import only) | done |
| W6-SMOKE | Opt-in smoke script `scripts/cnv1-smoke-rust-bridge.mjs` (tsx-runnable) covering GCF round-trip, validation, rule-pack validation, exclusions; graceful `exit 2` when addon unavailable | done |
| W6-GATES | Smoke executed in-env via tsx → `RUST_BRIDGE_UNAVAILABLE`/exit 2 as expected (no built addon); no Rust/runtime default-path changes | done |
| W6-COMMIT | Conventional commit in omniroute + push | done (`8643034`) |

**W6 notes:** native addon build/distribution, CI artifact strategy, and any call-site swaps behind feature flags remain follow-ups (opt-in only, no default behavior change).

## Wave 7 (caveman rule-pack data helpers, pure/FS-safe) — done

| ID | Deliverable | Status |
|----|-------------|--------|
| W7-PARSE-VALIDATE | `parse_rule_pack_value` / `parse_rule_pack_str` / `validate_rule_pack_str` (JSON parse + existing structural validation + typed deserialization) in `omniroute-compression::caveman_rules` | done (`6e54136`) |
| W7-SUMMARY-META | `RulePackMetadata` and `summarize_language_packs` (language/categories sorted+dedup/rule_count aggregation), pure in-memory | done |
| W7-SELECTORS | `filter_file_rules_for_context` and `select_file_rule_metadata` for `FileRule` sets (context/intensity, FS-safe) | done |
| W7-GATES | `cargo fmt --all --check`, `cargo clippy -p omniroute-compression -p omniroute-gcf --all-targets` clean, `cargo test -p omniroute-compression -p omniroute-gcf` pass | done |
| W7-COMMIT | Conventional commit in omniroute + push | done (`6e54136`) |

**W7 implementation:** extended `crates/omniroute-compression/src/caveman_rules.rs` + exports in `lib.rs`. **Parity evidence (Rust):** 20 tests green in `omniroute-compression` (+ gcf 16 green). No FS/Node deps, no JS-regex apply/execution, no TS runtime changes.

## Wave 8 (SSE/protocol primitives, additive crate) — done

| ID | Deliverable | Status |
|----|-------------|--------|
| W8-SSE-CRATE | `crates/omniroute-sse` (workspace member) with pure SSE helpers ported from `open-sse/utils/streamHelpers.ts` | done (`1280e69`) |
| W8-PRIMITIVES | `strip_ansi_codes`, `parse_sse_data_payload`, `parse_sse_data_lines`, `extract_sse_data_line`, `parse_sse_line`, `format_sse` (incl. `[DONE]`, event wrapper, Claude event typing, null/empty handling, `usage.perf_metrics` null cleanup) | done |
| W8-GATES | `cargo fmt --all --check`, `cargo clippy -p omniroute-sse -p omniroute-compression -p omniroute-gcf --all-targets` clean, `cargo test -p omniroute-sse -p omniroute-compression -p omniroute-gcf` pass | done |
| W8-COMMIT | Conventional commit in omniroute + push | done (`1280e69`) |

**W8 implementation:** `crates/omniroute-sse/src/lib.rs` + `Cargo.toml`; JSON boundary via `serde_json::Value` (preserve_order). **Parity evidence (Rust):** 6 tests green in `omniroute-sse` (+ compression 20, gcf 16). Additive, not wired into default runtime.

## Wave 9 (opt-in napi + adapter exposure for `omniroute-sse`) — done

| ID | Deliverable | Status |
|----|-------------|--------|
| W9-NAPI | `omniroute-napi` depends on `omniroute-sse` and exports `sseStripAnsiCodes`, `sseParseDataPayload`, `sseParseDataLines`, `sseExtractDataLine`, `sseParseLine`, `sseFormatSse` | done (`2c796dd`) |
| W9-ADAPTER | `open-sse/services/compression/nativeRust.ts` extended with SSE wrappers (opt-in, not in default runtime/index exports) | done |
| W9-SMOKE | `scripts/cnv1-smoke-rust-bridge.mjs` extended with SSE assertions (runs under tsx; graceful exit 2 when addon unavailable) | done |
| W9-GATES | `cargo fmt/check/clippy -p omniroute-napi` + clippy/tests for `omniroute-sse/compression/gcf` green; smoke probe exit 2 as expected (no built addon) | done |
| W9-COMMIT | Conventional commit in omniroute + push | done (`2c796dd`) |

**W9 notes:** binding/addon not built or wired into default runtime; exposure remains opt-in.

## Wave 10 (stateful SSE normalization/prefix primitives) — done

| ID | Deliverable | Status |
|----|-------------|--------|
| W10-NORMALIZER | `SseDataLineNormalizer` in `omniroute-sse` (buffered multiline `data:` merge, self-describing pending flush on control/comment boundaries, blank-line flush, `[DONE]`/JSON normalization semantics) | done (`50fcd3e`) |
| W10-PREFIX-BUFFER | `SseEventPrefixBuffer` (remember/emit-once prefix for `data:` frames, `event_type`, `flush`, `clear`) | done |
| W10-GATES | `cargo fmt --all --check`, `cargo clippy -p omniroute-sse --all-targets` clean, `cargo test -p omniroute-sse` pass | done |
| W10-COMMIT | Conventional commit in omniroute + push | done (`50fcd3e`) |

**W10 implementation:** additive methods/structs in `crates/omniroute-sse/src/lib.rs` with parity-oriented tests. **Parity evidence (Rust):** 9 tests green in `omniroute-sse`. Not wired into runtime/adapters.

## Wave 11 (SSE stream payload predicate helpers) — done

| ID | Deliverable | Status |
|----|-------------|--------|
| W11-PREDICATES | `is_openai_choices_payload`, `has_openai_finish_reason`, `is_known_non_claude_stream_payload` (+ internal value-shape helpers for OpenAI/Responses/Gemini stream detection) in `omniroute-sse` | done (`3add676`) |
| W11-GATES | `cargo fmt --all --check`, `cargo clippy -p omniroute-sse --all-targets` clean, `cargo test -p omniroute-sse` pass | done |
| W11-COMMIT | Conventional commit in omniroute + push | done (`3add676`) |

**W11 implementation:** pure JSON-shape predicates ported from `open-sse/utils/streamHelpers.ts` semantics with Rust tests. **Parity evidence (Rust):** 10 tests green in `omniroute-sse`. Additive, not wired into runtime.

## Proposed next waves (owner-prioritized, in order)

| Wave | Candidate | Rationale | Risk |
|------|-----------|-----------|------|
| W12 | Hot-path/protocol primitives continued (e.g. additional bounded chunk/value helpers or opt-in napi exposure for SSE predicates/stateful helpers) behind opt-in boundaries | deepen core engine leverage safely | M/L |
| W13 | Pure compression/rules core continued as needed (deterministic helpers, regex-semantics-safe transforms, pack/data abstractions) | extend portable core | M |
| W14+ | Services/handlers/routes/domain slices, incremental strangler-fig | broad app impact | L/XL |

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
