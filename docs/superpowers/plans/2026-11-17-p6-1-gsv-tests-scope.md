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

1. **Freeze baseline** — зберегти exit 0 (доведено сьогодні, ELAPSED=654s) як артефакт `test_gate.rs`: fail-fast має залишатись; без цього жодні зміни.
2. **Contract family audit**: виключити *тільки* тимчасово протухлі ассерти (vision rev, FM/CI-драйвові gates) — переписати на нові істини; жоден інший тест поки не чіпати.
3. **Live-hub tests guard**: `stand_smoke_contracts` зробити skippable при відсутності `:9999` (xfail/skip-if-down) або винести в окремий ручник типу `cargo stand-smoke` (бінарник вже є).
4. **Dedup mocks**: склеїти omni/edge/grid mocks в один спільний helper only after green baseline, with one commit and ratio check before/after.
5. **Vision gate**: GSV PR для vision rev використовує `--check`; тести, що прописують конкретний rev у файлах очікування — переписати на "rev ≥ фіксованої межі" або "rev рівний manifest".

## Ворота

- `cargo test` exit 0 на кожній фазі
- ratio stretch ≥ 96; formal ≥ 95
- stand-smoke 87/87 проти живого хаба після будь-яких змін

## Відкриті питання (owner)
1. Чи треба зберігати `stand_smoke_contracts` як тести чи перевести у вручну натиснутий smoke?
2. Яке правило для vision rev mirror? (GSV canon rev 516 проти poolAI rev 499; чи тепер "rev == manifest" без фіксованого числа)
