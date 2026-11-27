# FUNCTION_MANAGEMENT.md — GSV (мама, хаб)

**Status:** Canon, adapted 2026-11-27 @ HEAD `51b1512` (GASV G32–G49 + streaming MoE FWD-1…8 / LOAD-0…6 / SCALE-1a / PEER-1 + ecosystem repair 22/22; debug canon `GSV_DEBUG_CANON.md`) (toolchain rust 1.100.0; rusty-engine `25b1d4e`, LinFS `5666918`, adoreble_wifi `141ede0`) · **Owner:** platinoff
**Компаньйони:** [`GSV_FUNCTION_MANAGEMENT.md`](./GSV_FUNCTION_MANAGEMENT.md) (майстер-матриця) · [`GSV_ECOSYSTEM_SYMBIOSIS.md`](./GSV_ECOSYSTEM_SYMBIOSIS.md) (мама/тато) · [`PRODUCTS.md`](./PRODUCTS.md)
**Шар:** мама — операційний план екосистеми (хаб). poolAI (тато-план, FM) очолює управління; цей файл — функційний реєстр GSV.

## 0. Джерела правди (порядок звірки)

1. Код: `src/` (lib.rs, lib boxes, `crates/mesh-wasm`), bins у `src/bin/`.
2. Контрактні тести: `tests/*.rs` (33 файли).
3. Стани: `docs/gsv/GSV_ECOSYSTEM_SYMBIOSIS.md`, handoff, vision `docs/vision/manifest.json` (rev 516).
4. `data/` докази (fingerprints.jsonl, speed_index.json, rust_diagnostics.json).

Статуси: **І** Implemented (тест/гейт) · **P** Partial (є API, wire неповний) · **S** Declared (реалізовано, немає виділеного контрактного тесту) · **G** Concept-only.

## 1. Функції GSV (звірено з деревом @ 2026-11-27, HEAD `51b1512`)

