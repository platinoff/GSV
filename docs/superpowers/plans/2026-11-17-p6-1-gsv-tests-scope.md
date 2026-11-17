# P6.1 — GSV tests: scope & normalize plan (symbiosis)

**Driven by:** [`2026-11-17-tests-symbiosis.md`](./2026-11-17-tests-symbiosis.md) · **Inventory:** [`2026-11-17-tests-symbiosis-inventory.md`](./2026-11-17-tests-symbiosis-inventory.md)

## Фактична структура (2026-11-17)

**Canon-паттерн підтверджений:** 29/33 контрактних файлів GSV вже йдуть через `tower::ServiceExt::oneshot` (рутер без порту) — це правильна ідея, не треба її переписувати.

| Сім'я | Тест-файли | Стиль | Зовнішні залежності |
|-------|-----------|-------|---------------------|
| Hub HTTP smoke/API | apk, edge, grid, mds, omni, products, server, settings, sli, terminal, toolchain, usage, omni | oneshot | none |
| MCP | mcp_contracts | oneshot against `POST /mcp` | hub-free |
| Tickets/Ranks/Board | tickets, ranks, hooks | oneshot + direct box fns | none |
| Vision/XP | vision_contracts | oneshot (mirror of poolAI vision) | vision manifest in-tree |
| Watchdog/keep-live | watchdog, keep_live | oneshot + heartbeat temp files | disk only |
| Stand smoke | stand_smoke_contracts | `cargo build` of `gsv-http-stand-smoke` bin | **live hub** (optional gate) |
| UI/PIP/docs-text | readme, ui, sw, toolchain | oneshot/parse file | none |
| Infra/settings | ratio, fingerprint, toolchain, readme | direct fn | none |
| Edge/apk | apk, edge | oneshot | none |
| Slate | hooks, update_flow, products | oneshot | none |

**Непотрібних файлів-контестувань:** stand_smoke_contracts (дублює `gsv-http-stand-smoke` сам по собі на іншій lane; вимагає бінарник, що будуємо тільки для smoke).

## Основні ризики (наступна черга)

1. **49 тестових контрактів дзеркалять poolAI структуру** — після симбіозу (FM↔tickets, vision rev carry-on) деякі очікування протухнуть; особливо `vision_contracts` (mirror poolAI manifest) після FM rev 499 merge.
2. **Дублююча макетизація**: APK/Edge/omni mock однаково формуються з mini-даних; консолідувати в один `tests/harness.rs` лише після упевненості в зелений базовий стан (exit 0 вже є).
3. **Мутабельність ratio**: будь-яке видалення тестів може вдарити ratio. Спочатку скорочуй *просто дублюючі*, потім перерозбирай покриття.

## Фази P6.1

1. **Freeze baseline** — збережено exit 0 сьогодні (ELAPSED=654s); `test_gate.rs` фактично існує як завжди A' environment.
2. **Contract family audit**: inventory-документ (`2026-11-17-tests-symbiosis-inventory.md`) затиснув; проводимо відсічку протухлих ассертів наступно.
3. **Live-hub tests guard**: **виконано 2026-11-17 (A1, owner 2026-11-17)** — `gsv_stand_smoke_contracts.rs` фіче-гейтнутий через `#![cfg(feature = "stand_smoke")]`, default `cargo test` повністю пропускає файл; `cargo test --features stand_smoke --test gsv_stand_smoke_contracts` → **9 passed, 0 failed**; `Cargo.toml [features] stand_smoke = []`; header doc цього пояснює. Це і є відповідь на відкрите питання зі списку (див. §Відкриті питання 1 нижче).
4. **Dedup mocks**: склеїти omni/edge/grid mocks в один спільний helper only after green baseline, with one commit and ratio check before/after.
5. **Vision gate**: GSV PR для vision rev використовує `--check`; тести, що прописують конкретний rev у файлах очікування — переписати на "rev ≥ фіксованої межі" або "rev рівний manifest".

## Ворота

- `cargo test` exit 0 на кожній фазі (зокрема, default `cargo test` без `--features stand_smoke`)
- ratio stretch ≥ 96; formal ≥ 95
- stand-smoke 87/87 через `cargo test --features stand_smoke --test gsv_stand_smoke_contracts` та `cargo run --bin gsv-http-stand-smoke -- --base-url http://192.168.2.238:9999`

## Закриті розв'язки (2026-11-17)
1. **stand_smoke_contracts feature-gate** — **зроблено (e1b37ee)**: default `cargo test` пропускає; `--features stand_smoke` → 9 passed.
2. **Vision rev mirror** — відкрито; потрібен owner-decree: GSV canon rev 516 проти poolAI rev 499 — правило "rev == manifest" чи "rev ≥ фіксованої межі".
