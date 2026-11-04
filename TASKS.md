# Tasks

## Active

- [ ] **ECO-2: dep refresh — latest best frameworks across all products (band-per-product)** — ticket `t-1793795019678869200`, product gsv; heavy/long, band-per-product, gates never skip
- [ ] **ECO-3: portable auto-wire — `cargo xtask adopt <path>` (Rust, hub xtask)** — ticket `t-1793795019731059800`, product gsv; dry-first, `--yes` required

## Waiting On

- [ ] **ECO-5: rust 1.100 ready-lane** — ticket `t-1793795030516889300`; unblock when 1.100 stable lands
- [ ] **ECO-4 bands: owner-review dirs (`freebeat.ai`, `reebeat.ai`, Tokenomica, platinoff-profile) + retire 1.92/1.96 toolchains (~5G) after ECO-2 gates** — ticket `t-1793795030452014000`
- [ ] **sec owner-gated: rotate Telenetis bot token** — ticket `t-1789606660639448500`; owner must rotate, agent never echoes tokens

## Someday

## Done

- [x] ~~**ECO-1: ecosystem toolchain flat policy — stable-gnu pins everywhere + rustc 1.99.1**~~ (2026-11-04) - ticket `t-1793795019622282400`; rustup 1.29.3, stable gnu+msvc → 1.99.1, unified pin in 11 repos, stale poolAI override removed, AGENTS.md PATH fix
- [x] ~~**G2: GSV-WGPU WGSL compute smoke (matmul) — adapter+queue lane**~~ (2026-11-04) - ticket `t-1793734218923010900`, band G2, product gsv-wgpu
- [x] ~~**G1: GSV-WGPU probe — wgpu adapter discovery**~~ (2026-11-03) - ticket `t-1793635200123456791`, band G1, product gsv-wgpu
- [x] ~~**G0: GSV-WGPU plugin skeleton**~~ (2026-11-03) - ticket `t-1793635200123456790`, band G0, product gsv-wgpu