| # | Функція | Модуль | Статус | Звірка |
|---|---------|--------|--------|--------|
| 1 | HTTP router / AppState / hardened headers (CSP, nosniff, 256 KiB body cap, LAN gate `--allow-lan`) | `server/mod.rs`, `server/serve.rs`, `lib.rs` | І | `tests/gsv_server_contracts.rs`, `tests/gsv_security_contracts.rs` |
| 2 | MCP server (stdio `gsv-mcp` + `/mcp` HTTP/SSE sessions), 61 tools (from `GET /mcp`), resources/prompts, `catalog_stale` lockstep | `mcp.rs` | І | `tests/gsv_mcp_contracts.rs` |
| 3 | Galaxy UI — SPA + Rust-rendered cards (`/api/ui/card/<n>`), theme vars, dock/fullscreen, ERROR ring (`ui_errors`) | `ui.rs`, `ui/index.html`, `boxes/ui_errors.rs` | І | `tests/gsv_ui_contracts.rs` |
| 4 | Tickets board (claim/done/error/reclaim, lease 300s, federated presence/claim/done/reclaim) | `boxes/tickets.rs` | І | `tests/gsv_tickets_contracts.rs` |
| 5 | Ranks / Godfather merit ladder (L0–L15, peak, decay, grace-held) | `boxes/ranks.rs` | І | `tests/gsv_ranks_contracts.rs` |
| 6 | Telegram bus + poll loop + Godfather sync (dry-run default) | `boxes/telegram.rs` | І | `tests/gsv_telegram_contracts.rs` |
| 7 | Keep-live aggregation (GSV/Telenetis/llama-rs heartbeat/OmniRoute), fail-open | `boxes/keep_live.rs` | І | `tests/gsv_keep_live_contracts.rs` |
| 8 | Watchdog (self-respawn `target/live`, `version_lag`, debug-newer oneshot apply, sys-history sampler) | `boxes/watchdog.rs`, bin `gsv-watchdog.exe` | І | `tests/gsv_watchdog_contracts.rs` |
| 9 | Vision docs/panel (map, sprint-board, progress, feed, `speed_*`, `rust_diagnostics*` SVG) | `boxes/vision.rs` | І | `tests/gsv_vision_contracts.rs` |
| 10 | OmniRouter (catalog + quota + proxy tap), `bunke-rock`/`rusty-05` providers, route pick | `boxes/omni/{catalog,quota,proxy}.rs` | І | `tests/gsv_omni_contracts.rs` |
| 11 | APK / apk-edge bridge лежки | `boxes/apk.rs` | І | `tests/gsv_apk_contracts.rs` |
| 12 | Edge proxy (VDC §6: per-call token, 20 rps, allowlist) | `boxes/edge.rs` | І | `tests/gsv_edge_contracts.rs` |
| 13 | Grid / discovery / virtual-nodes mirror | `boxes/grid.rs` | І | `tests/gsv_grid_contracts.rs` |
| 14 | mesh WGPU dispatch (`/api/wgpu/*`, ready-gate, shard dispatch) | `boxes/wgpu.rs`, `crates/mesh-wasm` | І | `tests/gsv_wgpu_contracts.rs` |
| 15 | Fingerprints (JSONL, version lockstep) | `boxes/fingerprint.rs` | І | `tests/gsv_fingerprint_contracts.rs` |
| 16 | Hooks (`run mcp bot hook up scenario`) | `boxes/hooks.rs` | І | `tests/gsv_hooks_contracts.rs` |
| 17 | IDE helpers (preview messages, prompt blobs) | `boxes/ide.rs` | І | `tests/gsv_ide_contracts.rs` |
| 18 | MDS report (memory/disk/speed) | `boxes/mds.rs` | І | `tests/gsv_mds_contracts.rs` |
| 19 | Products scan/select (`cargo xtask products`, wiring) | `boxes/products.rs` | І | `tests/gsv_products_contracts.rs`, `tests/gsv_kit_wiring_contracts.rs` |
| 20 | Preview (path confine, highlight_rs) | `boxes/preview.rs` | І | `tests/gsv_preview_contracts.rs` |
| 21 | Ratio box + `gsv-loc-audit --stretch-96` | `boxes/ratio.rs`, bin `gsv_loc_audit` | І | `tests/gsv_ratio_contracts.rs` |
| 22 | Rules-check (git/vision/ratio/registry/sli/bench gates) | `boxes/rules.rs` | S (gate `cargo xtask rules-check`) | inline unit tests у `rules.rs` |
| 23 | Rust-intel (OSV vulns, CVSS v3.1, cadence §C) | `boxes/rust_intel.rs` | S | inline unit tests |
| 24 | SLI catalog (used/unused tracking) | `boxes/sli.rs` | І | `tests/gsv_sli_contracts.rs` |
| 25 | Settings / Godfather token store (redacted) | `boxes/settings.rs` | І | `tests/gsv_settings_contracts.rs` |
| 26 | SW shell-cache (`/sw.js`) | `boxes/sw.rs` | І | `tests/gsv_sw_contracts.rs` |
| 27 | Terminal (HTTP SLI allowlist; `cargo`/`git` only) | `boxes/terminal.rs` | І | `tests/gsv_terminal_contracts.rs` |
| 28 | Telegram `bus`/`inbox` + `kind:presence/claim/done/reclaim` envelope | `boxes/telegram.rs` | І | `tests/gsv_telegram_contracts.rs` |
| 29 | Toolchain probe (`cargo`/`rustc`/MSRV pinned) | `boxes/toolchain.rs` | І | `tests/gsv_toolchain_contracts.rs` |
| 30 | Usage tap (SSE `include_usage` taps, `/api/usage`) | `boxes/usage.rs` | І | `tests/gsv_usage_contracts.rs` |
| 31 | Update (SSE apply→offline, `/api/update/apply`) | `boxes/update.rs` | S | inline (band 208 rewritten test) |
| 32 | sysmon (host sample + sys-history ring) | `boxes/sysmon.rs` | І | `cargo xtask record-speed` / `sys-history` xtask |
| 33 | weight_store (35B shard upload/manifest) | `boxes/weight_store.rs` | I (live use: blk.0..blk.4 g-73…g-136) | dispatch через `#6` (mesh) |
| 34 | adoption / products hygiene / deny (license gate) | `boxes/adopt.rs`, `boxes/hygiene.rs`, `boxes/deny.rs` | S | inline |
| 35 | github/gitkit (queue/issues, `GH#N` hook) | `boxes/github.rs`, `boxes/gitkit.rs` | S | inline |
| 36 | guide / UI card guide | `boxes/guide.rs` | І | `tests/gsv_readme_contracts.rs` |
| 37 | xtask (products/disk/sync/intel/rules-check/record-speed/record-rust/sys-history/live) | `boxes/xtask.rs`, bin `gsv-xtask` | І | `tests/gsv_xtask_contracts.rs` |
| 38 | Stand smoke (feature-gated) + `gsv-http-stand-smoke` | `tests/gsv_stand_smoke_contracts.rs` | І | `--features stand_smoke` → **9/9** |
| 39 | Bins-родини ops: `gsv-server`, `gsv-mcp`, `gsv-watchdog`, `gsv-loc-audit`, `gsv-http-stand-smoke`, `gsv-ui-probe`, + test bins (62+) | `src/bin/` | І | cargo + focused bins |
| 40 | README/docs canon (hero SVGs, install, cards list) | `docs/`, `ui/` | І | `tests/gsv_readme_contracts.rs` |
| 41 | mesh-wasm (hub-side mesh ops + grants) | `crates/mesh-wasm/` | І | `tests/gsv_wgpu_contracts.rs` (hub lane) |
| 42 | fm-xlink (FM↔tickets advisory cross-link report: `cargo xtask fm-xlink`; `--strict` for CI) | `boxes/fm_xlink.rs`, `bin/gsv_xtask.rs` | І (read-only, advisory — poolAI FM is taто-plan, this only reports id-resolution of `fm_ref` in GSV tickets) | inline unit + `tests/gsv_xtask_contracts.rs` |
| 43 | Artifact-backed WGPU weight lane: 122/122 huge MoE tensors range-read from the 21 GB 4-bit artifact, no weight duplication (4698 artifact-source keys) | `boxes/wgpu.rs`, `boxes/wgpu_dequant.rs`, `boxes/weight_store.rs` | І | `tests/gsv_wgpu_contracts.rs` + `boxes::wgpu_dequant` unit tests; canaries g-721/g-730/g-736 |
| 44 | Q4_K + Q6_K streaming dequant with golden parity vs rusty-engine oracle (0.0072112083 / 0.017808557); compact `x_spec` tasks + `MAX_TASKS=32` + `spawn_blocking`/`Semaphore(2)` | `boxes/wgpu_dequant.rs` (7 unit tests), `boxes/wgpu.rs` | І | `boxes::wgpu_dequant` 7 tests + 65 wgpu tests, clippy `-D warnings` green |
| 45 | Durable peer identity `wgpu_peer_ids.json` (device_key→peer_id, never pruned; stable IDs/logging/metrics across reconnect) | `boxes/wgpu.rs` (`peer_join`), `data/wgpu_peer_ids.json` | І | `tests/gsv_wgpu_contracts.rs::peer_rejoin_after_prune_keeps_durable_id` + reuse test |
| 46 | Versioned 4-bit MoE companion in mirror `publish.json` (`gsv-versus-ai-full-q4k.gguf` 733 tensors, sha256 `2f37d609…`), canonical F32 unchanged; three Q4_K vision companions (2048/2304/1024) bound as versioned set | `docs/gsv/research/GASV_G20_ARTIFACT_WEIGHT_LANE_2026-11-22.md`, mirror publish-set | І (docs/evidence) | `docs/gsv/research/GASV_G15_FITNESS_SCORECARD_2026-11-22.md` + G20–G23, G34/G39 evidence |
| 47 | Vision-block forward orchestrator: result bus `GET /api/wgpu/task/result?group=<id>` + `cargo xtask vision-forward` (all linear layers via `weights_matvec`, hub keeps norms/softmax/GELU) | `boxes/wgpu.rs`, `boxes/vision_forward.rs`, `server/mod.rs`, `bin/gsv_xtask.rs` | І | `tests/gsv_wgpu_contracts.rs::group_results_returns_ordered_verified_partials`; full 23/23 (557 ops, 2304-dim) + 27/27 (653 ops, 2048-dim) forwards |
| 48 | Real PNG image input (`flate2` decoder, patchify per manifest) + full-vector export `--out` with `gsv.vision-prefix.v1` manifest | `boxes/image_input.rs`, `boxes/vision_forward.rs` | І | unit `decodes_rgb_png_and_patchifies`; live 1-block PNG forward 29 ops / 2304-dim, 252 s |
| 49 | Checkpoint/resume `gsv.vision-checkpoint.v1` (`--checkpoint/--resume`) + dropped-partial retry + scheduled resume automation | `boxes/vision_forward.rs` (`save_checkpoint`/`load_checkpoint`/`matvec_inner`), `aut_20e65ced2001Kt1aKXZ0aZaSpk` | І | unit checkpoint roundtrip; live stages 4/8/12/15/18, checkpoint `next_block=18/27` |
| 50 | Multi-token image prefix (`--image-tokens N`) — K × 1024-dim projector outputs in `gsv.vision-prefix.v1` | `boxes/vision_forward.rs`, `bin/gsv_xtask.rs` | І | 4-token smoke 119 cluster ops / 1018 c; full 27/27 4-token vector `vision_prefix_1024_mt4_full.f32` (sha `bbdcf2f9…`) |
| 51 | `cargo xtask vision-chat` image→cluster→text orchestration + `--prompt-file` | `bin/gsv_xtask.rs`, `boxes/vision_forward.rs` | І | live: 119 ops, 1018 c, answer "In the image, there is a man wearing a black shirt…" (not grounded) |
| 52 | Firefox mesh-worker OOM guards (GPU cache destroy before retry, WASM linear-memory bounds + cache clear, null-adapter diagnostic) | `boxes/wgpu.rs` PAGE string | І | `gsv_wgpu_contracts` 13/13, `gsv_ui_contracts` 33/33; page deployed via `/api/update/apply` |
| 53 | 4-bit MoE numerics forward FWD-1…8: router/top-8 → gate/up/SwiGLU/down → combine; layer-in-alias fix; concrete SwiGLU activation to down; multi-layer `moe-forward-all --layers N --start-layer --pack`, per-layer checkpoint, zero-padded 512→2048 handoff | `boxes/moe_forward.rs`, `bin/gsv_xtask.rs` | І | canary `3.47e-18` · 1-layer `6.94e-18` · 2-layer proof PASS (227 c); 40-layer environmental-blocked (2 flapping peers, concurrency=1). **NUMERICS ONLY, G13 blocked** |
| 54 | Expert-major pack + raw chunk lane LOAD-0…6: 4096-aligned `gate Q4_K|up Q4_K|down Q6_K` + sidecar sha256 manifest; 40×522 190 848 B = 20 ГБ на L:; bounded 256 MiB chunk cache; coalescing helper; octet-stream GET + `x-gsv-key`; multi-layer path --pack | `boxes/moe_pack.rs`, `boxes/weight_store.rs`, `server/mod.rs:1042`, `server/serve.rs:152` | І | `moe-pack --layer N` · `moe-forward --pack`; A/B pack 101 c vs GGUF 103 c, обидва `max_err 5.35e-11`, біт-ідентичний output |
| 55 | Adaptive claim lease SCALE-1a (30 s <8 MiB / 300 s mid / 1200 s wide) + PEER-1 sticky-route cleanup on partial-less / ok:false result | `boxes/wgpu.rs:371,1389,1609` | І | inline `scale1a_claim_ttl_three_tiers`; 13/13 wgpu contract; 40-layer stale-slot recovery 300→30 c |

