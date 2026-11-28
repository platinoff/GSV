# Rust 1.100 + WebAssembly standards audit — 2026-11-28

**Status:** read-only research + recommendations. **No product code changed**, no
correctness claims (numerics/quality gates untouched, G13 still blocked).
**Scope:** GSV host + 13 registered plugins + 2 unregistered git trees from
`cargo xtask products`; installed toolchain `rustc 1.100.0 (0ef1bc386 2026-11-09)`,
`cargo 1.100.0`, default `stable-x86_64-pc-windows-gnu`. All repos clean/pushed
after this session's drain (GSV `02d6b00`, rusty-engine `81262f8`, LinFS `84ca275`,
sport-forcast `8ec013c`); PH-1202 board row is `open` again after lease expiry.

## 0. Sources

- Rust 1.100.0 announcement / release notes: <https://blog.rust-lang.org/2026/11/12/Rust-1.100.0/>,
  <https://doc.rust-lang.org/stable/releases.html>, <https://releases.rs/docs/1.100.0/>
  (1.100.1 backport released 2026-11-27: iterator-inference fix).
- Clippy 1.100 changelog: <https://github.com/rust-lang/rust-clippy/blob/master/CHANGELOG.md>.
- Cargo lint system / min-publish-age: Rust 1.100 release notes above.
- Wasm 3.0 live standard: <https://webassembly.org/news/2025-09-17-wasm-3.0/>,
  W3C CR draft 2026-11-24 <https://www.w3.org/TR/wasm-core-2/>.
- WASI 0.3 / `wasm32-wasip3` Tier 2 (first stable in 1.100):
  <https://doc.rust-lang.org/rustc/platform-support/wasm32-wasip3.html>,
  <https://github.com/webassembly/wasi/releases/tag/v0.3.0>,
  <https://component-model.bytecodealliance.org/language-support/creating-runnable-components/rust.html>
  (Wasmtime 46+ required for final 0.3.0).
- SIMD128 deployment / wasm-opt: <https://doc.rust-lang.org/core/arch/wasm32/index.html>,
  <https://github.com/WebAssembly/binaryen>, Wasmi optimization guidance
  <https://github.com/wasmi-labs/wasmi/blob/main/docs/usage.md>.
- Local evidence: repo `Cargo.toml` / `rust-toolchain.toml` / `.cargo/config.toml`,
  `.github/workflows/`, `cargo xtask products`, `git status -sb`, `rustc --version`,
  plus this session's gate logs (fmt/clippy `-D warnings`/`cargo test` 0 failed,
  GSV loc-audit 99.68%, sync rev 516).

## 1. Verdict

| Area | State | Risk |
|---|---|---|
| Toolchain pin | 13/16 trees pin `stable-x86_64-pc-windows-gnu`; local toolchain 1.100.0 | **Medium** — floating channel vs docs claiming exact 1.100.0; upstream 1.100.1 not installed locally |
| `RUSTUP_TOOLCHAIN` policy | GSV kit says `unset`; poolAI exports it in AGENTS/scripts/CI; LinFS/llama-rs/mesh-node docs still export | **High** (config drift, not code correctness) |
| Cargo 1.100 lints | Only GSV has `[lints.cargo] unused_dependencies="warn"`; LinFS emitted 10 cargo-manifest warnings in this session | **Medium** |
| Dependencies cooldown | No `[registry] global-min-publish-age` anywhere | **Low/Medium** (supply chain policy decision) |
| Edition / MSRV | 3 repos edition 2024 (rebook, ORR_DESKTOP, adoreble_wifi), rest 2021; only rusty-engine + sport-forcast declare `rust-version=1.85` | **Low** — migration is optional, not a defect |
| Lint suppressions | poolAI 137, rusty-engine 24, ORR 22, GSV 16; package-level `result_large_err="allow"` in poolAI; no `[workspace.lints]` or `clippy.toml` | **Medium** — legacy debt, freeze rather than bulk-edit |
| CI | poolAI 7 workflows all `RUSTUP_TOOLCHAIN: stable` (deliberate Linux fix); llama-rs/ORR use older `checkout@v4`/`cache@v4`; adoreble_wifi Windows CI uses MSVC; omniroute ~28 workflows with **no Rust CI** | **Low/Medium** |
| WebAssembly | 3 artifacts; GSV SIMD128 kernel 28 035 B + JS fallback is deliberate; `wasm-opt` never applied (MOD-3 owner-gated); docs still say 27 462 B | **Medium** — doc/artifact drift + missing size optimization |

## 2. Rust 1.100 — findings → recommendations

