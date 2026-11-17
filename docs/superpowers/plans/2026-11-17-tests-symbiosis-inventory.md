# Test inventory — 2026-11-17 (symbiosis P6.0)

**Mono-flux rule:** counts measured live on 2026-11-17 (MSYS2 bash; `grep -c` on `#[test]`/`#[tokio::test]` in `tests/` and `src/`). Where precise totals are needed, cargo test is the authority; this document is a map, not a diff.

## Per-repo snapshot

| repo | `tests/` files | LOC in `tests/` | test attrs in `tests/` | inline `#[cfg(test)]` modules | shape |
|---|---|---|---|---|---|
| **GSV** | 33 | 14 657 | 546 | 64 | 498 lib + 546 contract; `cargo test` = exit 0 (ELAPSED=654s) |
| **poolAI** | 372 | 51 964 | 1 322 | 184 | 316 integration + 14 audit + 9 tests + 7 contracts; FM-каталог групи: galaxy (157), gpu (15), tenant/monitoring/vm/sso (10 each) |
| llama-rs | 1 | 104 | 6 | 14 | integration.rs + inline mods (sampling/context/safe API) |
| rusty-engine | 9 | 2 595 | 40 | 30 | issue/phase-gated (PH-S*); MiniCPM/cores; served circuit |
| telenetis | 3 | 975 | 12 | 35 | integration_test + phone_emulator + phone_tour; Mini App contracts |
| rebook | 0 (tests inline) | — | — | 41 | content/EPUB + lib tests only |
| LinFS | 1 (`crates/linfs-fs`) + fixtures | — | 14 | 0 | linfs-fs integration tests only |
| ORR_DESKTOP | 0 | — | — | 12 | inline mods (FFmpeg pipeline prone) |
| sport-forcast | 0 | — | — | 7 | inline mods (forecast kernels) |
| CoreBoard_Labs_shop | 2 | 418 | 11 | 19 | e2e/payment contracts |
| mesh-node | 0 | — | — | — | `selftest` CLI subcommand; no cargo tests (aarch64 Pi) |
| GSV-WGPU | 1 | 52 | 4 | 6 | MC-2 plan tests |

## Target vs canon (symbiosis «тато/мама»)

| expectation from canon | GSV | poolAI | llama-rs | rusty-engine | telenetis | others |
|---|---|---|---|---|---|---|
| `/api/` live hub vs mocks | live :9999 + `gsv-http-stand-smoke` | own stand/mocks + OpenAPI PCR | `--port` own; no live ports hit | `:8090/:8092` heartbeat | `:9800` health/snapshot | mixed |
| tickets board path | `gsv_tickets*` direct + hub HTTP | FM↔GSV tickets pointer | — | — | gsv_tickets through hub | — |
| MCP 60/60 | `gsv_*` tests per tool group | — | — | — | health probe only | — |
| `products --state/--wiring` | `xtask` asserted in contracts | `xtask` run in CI lane expected | n/a | n/a | n/a | watchdog: `telenetis_alive` |
| vision rev lockstep | CI gate `--check` asserts rev | `poolai-vision-sync --check` pre-push | — | — | — | — |
| ratio ≥ 96 stretch | yes | advisory ≥ 95 flat hold | — | — | — | — |

## Rot audit — що виглядає *не* вражати нової арки (кандидат на перепис)

**2026-11-17 (phase 2 виконано):** GSV `cargo test` повністю зелений, ELAPSED=490s після A1 feature-gate. Аудит усіх 33 контрактних файлів + рядкових стати повір низькі. Сканування рускі складні:
- `tests/*.rs` містять **синтетичні фікстури** для всіх version/rank/level/port/URL-констант — жоден не стверджує живстановлений стан live-хабів. Приклади: `gsv_keep_live_contracts.rs` фікстура `version: "0.223.0"`, `gsv_fingerprint_contracts.rs` фікстура `git_head: "6ef8cd1"`, `gsv_ranks_contracts.rs` `LADDER/MAX_LEVEL/MIN_LEVEL` — все це розраховують властивість, не benchmarks живого стану.
- `gsv_server_contracts.rs:149-151` *перевірка класифікації host string* (`127.0.0.1:9999` → `loopback`, `192.168.2.238:9999` → `lan`) — це перевірка власної логіки parse_host, не прив'язка до live-хабу.
- `gsv_apk_contracts.rs:65-66` — заглядає на `http://192.168.2.238:8091/api/v1` (expect err) та `:9999/api/edge` (expect ok) — error-path фікстурнезоване parity.
- `gsv_mcp_contracts.rs:643/698/1726` — включають `http://127.0.0.1:9999/mcp` як cross-check виявлення потенційного leak; pass.- `gsv_security_contracts.rs:82/259`, `gsv_settings_contracts.rs:112`, `gsv_telegram_contracts.rs`, `gsv_tickets_contracts.rs:285`, `gsv_stand_smoke_contracts.rs:215` — всі використаний synthetic host URL fixture.
- Vision rev never asserted directly — лише `vision_api_*` перевіряє `revision > 0` гарантією.
- **Zero legacy-port checks (8765, Hyper-V), zero User-scope MCP assertions**, zero hardcoded session state.

**Вихід:** GSV тести пережили перетворення у напрямку «router oneshot + mock data» без єдиного протухлого ассерту. P6.1 phase 2 audit verdict: 0 red flags, 0 stale tests.
- **WSP/evaluator rates:** `rusty-engine` tests PH-S* — потрібні; `llama-rs` + `telenetis` невеличкі — лишити.
- **Ratio gate «виношування»:** на цьому боксі чистити only after P6.0/P6.1 grün; не рухати до зростання.

## Фікстури, що займають диск
- poolAI `tests/fixtures/` — EPUB/WS/prompt фікстури; живуть у git, не чіпати без окремого вироку (регрес ризик).
- GSV test binaries у `target/debug` — очищено 2026-11-16 (cargo xtask disk janitor); інкрементальні збірки вартістю RAM — не розгортати без потреби.

## Порядок переробки (canonical cross-links)
1. **P6.1 (GSV):** нормалізувати за контрактом: hub/server, mesh/`wgpu`, mcp, ui cards, vision/xp, xtask. Прибираємо дублюючі mocks; ratio не гірше 96 stretch.
2. **P6.2 (poolAI):** зонально по FM-модулях; тільки CI/окремий вузол. Стартуємо з `orchestrator` і `raid`.
3. **P6.3 plugins:** llama-rs/engine/comm/telenetis — свої providers; вимикаємо GSV-ass або live-port awaits.
4. **P6.4 dead:** видалення лише після P6.0–P6.3 зеленого на їх MB; ratio записується перед/після.
5. **P6.5:** exit gate.

## Спалює грань (НЕ чіпаємо)
- `RATIO` ≥ 95 formal; ≥ 96 stretch.
- `.pytest/jest` — нуль (у цьому домені: no Python e2e counterparty).
- poolAI FM↔GSV tickets перевірки — тільки після P2 (виконано 2026-11-17).
