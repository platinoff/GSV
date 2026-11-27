# Ecosystem repair & modernization plan — 2026-11-27

**Статус:** execution complete for every non-owner-gated item in the inventory. Кожен пункт — окремий тікет, TDD + gates + один commit + push. **Межа:** G13 blocked, жодного image→answer/quality-claim; усе MoE — numerics/orchestration.

Джерела: 3 паралельні агенти (ticket board + code markers у 16 репо; Rust 1.100/WASM/framework research; FM/roadmap/ratio drift scan). Поточні HEAD'и після виконання: GSV `git log -1` (snapshot `6adec96`), rusty-engine `468a6f2`, LinFS `5666918`, adoreble_wifi `141ede0`, omniroute `a2e1e9f` (W25–W28 були вже done — інвентар був stale). Ratio GSV: **99.68%** (76888/77138, wasm 0%, ui_html 250).

**Тікетна звірка 2026-11-27:** FWD-5 (`t-1795725845907114700`) закрито як покрите `fwd5_*` тестами + MOD-1/MOD-4/LOAD-4; GASV-F1/F2 закрито як stale (реалізовані ще `ebb7b79`, Q8_0/Q4_0 опубліковані F4 2026-11-21); PH-405/PH-407 лишаються **open** — повний обсяг (llama.cpp oracle + perplexity / tok-s baseline) не виконано, synthetic parity+bench `4082a98` є лише частковим покриттям.

**Не зроблено навмисно:** MOD-3 (`wasm-opt`, потребує `pacman -S binaryen` — системна зміна); owner-gated G13/G19/SCALE-1/remotes/`:8091`/rebook/token rotate; ADO-5 phase 2 (`nl80211`/`hostapd`) — системні зміни; poolAI VM process spawn — **deferred by design** (чесний noop `is_supported=false` + explicit error + тест; фейковий spawn був би шкідливим). MOD-5 coalescer **не wiring** через чесний A/B (9× slower).

## 1. Інвентар зламаного (топ-22, скорочено)

Категорії: **(a)** broken code · **(b)** unimplemented/ticketed · **(c)** stale doc · **(d)** owner-gated · **(e)** environmental.

| # | Repo | Де | Симптом | Кат | Рекомендація |
|---|---|---|---|---|---|
| 1 | rusty-engine | `moe.rs:359,441` | MoE на synthetic weights; real GGUF → `PH-406` | b | One ticket: wires `ExpertCache` loader до real dequant tensors |
| 2 | rusty-engine | `runner35.rs:222-224` | `runner35` dense FFN; **немає qwen35moe runner** (`PH-1206`) | b | PH-1206 branch у runner35 (dedicated node) |
| 3 | rusty-engine | `quant.rs:242` | Q5_K/Q6_K encoder "not implemented" | b | Implement або document parity-gated limit |
| 4 | rusty-engine | tickets PH-1202…1205 | vision tower/compressor/multimodal open | b | One vision band, honesty gate зберегти |
| 5 | rusty-engine | `roadmap.md:157,161` | PH-405/406/407 parity+real+bench open | b | PH-405/407 one ticket |
| 6 | LinFS ✅ | `ext4/mod.rs` | extent depth>0 / indirect → "not yet implemented" | b | **LFS-1 done** (`cb28eda`): indirect pointers [12]/[13]/[14] implemented + fail-closed reader; depth>0 extent лишається явним Unsupported |
| 7 | LinFS ✅ | `cli/main.rs` | mount/ls/fsck stubs | b | **LFS-1 done** (`cb28eda`): `anyhow::bail!` з `unsupported (band N)` замість success-looking print |
| 8 | LinFS ✅ | `block/win.rs` | Windows block ops not implemented | b | **LFS-2 done** (`9d75038`): real `seek_read`/`seek_write` + `IOCTL_DISK_GET_LENGTH_INFO` with file-metadata fallback; non-Windows explicit Unsupported; 2 Windows TDD tests |
| 9 | poolAI ✅ | `vm/mod.rs:1507`, `noop.rs:4` | VM process spawn stub / noop isolation | b | **Deferred by design (перевірено, не баг):** noop-ізолятори чесно повертають `is_supported=false`, помилка spawn eksplicitна, є тест `test_get_instance_resource_usage_not_implemented`. Фейковий spawn був би шкідливим — лишаємо як є |
| 10 | GSV | `weight_store.rs` `read_ranges_coalesced` | LOAD-3 coalescer не wired | b | MOD-5 A/B зроблено: 9× slower (sha256 dominates) → чесний verdict: не wiring, лишати primitive |
| 11 | GSV | `mesh-wasm/lib.rs:26-34` | Native tests scalar Q4_K; SIMD LUT не зроблено | b | P1 SIMD LUT + re-golden f16 `e=113` |
| 12 | adoreble_wifi ✅ | `crates/adoreble-linux/src/lib.rs` | Linux/Pi backend — honest stub, no Wi-Fi probing yet (ADO-5) | b | **ADO-5 phase 1 done** (`eb471e3`): read-only sysfs probe (`wireless/`/`phy80211` + `operstate`), pure helper + synthetic-fixture TDD (3 tests). AP через `nl80211`/`hostapd` лишається owner-gated — жодних системних змін |
| 13 | omniroute ✅ | tickets CNV-1 W25 | reasoning_fields helpers no runtime wiring | b | **Помилка інвентарю:** W25/W26/W27/W28 вже done (`a2e1e9f`); код у `omniroute-compression/src/reasoning_fields.rs` + napi/wasm exposure. Дубльований open-тікет закрито як noise |
| 14–18 | rusty/poolAI/GSV | ignored parity/k8s tests, test-ci CI-only, LAN sign-off, BENCH-2 down | — | e | CI/wired node/stabilni peers — не локально |
| 19–22 | GSV/plugins | G13, G19, SCALE-1, no remotes, `:8091`, rebook | — | d | Owner decisions only |