1. **[P0] Pin policy is split-brained.** GSV `AGENTS.md`, `README.md`, HANDOFF and
   `.cursor/rules/*` say `unset RUSTUP_TOOLCHAIN` so `rust-toolchain.toml` wins;
   poolAI `AGENTS.md`, `bin/build-ui-wasm.sh`, `bin/pre-push-hook.sh`,
   `record-test-ci-speed.sh`, `record-rust-diagnostics.sh` and 7 docs **export**
   it; LinFS/llama-rs/mesh-node docs still show exports. `GSV/docs/gsv/PLUGIN_AGENTS.md:34`
   is self-contradictory. → Standardize: **never export in local shells**; keep
   CI-level `RUSTUP_TOOLCHAIN: stable` only where it fixes the Linux/gnu mismatch
   (poolAI) and document that exception explicitly. No code impact.
2. **[P0] 1.100.0 vs 1.100.1.** 1.100.1 (2026-11-27) backports an iterator
   inference regression fix. Local toolchain is still 1.100.0. → Owner choice:
   (a) `rustup update stable-x86_64-pc-windows-gnu` (floating channel picks 1.100.1),
   or (b) pin exact `channel = "1.100.1-x86_64-pc-windows-gnu"` in all GNU trees.
   Then re-run `cargo xtask check` per product. Do **not** export `RUSTUP_TOOLCHAIN`.
3. **[P1] Cargo 1.100 lints unused.** `[lints.cargo]` exists only in GSV
   (`Cargo.toml:139-140`). This session's LinFS build printed warnings for unused
   workspace-package fields and unused dependencies. → Add
   `[lints.cargo] unused_dependencies = "warn"` to each Rust workspace; triage
   warnings per product. Do **not** bulk-remove poolAI's 137 `allow(...)` or
   its `result_large_err="allow"` (ECO-8.5 decision) without a dedicated band.
4. **[P1] `build-dir` layout changed in Cargo 1.100** (artifacts grouped by
   build step, hash moved to parent dir). Any script that greps `target/debug`
   paths rather than using Cargo JSON messages may break. → Audit `cargo xtask`
   disk/artifact helpers and `gsv-loc-audit`/`xtask` path assumptions; read-only
   check now, fix only on concrete failure. Note we removed stale `target/debug/build`
   etc. on 2026-11-28 to free disk; a rebuild is expected.
5. **[P1] Never-type fallback changed on editions 2015/2018/2021** (`()` → `!`);
   more code may be flagged unreachable, and `Infallible` is now an alias. →
   After the 1.100.1 update, run `cargo clippy --all-targets -- -D warnings`
   across the 2021-edition repos; review only new warnings, no semantic edits.
6. **[P2] Edition 2024** is stable since 1.85; rebook/ORR/adoreble_wifi already
   use it, proving feasibility. → Per-product migration band for the remaining
   11 Rust trees **only after** the higher-priority config issues close; each
   band carries its own fmt/clippy/test gates. Not a defect today.
7. **[P2] MSRV.** Only rusty-engine and sport-forcast declare `rust-version="1.85"`
   (deliberate for Pi/Android portability). → Keep 1.85 for plugins; add a one-line
   rationale comment in those `Cargo.toml` files, do not raise MSRV without owner
   sign-off (Pi 4 / aarch64 toolchain constraints).
8. **[P2] Supply chain.** Cargo 1.100 supports `[registry] global-min-publish-age`
   (dependency cooldowns). → Evaluate `global-min-publish-age = "3 days"` in GSV
   and poolAI `.cargo/config.toml`; it will interact with the ECO-8.3 patch-sweep
   workflow (`cargo update --precise`), so make it an owner policy decision, not
   a silent config edit.
9. **[P2] CI hygiene.** `llama-rs` and `ORR_DESKTOP` still use `actions/checkout@v4`
   / `cache@v4` (ecosystem peers use @v6/@v7); `adoreble_wifi` Windows CI uses
   `stable-x86_64-pc-windows-msvc` while its local pin is host-agnostic `stable`
   — document as intentional; omniroute has no Rust pin and no Rust CI despite
   `omniroute-wasm`. → Bump actions in a docs/CI-only band; add either a
   `rust-toolchain.toml` + minimal `cargo check` workflow to omniroute or an
   explicit "node-primary, Rust wasm crate is vendored" exemption in PRODUCTS.md.
10. **[P3] Stale version prose** (not code): poolAI README/status/release/troubleshooting
    docs still say 1.87/1.92; GSV `GSV_FRAMEWORKS_SURVEY`, `GSV_TECH_ROADMAP`,
    `TASKS.md`, GSV-WGPU spec; GSV/poolAI/rusty-engine/sport-forcast READMEs
    say "pins 1.100.0" although the channel floats. → One docs-only sweep per
    product, replacing exact-version claims with "floating stable-gnu, currently
    1.100.x" or the chosen exact pin from item 2. Fix alongside the WASM size
    drift in item 11.

## 3. WebAssembly — findings → recommendations

