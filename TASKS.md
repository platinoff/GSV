# Tasks

## Active

- [ ] **ECO-6: ecosystem drain — cleanup, dead-code, perf-frameworks, dedup, updates (owner ask 2026-11-06)** — epic `t-1793982750395443700`; sub-tickets (2026-11-06): cleanup ✅, далі drain:
  - ✅ Step 1 cleanup executed (2026-11-06): llama-rs `.cmake/*.zip` + omniroute `llm.txt` + poolAI `comitmsg/*.txt`/`commit_msg.txt` + poolAI dep-refresh lock (`e2a454421`, `50808b7`, `7ae79c3`)
  - ✅ **ECO-6.2 dead-code/undeveloped/broken hunt done (2026-11-06)** — ticket `t-1793983440266486800` (done): compiler dead_code clean in gated repos; LinFS TODOs = planned band 204 + graceful Err fallbacks; poolAI todo!() = doc-example only; no actionable dead-code findings
  - ✅ **ECO-6.3 concepts vs concept-plans done (2026-11-06)** — ticket `t-1793983440323800500` (done): scan mapped concept docs per repo; ORR_DESKTOP pure-Rust gap resolved (shipped), no new gap confirmed; poolAI docs/concept/*.txt presence noted. First deep-gap list → follow-up tickets if needed.
  - ✅ **ECO-6.4 web-research + update-plan notes done (2026-11-06)** — ticket `t-1793983440380816200` (done); findings → docs/gsv/GSV_SPEED_RESEARCH_2026-11.md; deeper update-plan adoption per band remains follow-up.
  - ✅ **ECO-6.5 dedup scan done (2026-11-06)** — ticket `t-1793983440434122000` (done): no internal duplicate items; cross-repo conceptual dupes flagged (bus/ticket infra).
  - ✅ **ECO-6.7 txt/ps/py junk cleanup executed (2026-11-06)** — ticket `t-1793983440542048300` (open): poolAI tracked docs/concept+docs/archive .txt deleted (2752 lines), untracked commit-msg notes removed; rebook/omniroute build junk (.build, KDP.txt) removed; rusty-engine golden fixture kept.
  - ⬜ **ECO-6.6 dep updates + tests via tickets, git push per ticket** — ticket `t-1793983440484757800` (open) ← **NEXT DRAINING**
  - ⬜ **ECO-6.8 ripgrep instead of grep** — ticket `t-1793983440593346700` (open)

- [ ] **ECO-2: dep refresh — latest best frameworks across all products (band-per-product)** — ticket `t-1793795019678869200`, product gsv; heavy/long, band-per-product, gates never skip
  - ✅ Bands done: gsv (`effd45f`: tokio 1.54.1, serde_json 1.0.154, toml 1.2.0, rustls 0.23.46, rustls-webpki 0.103.16; osv advisories 12→0; test exit 0), gsv-wgpu (`96a6548`: ordered-float 5.5.1; 8/8), linfs (`a01d543`), telenetis (`4a24f41`), orr_desktop (`4a26a13`), rebook (`30e97f5` + fmt fix), sport-forcast (`d71cbeb` no-op), coreboard (`14668a3`)
  - ⛸️ Bands open: poolai (deps refreshed OK; fmt ✓ clippy ✓ canonical green; full `cargo test` blocked by S:/ disk full — link stage "No space left", rerun after cleanup), llama-rs (update no-op; clippy blocked by pre-existing llama-cpp-sys-2 native build: cpp-httplib Win8 error; not dep-caused)
  - ➡️ Next bands: poolai (rerun full test after disk cleanup), llama-rs (build fix), omniroute (npm N/A); then ECO-4/ECO-5 sequencing

## Waiting On

- [ ] **ECO-5: rust 1.100 ready-lane** — ticket `t-1793795030516889300`; unblock when 1.100 stable lands; now on **forecast** (owner ask 2026-11-06: periodic check whether release is out)
- [ ] **ECO-4 bands: owner-review dirs (`freebeat.ai`, `reebeat.ai`, Tokenomica, platinoff-profile) + cargo-cache taxonomy** — ticket `t-1793795030452014000`; toolchain retire 1.92/1.96 DONE this block (toolchains gone, cargo still resolves 1.99.1)
- [ ] **sec owner-gated: rotate Telenetis bot token** — ticket `t-1789606660639448500`; owner must rotate, agent never echoes tokens

## Someday

## Done

- [x] ~~**ECO-3: portable auto-wire — `cargo xtask adopt <path>` (Rust, hub xtask)**~~ (2026-11-05) - ticket `t-1793795019731059800`; GSV `71e4b71`: digest + artifacts (AGENTS/pin/HANDOFF/NEXT/PRODUCTS row/workspace), dry-first `--yes`, 6 unit tests, smoke dry-run on `freebeat.ai`; GSV-WGPU wiring fixed `72b4708`
- [x] ~~**ECO-1: ecosystem toolchain flat policy — stable-gnu pins everywhere + rustc 1.99.1**~~ (2026-11-04) - ticket `t-1793795019622282400`; rustup 1.29.3, stable gnu+msvc → 1.99.1, unified pin in 11 repos, stale poolAI override removed, AGENTS.md PATH fix
- [x] ~~**G2: GSV-WGPU WGSL compute smoke (matmul) — adapter+queue lane**~~ (2026-11-04) - ticket `t-1793734218923010900`, band G2, product gsv-wgpu
- [x] ~~**G1: GSV-WGPU probe — wgpu adapter discovery**~~ (2026-11-03) - ticket `t-1793635200123456791`, band G1, product gsv-wgpu
- [x] ~~**G0: GSV-WGPU plugin skeleton**~~ (2026-11-03) - ticket `t-1793635200123456790`, band G0, product gsv-wgpu