**Doc-only drift (не код) — ВИПРАВЛЕНО DOC-1 (`f02d9ea`) + steady-state звірка 2026-11-27 (`51b1512`):** FM §1/§2 оновлено з `12bc953`/`cc7158a` → `51b1512` (rows 43–52 з MoE/FWD/LOAD); roadmap тепер має FWD/LOAD/SCALE/PEER addendum; HANDOFF/NEXT HEAD `51b1512`; `eco_check.rs` 46→52; `scripts/README.md` має moe-*/vision-* задачі. Поточні HEAD'и у FM/roadmap/HANDOFF/NEXT звірено з реальними `git log -1` — розсинхрону немає.

## 2. Research — найкращі сучасні практики (Rust 1.100 / WASM / blazing-fast)

**Stable @ 1.100:** `Allocator` API (`Vec::new_in`), `DropGuard`, `Vec::from_fn`, `-Cprofile-sample-use` (PGO on stable), `wasm32-wasip3` Tier 2, never-type `!` fallback change. **Nightly — не планувати:** `std::simd`/portable_simd (#86656), GCA, tokio io_uring/unstable.

**Axum 0.8.9 / tokio 1.54.1 / hyper 1.12→1.13:** GSV вже використовує `Bytes`, streaming `Body::from_stream`, per-route body limits. **Безпечні оновлення:** hyper → 1.13.0 (h2 shutdown fixes для flaky phone links), bytes → 1.12.2 explicit (вже transitive), tower-http dedupe 0.7.1, `[lints.cargo]` unused-dependency warnings (1.100). axum 0.9 tuning (`TCP_NODELAY`, connection limits) — track, не backport.

**WASM (27 КБ kernel):** strict SIMD128 + JS fallback — **правильний вибір, лишати**. Relaxed SIMD — опційний другий артефакт (Safari iOS unsupported, 78.5% global). `wasm-bindgen`/component model/wasmtime — НЕ для browser kernel. **`wasm-opt` Binaryen version_133 post-build** (15–40% size reduction) — build-time tool, no Cargo dep, re-golden f16 `e=113` обов'язково.

**Blazing-fast, оцінка:** `bytes::BytesMut`/`Arc<[u8]>` ✅ (zero-copy, вже transitive); `rayon` ✅ вже; `parking_lot`/`dashmap` ⚠️ contention unproven, не rewrite; `memmap2` ⚠️ RUSTSEC-2026-0186 <0.9.11 + UB on mutable files — bounded cache безпечніша; `simd-json` ❌ (unsafe, 256 КБ bodies); `postcard 1.2.0` ⚠️ opt-in для mesh control frames, JSON лишається default; `blake3`/`xxhash` ❌ ламає content-addressed sha256 keys.

**Novi 2026 Q4 ідеї (port scheduling, не numbers):**
- **Belady/next-use eviction** (arXiv 2611.15842) — 2.1× fewer expert transfers vs LRU, 96% gain from 16-token lookahead, zero quality loss. **Highest-value для GSV** — `HotExpertSet` вже має `routing_profile`/`deterministic_hidden`, pure Rust, no deps.
- **PDScope** (2610.39216) — phase-aware prefetch (prefill aggressive / decode conservative), 2.5–10.3× prefill.
- **MoE-CORE** (2610.01950) — alternating-layer staging (38–44 ms TPOT), portable на 40×522 МБ pack.
- **Bevy** (2611.04794) — runahead of ready requests > prediction when cache present.
- **PolyStrata** — async cache refresh +4–5% decode; pin-by-need +4.4% prefill (Windows 11).
- **Pooled** — 17% experts cached serves 77% lookups — calibration для HotExpertSet.

## 3. Рангований план (fixable now → owner-gated)

| Пріоритет | Тікет | Що | Очікуваний ефект | dep |
|---|---|---|---|---|
| P0 | **DOC-1** | FM §1/§2 + companion FM, roadmap addendum, HANDOFF/NEXT HEAD+counts+canaries, eco_check 46→52, scripts/README moe-* — адаптація до `e1903ad`, ratio 99.68% | Docs істина; eco-check не бреше | — |
| P0 | **MOD-1 ✅** | Belady/next-use eviction у `HotExpertSet` + LRU fallback, Rust micro-bench (`98e7cc7`) | **Bench:** 23 vs 32 transfers = 9 avoided (~28%), 4837 ns/iter | no deps |
| P0 | **MOD-2 ✅** | Dependency currency: hyper 1.13.0, `[lints.cargo]` (`fb8a361`) | h2/chunked fixes для phone links | same 1.x line |
| P1 | **MOD-3 ⏸** | `wasm-opt` Binaryen post-build + size/ABI/golden gate | **Owner-gated:** `pacman -S binaryen` = системна зміна; не робив без go | build tool only |
| P1 | **MOD-4 ✅** | Phase-aware prefetch (`37703e3`): `moe-hotset --phase prefill` resident 8 / `--phase decode` resident 4 | prefill headroom, bit-identical | tokio вже |
| P1 | **MOD-5 ✅** | `Arc<[u8]>` у LOAD-2 cache (one allocation) + Rust A/B bench coalesced vs naive ranges (`a149130`) | **Honest verdict:** coalescer 2.74 ms vs naive 0.30 ms = **9× slower** on 8×64 KiB ranges — per-range sha256 re-hash dominates saved seeks. NOT wired into hot path; stays a primitive for a future large/contiguous pack reader. `Arc<[u8]>` kept (std only) | no new deps |
| P2 | **FIX-1 ✅** | rusty-engine PH-405/407 parity+bench + MSRV-safe decoder fixes (`4082a98`) | чесний parity gate; cache slower (22 vs 62 ms) коли все в RAM | — |
| P2 | **FIX-2 ✅** | LinFS ext4 indirect + CLI stubs (`cb28eda`): LFS-1, 2 TDD tests, fail-closed reader, depth>0 = explicit Unsupported | real filesystem read path для файлів >4 MiB | — |
| P2 | **FIX-3 ✅** | GSV mesh-wasm SIMD128 Q4_K dequant + re-golden (`ce5a4bb`): `.wasm` 27 462 → 28 035 B, live asset 28 035 B | SIMD parity with f16 `e=113` | no deps |
| Owner | — | G13, G19, SCALE-1, remotes, `:8091`, rebook, token rotate | не стартувати без go | — |
| Environmental | — | ignored parity/k8s tests, BENCH-2 down, LAN sign-off | CI/wired node/stabilni peers | — |
| Не робити | — | portable_simd, io_uring, wasmtime/component для browser, relaxed SIMD як єдиний шлях, wasm-bindgen у kernel, simd-json, blake3 замість sha256, dashmap/parking_lot rewrite | — | — |

## 4. Виконання (érica)

Кожен P0/P1/P2 — окремий тікет з TDD + gates + один commit + push. Бенчмарки: `cargo xtask record-speed` / `record-rust` (як у GSV canon), Rust micro-bench для MOD-1. Жодного quality-claim (G13 blocked).