| Artifact | Size | md5 | Notes |
|---|---|---|---|
| `GSV/ui/assets/mesh-wasm.wasm` | 28 035 B | `30bd74ad063cac53bf5cd3aa591cdb41` | SIMD128 dequant kernel + JS fallback; FIX-3 |
| `poolAI/src/ui/wasm/poolai_ui_wasm_bg.wasm` | 569 307 B | `54a334d4208dd8f64128aa3b3a03986d` | wasm-bindgen 0.2.108 CLI pin |
| `omniroute/open-sse/lib/sha3_wasm_bg.wasm` | 26 612 B | `fa5f90dcec4f33f638ee3709239c4d77` | `wasm-opt = false` in wasm-pack metadata |

11. **[P1] Doc/artifact mismatch now.** `GSV_TECH_ROADMAP.md` and
    `docs/NEXT_SESSION_PROMPT.md` still claim 27 462 B; on-disk FIX-3 kernel is
    28 035 B. → Correct both docs to 28 035 B / md5 prefix `30bd74ad…` in a
    docs-only commit. Cheap, removes a false audit signal.
12. **[P1] Keep strict SIMD128, do not enable relaxed SIMD globally.** Wasm 3.0
    adds relaxed vector ops whose results may differ per engine; our golden
    parity (`max_err` checks, bit-identical pack/GGUF comparisons) requires the
    deterministic profile. SIMD-capable browsers are the norm, but a SIMD module
    **fails at validation** on old engines — the existing JS fallback is the
    correct deployment hedge. → Keep two artifacts (SIMD + JS), verify vector
    instructions with `wasm-objdump` after any rebuild, and treat relaxed SIMD
    only as an optional third artifact for a measured kernel.
13. **[P1] `wasm-opt` is not applied** (MOD-3 parked, needs Binaryen install —
    previously owner-gated because it would require `pacman -S binaryen`). Wasm
    3.0 / modern toolchains expect a post-optimized binary; `wasm-opt -Oz
    --enable-simd` typically shrinks a Rust cdylib materially. → Owner decision:
    approve a controlled Binaryen install inside the environment, then add a
    GSV xtask post-build step, re-run golden parity, and update size/md5 docs.
    Do not silently change the shipped bytes.
14. **[P2] wasm-bindgen / wasm-pack metadata.** poolAI pins CLI 0.2.108 in
    `bin/build-ui-wasm.sh` while `Cargo.toml` says `wasm-bindgen = "0.2"`;
    omniroute explicitly disables `wasm-opt`; adoreble_wifi has a wasm crate but
    no build script. → Per-product band: align CLI and crate versions with
    `cargo update -p wasm-bindgen`, enable `-Oz` where size matters, and document
    why omniroute disables optimization (if any). Re-verify each artifact's
    hash after rebuild.
15. **[P2] `wasm32-wasip3` (WASI 0.3) is Tier 2 as of 1.100** and needs
    `wasi-sdk-34` + Wasmtime 46+ with matching WIT version. This is relevant to
    non-browser components (mesh-node/agent), **not** to the browser dequant
    kernel — the component model is unavailable in browsers. → Feasibility
    note only; do not migrate the mesh kernel. If a CLI component is ever
    componentized, pin all of `wit-bindgen`/Wasmtime/jco to 0.3.0 together.
16. **[P2] Format/ratio policy stays.** `.wasm` is allowed at ≤5% of audited
    product LOC (GSV `AGENTS.md`, `inventory.rs`). Current sizes are tiny;
    adding a second SIMD/relaxed artifact must stay under that budget. → Include
    `loc-audit --stretch-96` in any wasm-band gate.

## 4. Proposed ticket set (not created — awaiting owner/product decision)

| Id (proposed) | Scope | Owner gate? |
|---|---|---|
| `ECO-9.3` | Unify `RUSTUP_TOOLCHAIN` policy + fix `PLUGIN_AGENTS.md:34`; docs-only | no |
| `ECO-9.4` | Toolchain 1.100.1 update (floating vs exact pin decision) + per-product re-gate | **yes** (pin policy) |
| `ECO-9.5` | Cargo `[lints.cargo]` adoption + triage per workspace, no bulk `allow` removal | no |
| `ECO-9.6` | Stale Rust-version prose sweep + WASM size/md5 doc fix (27 462 → 28 035 B) | no |
| `ECO-9.7` | wasm-opt/Binaryen post-build band (GSV kernel) + golden re-verify | **yes** (tool install) |
| `ECO-9.8` | wasm-bindgen alignment + wasm-pack metadata per plugin (poolAI/omniroute/adoreble) | no |
| `ECO-9.9` | CI action bumps (llama-rs/ORR) + omniroute Rust-pin/CI exemption | no |
| `ECO-9.10` | Edition 2024 migration per product, one repo per band | no, but low priority |

## 5. Honest boundary

This is a configuration/standards audit, not a semantic correctness review and
not a permission to mass-edit dependencies or toolchains. No model was run, no
quality claim, no image→answer claim (G13 blocked). PH-406/1206 and the mtmd
golden parity remain owner-gated; the B12 code landed in rusty-engine is
structural/numeric only. All recommendations are proposals until filed as
tickets and gated per product.
