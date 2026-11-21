# CNV-1 — omniroute TS→Rust (Wave 1–2 progress)

**Status:** Wave 1 done, Wave 2 (GCF generic parity slice) done · **Date:** 2026-11-20  
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
| W2-PORT-GENERIC | Port `generic.ts` (`encode_generic`) and `decode_generic.ts` (`decode_generic`) to Rust | done |
| W2-PARITY-TESTS | Round-trip/parity integration tests incl. B-GCF-QUOTE inline-array quoting cases | done |
| W2-GATES | `cargo fmt --all --check`, `cargo clippy -p omniroute-gcf --all-targets` clean, `cargo test -p omniroute-gcf` pass | done |
| W2-COMMIT | Conventional commit in omniroute + push | pending (this wave) |

**W2 implementation:**
- `crates/omniroute-gcf/src/generic.rs` — generic encoder (objects/arrays, tabular detection, flattening, inline/shared attachments/arrays, expanded forms) over `serde_json::Value` (preserve_order).
- `crates/omniroute-gcf/src/decode_generic.rs` — generic decoder (profile/header validation, root forms, object/array/tabular/expanded parsing, attachments, path-column unflatten with order preservation, numeric parity preserving integer forms and `-0.0`).
- `crates/omniroute-gcf/tests/generic.rs` — 7 integration round-trips: B-GCF-QUOTE lossless case (`ERR[404]: Not Found`, `[Speaker 1]: ...`), homogeneous/simple, root primitives/arrays, simple object, heterogeneous keys, mixed-type columns, nested objects/tabular flattening.

**Parity evidence (Rust):** 16 tests total green (9 scalar unit + 7 generic integration), `clippy` clean for `omniroute-gcf`.

## Proposed next waves (owner-prioritized)

| Wave | Candidate | Rationale | Risk |
|------|-----------|-----------|------|
| W3 | Compression pure utilities (caveman rules/token helpers/validators) as crate(s) | perf/value, testable | M |
| W4 | N-API or WASM bindings for selected crate(s) + Node adapter | integration boundary | M/L |
| W5 | `open-sse` hot-path components (translator/chunk/protocol primitives) behind adapters | core engine leverage | L |
| W6+ | Services/handlers/routes/domain slices, incremental strangler-fig | broad app impact | L/XL |

> Note: original W2 draft listed "GCF generic + golden vectors vs TS"; core generic encode/decode + Rust parity tests are landed. Additional TS-shared golden vectors / full TS↔Rust fixture harness can be appended as hardening if desired.

## Risks / constraints

- omniroute CI is Node/npm-centric (many workflows); Rust jobs/toolchain may need to be added in a later wave (not changing existing workflows except additive files).
- Some TS modules depend on Node/browser APIs; not all are portable to pure Rust lib.
- License/provenance: preserve upstream notices for vendored logic ports (GCF MIT noted in TS originals).
- Keep omniroute `.env`/keys out of commits (SEC rules still apply).
- npm/npx hooks may be skipped in constrained envs (as in W1/W2 commits); TS/JS untouched in these waves, Rust gates authoritative for changed Rust code.

## Gates

- Rust: `cargo fmt --all -- --check`, `cargo clippy -p omniroute-gcf --all-targets` (clean), `cargo test -p omniroute-gcf` in omniroute workspace.
- No changes to tracked TS runtime behavior in these waves.
- Commit: one conventional commit per wave/slice in omniroute, push with summary. Plan doc maintained in GSV.
