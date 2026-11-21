# CNV-1 Wave 1 — omniroute TS→Rust (bootstrap)

**Status:** in progress · **Date:** 2026-11-20  
**Ticket:** CNV-1 (owner: exempt-till-epic → epic started, Wave 1 go)  
**Repo:** `S:\rust\omniroute` (origin `https://github.com/diegosouzapw/OmniRoute`, branch `release/v3.8.50`)  
**Scope reality:** 8,263 tracked TS/JS-family files deferred under CNV-1 (Next.js 16 app + `open-sse/` streaming engine + Electron + CLI + tests + scripts). Full migration is a multi-wave epic; Wave 1 is a **foundation + proof-of-port**, not a rewrite.

## Goals (Wave 1)

1. Establish Rust build foundation inside omniroute without breaking existing npm/Node flows.
2. Port one small, pure, deterministic module to Rust with tests as a reference migration slice.
3. Document strategy, boundaries, and next waves.

## Non-goals (Wave 1)

- No mass TS→Rust conversion of app/routes/providers/UI.
- No N-API/WASM/FFI integration into the running Node app yet (follow-up wave).
- No changes to omniroute runtime behavior, CI semantics, or build outputs beyond additive Rust workspace files.
- No touching `.env`, secrets, `node_modules`, `target/` artifacts, rebook, model weights.

## Strategy

- **Additive Rust workspace:** `crates/` + root `Cargo.toml` workspace (resolver 2). Existing TS/JS remains source of truth for runtime until a port is wired.
- **Pick pure, dependency-light modules first:** deterministic transforms, parsers/encoders, config/validation logic, utilities. Avoid DB/network/UI/framework-coupled code early.
- **Parity via tests:** Rust ports mirror TS semantics and include unit tests (and later golden vectors shared with TS).
- **Integration later:** after parity crates stabilize, expose via N-API (`napi-rs`) or WASM or sidecar/CLI, then swap call sites wave-by-wave behind tests.

## Wave 1 deliverables

| ID | Deliverable | Status |
|----|-------------|--------|
| W1-PLAN | This plan (scope, waves, risks) | done |
| W1-BOOT | Rust workspace bootstrap in omniroute (`Cargo.toml`, `crates/omniroute-gcf/`, `.gitignore` entries for Rust targets) | in progress |
| W1-PORT-GCF-SCALAR | Port `open-sse/services/compression/engines/headroom/gcf/scalar.ts` → `crates/omniroute-gcf` (Rust lib, unit tests, parity-focused) | in progress |
| W1-GATES | `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings` (or project policy), `cargo test` pass for new crate(s) | pending |
| W1-COMMIT | Conventional commit in omniroute + push to `origin/release/v3.8.50` with summary | pending |

## Wave 1 target: GCF scalar module

- **Source:** `open-sse/services/compression/engines/headroom/gcf/scalar.ts` (vendored GCF scalar grammar; MIT in upstream header). Pure functions: quoting/escaping, scalar format/parse, key formatting, delimiter-aware split, field decl parsing, sentinels.
- **Why:** small, deterministic, no I/O/framework deps, clear spec behavior, easy to unit-test and later bind.
- **Rust crate:** `crates/omniroute-gcf` (lib `omniroute_gcf`, module `scalar`). API shaped for parity (String/&str in, Result/Value out) with idiomatic Rust types; JS `Symbol` sentinels represented as enum variants.
- **Parity notes:** number formatting (`toExponential` normalization, `String(f)` shortest round-trip), Unicode whitespace/control handling, surrogate-pair decode/validation, and error messages kept behavior-compatible where practical (tests assert key cases).

## Proposed next waves (post-W1, owner-prioritized)

| Wave | Candidate | Rationale | Risk |
|------|-----------|-----------|------|
| W2 | Complete GCF (generic encode/decode) + golden vectors vs TS | contiguous module, pure data transform | M |
| W3 | Compression pure utilities (caveman rules/token helpers/validators) as crate(s) | perf/value, testable | M |
| W4 | N-API or WASM bindings for selected crate(s) + Node adapter | integration boundary | M/L |
| W5 | `open-sse` hot-path components (translator/chunk/protocol primitives) behind adapters | core engine leverage | L |
| W6+ | Services/handlers/routes/domain slices, incremental strangler-fig | broad app impact | L/XL |

## Risks / constraints

- omniroute CI is Node/npm-centric (many workflows); Rust jobs/toolchain may need to be added in a later wave (not changing existing workflows in W1 except additive files).
- Some TS modules depend on Node/browser APIs; not all are portable to pure Rust lib.
- License/provenance: preserve upstream notices for vendored logic ports (GCF MIT noted in source).
- Keep omniroute `.env`/keys out of commits (SEC rules still apply).

## Gates (Wave 1)

- Rust: `cargo fmt --all -- --check`, `cargo clippy --all-targets` (warnings policy as agreed), `cargo test` in omniroute workspace.
- No changes to tracked TS runtime behavior in W1; existing omniroute test suites not modified (full npm test matrix is CI/heavy - run only if targeted checks are warranted for touched areas).
- Commit: one conventional commit for W1 bootstrap+port (+ plan reference), push with summary.
