# Plan — Rust 1.100 / Wasm 3.0 standards drain (2026-11-28)

**Source:** [`gsv/research/RUST_1_100_WASM_STANDARDS_AUDIT_2026-11-28.md`](../../gsv/research/RUST_1_100_WASM_STANDARDS_AUDIT_2026-11-28.md)
(recommendation-only audit, no product code changed). **Board:** 12 tickets filed
via MCP `gsv_tickets_create`; **all are `open`, none claimed**. Owner-gated rows
(9.4, 9.7) must stay unclaimed until explicit owner go.

## Rules for this drain (GSV kit canon)

1. Session cycle: `gsv_tickets_presence` → `gsv_products` / explicit product pick →
   `gsv_tickets_claim` → drain in the **selected tree only** → `gsv_tickets_done`
   with evidence → `gsv_tickets_next`. Never touch another's `claimed_by`; stale
   leases return to `open` only via `gsv_tickets_reclaim` (`GSV_VDT_KIT.md:121-139`).
2. **One ticket = one product = one commit** in that product's repo. Never mix
   products in one commit; never `git add -A`; trailers `Gsv-Actor/Gsv-Ide/Gsv-Model`.
   Push only per product law (GSV and registered plugins have `origin`; GSV-WGPU /
   mesh-node are remote-less → local commit only, owner gate for remote).
3. Two `cargo` on the same `target/` is forbidden; different product targets may
   run in parallel. PoolAI local gate = `fmt + clippy + check`; **do not run
   `cargo test-ci` locally** (RAM/EDR) — GitHub Actions is authoritative.
4. Do not bulk-delete lint suppressions (poolAI 137 `allow(...)`, package-level
   `result_large_err="allow"` from ECO-8.5). Do not change pinned dependency
   versions blindly (`wasm-bindgen = "=0.2.108"`, llama-cpp-2 pin).
5. Owner-gated rows: `ECO-9.4` (exact pin vs floating toolchain), `ECO-9.7`
   (Binaryen install). No RB/book tickets, no `:8091`, no token rotation, no
   image→answer or quality claim (G13 blocked).

## Ticket register

| Ticket id | Product | Scope | Gate |
|---|---|---|---|
| `t-1795876931168114400` | gsv | **ECO-9.3a** unify `RUSTUP_TOOLCHAIN` canon (`PLUGIN_AGENTS.md:34`, stale plan export) | docs-only |
| `t-1795876931271214500` | poolai | **ECO-9.3b** remove local exports; keep CI `stable` exception documented | docs/scripts |
| `t-1795876931344528300` | gsv | **ECO-9.4** 1.100.1 pin policy — **OWNER-GATED, do not claim** | owner decision |
| `t-1795876931427540100` | gsv | **ECO-9.5a** Cargo 1.100 lints + build-dir / never-type audit | full GSV gates |
| `t-1795876931524358000` | linfs | **ECO-9.5b** `[lints.cargo]` + 10 manifest warnings | full LinFS gates |
| `t-1795876931604872700` | gsv | **ECO-9.6a** stale version prose + WASM size 27 462 → 28 035 B | docs-only |
| `t-1795876931682755800` | poolai | **ECO-9.6b** stale 1.87/1.92 toolchain prose | docs-only |
| `t-1795876931763500500` | gsv | **ECO-9.7** Binaryen/wasm-opt post-build — **OWNER-GATED, do not claim** | owner decision |
| `t-1795876931842077200` | poolai | **ECO-9.8a** align wasm-bindgen CLI/crate + artifact hash | local gates |
| `t-1795876931913129400` | llama-rs | **ECO-9.9a** checkout/cache v4 → supported majors | CI-only |
| `t-1795876932006522400` | omniroute | **ECO-9.9b** Rust pin + minimal wasm CI or documented exemption | one commit + push |
| `t-1795876932081609700` | gsv | **ECO-9.10** edition-2024 migration scouting (no migration code) | docs-only |

## Ordered waves

### Wave 0 — canon consistency (no code)
1. **9.3a (gsv)** — make the kit say one thing: local shells never export
   `RUSTUP_TOOLCHAIN`; `rust-toolchain.toml` wins. Fix `PLUGIN_AGENTS.md:34` and
   the stale export example in the 2026-08-18 plan. Close with the new canon
   quoted in the done-note.