## 2. Гейти (2026-11-27 steady state)

- Toolchain: rustc/cargo **1.100.0** stable (`stable-x86_64-pc-windows-gnu` pin on 11 products; host-agnostic `stable` on adoreble_wifi/mesh-node; omniroute no pin, policy-down). `RUSTUP_TOOLCHAIN` unset.
- `cargo xtask record-rust` — **warnings=0 errors=0** @ HEAD `51b1512`.
- `cargo clippy --all-targets -- -D warnings` — green (65 wgpu + 7 dequant tests green after GASV track).
- `cargo xtask record-speed --skip-run` — ok (останній повний `cargo test` 518s, exit 0).
- `cargo fmt --all -- --check` — ok.
- `gsv-loc-audit --stretch-96` → **99.68%** (виміряно 2026-11-27; rust 76888 / product 77138; wasm 0%, ui_html 250; Rust-first ≥ 95% ✅).
- `cargo test -q` → **52 suites / 0 failed**; `cargo xtask rules-check` → **5/5**; `cargo xtask sync` rev **516**, next **PH-S3129**.
- `cargo xtask mirrors --check` → **ok** (canon `.agents/skills` == `.cursor/skills` + `.opencode/skills`).
- `cargo xtask fingerprint-recheck` → advisory: historical `unknown_model`/`missing_band` rows 148–150 (2026-11-15), `missing_band` on `eae0003` (fingerprint без `--band`), 1 duplicate line 89 (0.232.0). Не hard gate; не чіпаємо без owner/drain-кроку.
- Vision rev (canonical) — **516**, next **PH-S3129**; `sync` green.
- FWD/LOAD canaries: per-expert `3.47e-18`, 1-layer `6.94e-18`, pack/GGUF A/B `5.35e-11` (біт-ідентичний output), 2-layer proof PASS; SCALE-1a 3-tier TTL; PEER-1 partial-less guard. NUMERICS ONLY — жодного quality-claim (G13 blocked).
- 4-bit MoE canaries: g-721 `6.94e-18`, g-730 `3.47e-18`, g-736 `1.73e-18` (all < 1e-4); 122/122 huge experts addressable.
- Q4_K vision canaries g-739 `1.39e-17`, g-740 `6.94e-18`; full vision forwards 23/23 (557 cluster ops, 2304-dim) and 27/27 (653 ops, 2048-dim, real PNG). No runnable-full-LLM/quality claim (G13 blocked).