2. **9.3b (poolai)** — remove local exports in AGENTS.md + four `bin/*.sh` + seven
   docs; explicitly retain CI `RUSTUP_TOOLCHAIN: stable` as the Linux/gnu fix.
   Local gate: fmt + clippy + check; one commit + push.

### Wave 1 — stale facts (must land before 9.7)
3. **9.6a (gsv)** — 28 035 B / md5 `30bd74ad…` in `GSV_TECH_ROADMAP.md:2396` and
   `NEXT_SESSION_PROMPT.md:3`; replace "pins 1.100.0" with floating-channel wording.
   Leave historical archives intact. `git diff --check`, `cargo xtask sync`.
4. **9.6b (poolai)** — 1.87/1.92 → "floating stable-gnu, currently 1.100.x"
   (or exact 1.100.1 **only after** owner decision 9.4). Docs-only commit + push.

### Wave 2 — Cargo 1.100 lints
5. **9.5a (gsv)** — verify `[lints.cargo]` behaviour under 1.100; inspect xtask /
   disk / loc-audit for hard-coded `target/debug` layout assumptions (Cargo 1.100
   changed build-dir layout); review new clippy findings from never-type fallback.
   Fix only concrete failures. Full gates: fmt, clippy `-D warnings`, `cargo test`,
   loc-audit `--stretch-96`, `xtask sync`.
6. **9.5b (linfs)** — add `[lints.cargo] unused_dependencies="warn"`, triage the
   10 manifest warnings (unused workspace fields + unused deps), one crate at a
   time with rationale comments. Full LinFS gates.
7. Further products (rusty-engine, ORR, GSV-WGPU, …) get new per-product tickets
   **on demand**, not bundled into 9.5a/b.

### Wave 3 — WebAssembly
8. **9.8a (poolai)** — align `wasm-bindgen` CLI 0.2.108 with the crate constraint
   via patch-level `cargo update`, rebuild `poolai_ui_wasm_bg.wasm`, record
   size/md5. Keep SIMD/JS deployment split untouched.
9. **9.9b (omniroute)** — add a Rust toolchain pin + minimal wasm `cargo check`
   workflow, or file the node-primary exemption in GSV PRODUCTS.md (separate GSV
   docs commit). Do not silently change `=0.2.108`.
10. **9.7 (gsv, OWNER-GATED)** — after owner go and after 9.6a: install Binaryen,
    add `wasm-opt -Oz --enable-simd` post-build, re-verify Q4_K golden, update
    size/md5 docs. **Never** enable relaxed SIMD globally — non-deterministic
    results break the bit-identical pack/GGUF parity.

### Wave 4 — CI (separate product bands)
11. **9.9a (llama-rs)** — bump `checkout@v4`/`cache@v4` to supported majors; keep
    `toolchain: stable-x86_64-pc-windows-gnu` and `LIBCLANG_PATH` handling.
12. ORR_DESKTOP CI bump: file on demand (same pattern as 9.9a).

### Wave 5 — deferred / owner decisions
13. **9.10 (gsv)** — edition-2024 scouting inventory + proposed one-repo-per-band
    order; no `Cargo.toml` edits in this ticket.
14. **9.4 (gsv, OWNER-GATED)** — choose floating stable-gnu vs exact 1.100.1 pin;
    then fan out per-product re-gates as new tickets. Local toolchain is 1.100.0;
    1.100.1 is available upstream and fixes an iterator-inference regression.

## Definition of done (per ticket)

- `gsv_tickets_done` with evidence: commands + exit codes, changed files, HEAD,
  artifact hash when wasm is rebuilt.
- Product repo `git status` clean; one commit with `Gsv-*` trailers; pushed where
  allowed; `gsv_tickets_next` for the next action.
- No quality claim, no model run, no image→answer claim; G13 / PH-406 / PH-1206
  unchanged. No locked or owner-gated work claimed.

## Current state

All 12 rows open, none claimed. `PH-1202` is open again after lease expiry
(board reclaim `02d6b00`); its structural + real-mmproj code is already in
rusty-engine `bc236ac`, only mtmd golden parity remains blocked. Nothing in this
plan has been executed yet beyond filing the tickets and this plan document.