## 3. Дублі / межі

- Ніколи: inference engine (інференс живе тільки в llama-rs / rusty-engine), FM-журнал (живе у poolAI), Telegram bot host (живе в telenetis).
- Свідомо розділено: D1/D2/D5 (див. майстер §3).
- 35B dispatch: GSV `weights_matvec` маршрутизація → engine виконує (rope recompute на практиці через hub probe).

## 4. Owner-decision queue (GSV-side, 2026-11-27)

1. **G13 — runnable full LLM**: потрібен owner-approved runtime target/RAM node + infer/quality gate; 4-bit MoE artifact готовий, але quality/infer не заявляється.
2. **RAM/EDR** — Defender exclusion `target/live/gsv-server.exe` або розвантаження box (historical hub падіння); зараз стабільно ~30–45 MB RSS після G22/G23 fix.
3. **Vision rev mirror rule** (GSV 516 vs poolAI 499), next PH-S3129.
4. **P4 protocol federation** (FM↔tickets deep-link через hub).
5. **Plugin remotes**: `gsv-wgpu` (HEAD `9c65f59`) і `mesh-node` (`08cf81b`) не мають remote — створити `github.com/platinoff/<name>` і запушити (owner gate).
6. **omniroute** — policy-down (`:20128`), не піднімати без owner.
7. Поверхня host-ага (куплена одиниця): раскачування backup/sync кодексу — не в GSV.
8. PoolAI CI runner — поза цим репо; коли з'явиться — GSV не блокується.
9. **G19** (Astra-style weight patch) — не стартувати: G17 не знайшов безпечного кандидата (ability-gap, не obstacle-gap).
10. **Image grounding (2026-11-26)**: всі кластерні смуги завершені (23/23, 27/27, 27/27 × 4 токени), automation видалено. Повний 4-токенний вектор дає вироджений текст — потрібне **навчене вирівнювання vision-токенів до embedding-простору MiniCPM 1B** (PH-1204) + G13; PH-1205 multimodal wire готовий, реальні image bytes ще не інтегровані. G19 — не стартувати. Жодного image→answer/quality-claim.
11. **SCALE-1 (in-memory expert registry)** — не стартувати без реального ≥10 active пірів; SCALE-1a (30 s TTL) вже зроблено.
12. **FWD-8 межа** — 40-layer безперервно environmental (2 flapping peers, concurrency=1, single JSONL slot), не код; numerics only.

## 5. Як агенти користуються

- Будь-яка нова функція GSV → новий рядок §1 з посиланням на тест.
- `cargo xtask sync` — після зміни `docs/gsv/*` синхронізує vision board.
- FM-перелік — вхідна точка для аудиту перед PR; tests — джерело правди, README — реклама.
