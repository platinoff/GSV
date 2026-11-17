# P4 — Protocol federation FM↔tickets (plan)

**Driven by:** [`GSV_ECOSYSTEM_SYMBIOSIS.md`](../../gsv/GSV_ECOSYSTEM_SYMBIOSIS.md) (§4, P4) · owner picked this as next logical step 2026-11-18
**Companion:** [`GSV_FUNCTION_MANAGEMENT.md`](../../gsv/GSV_FUNCTION_MANAGEMENT.md) · [`2026-11-18-poolai-p6-2-rewrite-plan.md`](./2026-11-18-poolai-p6-2-rewrite-plan.md)
**Status:** Plan only — no code until owner go/no-go.

## 0. Що таке FM↔tickets federation за цим каноном

> **P4 (canon):** Глибока інтеграція — FM→ticket federation: poolAI FM rows auto-render зі статусом GSV tickets (FM читатиме дошку); poolAI keep-live advisory; розвантаження poolAI `docs/` до ≤ 30 (зроблено P1); ECO-8.2 — лише після owner-рішення (поза цим планом).

Тобто: **GSV не тягне FM**, а **poolAI FM читає GSV борд**. GSV додає лише *тонке якорне поле* в квиток + один read-only FM checker. Жодна домовленость у коді в ту сторону не йде.

## 1. Які докази вже є (2026-11-18)

- Факт ліній-цитування `t-*` у poolAI FM: рівно один (`t-1794880501219432800`, коміт `e123ef05d` — FM-row на P3 variance gate). Тобто зв'язок уже живий, але ручний.
- GSV tickets **не мають** поля `fm_ref`; `Ticket` у `src/boxes/tickets.rs` має готовий патерн для нових optional полів (`workflow`, `scenario`) — `#[serde(default, skip_serializing_if = "String::is_empty")]`.
- Hub `POST /api/tickets/claim` приймає зовнішній JSON-об'єкт (перевіряємо — claim_body schema може бути fixed, тоді додаємо `fm_ref` там самим способом).
- `products --state/--wiring` — окремий gate; `fm-xlink` — ОКРЕМИЙ advisory-чек, що не є hard-block.

## 2. Архітектура (що й куди належить)

| Шар | Що робить | Що НЕ робить |
|-----|-----------|--------------|
| **GSV hub** | зберігає `fm_ref` у квитКу; віддає список з фільтром `?fm=`; `fm-xlink --check` (read-only); MCP `gsv_tickets{fm?}` | не парсить FM prose повністю; не пише в poolAI |
| **poolAI FM** | читає `http://<lan>:9999/api/tickets?fm=FM-005` з док-власника (свій agent/autonomous action); **авто-рендер** — опція на майбутнє, не в цьому PR | не створює квитків сама; не claim-ить |
| **GSV MCP** | `gsv_tickets {fm}` — фільтр | не claim-ить по fm автоматично |

**Правило симбіозу:** посилання — завжди id-to-id (`FM-005` ↔ `t-…`). Жодного FM-тексту в GSV-комітах; жодного GSV-тексту поза id'ами в poolAI-репо.

## 3. Фази

### P4.0 — Цей документ
Ручні зв'язки; правила; жодного коду. Owner go/no-go вирішує старт P4.1.

### P4.1 — Схема `fm_ref` (GSV, мінімальний код)
- `Ticket.fm_ref: String` з `#[serde(default, skip_serializing_if = "String::is_empty")]`;
- HTTP `POST /api/tickets/claim` приймає optional `"fm"`; board rows з порожнім fm_ref — сюрприз-free;
- backward compat: старі рядки без поля десеріалізують як `""`; `scenario` вже є (`tickets[]` wiring за `ScenarioStep`) — окремий vehicle не потрібен;
- тести: один unit-тест + один contract-тест (claim with fm persists; без fm — порожній fm_ref).
- **Commit:** GSV, main (fmt → clippy → test → push) лише якщо owner-го; окрема гілка не потрібна (малий PR).

### P4.2 — Hub filter + MCP filter
- `GET /api/tickets?fm=<id>` — optional query param (без fm — поведінка як зараз);
- MCP `gsv_tickets({fm})` — optional arg (фільтр по fm_ref);
- resource `gsv://docs/fm-federation` — канон (1 стор.): формат лінків, приклади FM-row ↔ t-*;
- stand-smoke — **86/86 ціль збережено** (новий кейс не додаємо: фільтр опціональний).

### P4.3 — `cargo xtask fm-xlink --check` (advisory)
- читає `docs/gsv/tickets.jsonl` (локально або `--repo-root`);
- читає poolAI FM (`--fm-path S:/rust/poolAI/docs/catalog/FUNCTION_MANAGEMENT.md`);
- витягує FM-ID: regex `^\|\s*(FM-\d+|PH-S\d+)\b` (узгоджується з чинними FM-ids — перевірено 2026-11-18: `FM-001`, `PH-S156`, `PH-S1579`, `PH-SVC47`);
- рапортує: (a) fm_ref класу "PH-SVC-*" — поза скоупом fm check (ліцензійне лінія, якій не треба link в тикети), ok-no-op; (b) orphans — fm_ref не знайдений у FM наборі; (c) квитки з pending fm_ref (CLAIMING_BLOCKED?); exit 0 advisory; `--strict` → exit 1.

### P4.4 — Міграція ручних лінків
- Для кожного `t-*`, який уже цитується у poolAI FM (один сьогодні: `t-1794880501219432800`) → `POST /api/tickets/<id>` з `{fm: "FM-<row>"}` (new endpoint чи free-form update? Якщо немає update endpoint — P4.1 додає `POST /api/tickets/link {id, fm}`; перетворення 10-тийкета тривіальне);
- FM — нічого не міняється (poolAI відповідає за свої rows; обидва посилання мають бути узгоджені випадково пізніше, не насильно).

### P4.5 — Gate + docs close
- gate: fmt → clippy 0 → cargo test exit 0 → stand-smoke **86/86** (не змінений стан);
- оновити symbiosis doc (P4 row → design + commit hashes), PRODUCTS/NEXT/HANDOFF;
- fetch last pushed HEAD в FM pointer; products --state/--wiring зелений (mesh-node тепер `ok`).

## 4. Що НЕ робимо в P4

- **Не** генеруємо квитки з FM (one-directional ручний лінк, ніколи авто);
- **Не** імпортуємо FM-текст у GSV;
- **Не** робимо fm-xlink hard-блокером воріт (помітимо як SOFT check; exit 1 лише з `--strict`);
- **Не** чіпаємо ECO-8.2;
- **Не** чіпаємо `data/*`/`.env*`/ключі;
- **Не** переписуємо poolAI код — максимум read-only fm-xlink consumption зі своєї сторони каноном, і тільки після того, як аванси будуть в GSV-only.

## 5. Ризики

| Ризик | Міра |
|-------|------|
| GSV «зсуває» FM-семантику всередину себе | тільки `id`; жодного FM-тексту; окремий SOFT-чек |
| Ручні лінки гниють | `fm-xlink --check` advisory показує orphans на кожному run |
| FM-ids не погоджені з GSV лінками | validator повідомить; виправлення в poolAI — вручну |
| Паралельна робота на квитках | `BOARD_LOCK` уже серіалізує мутація-борду; `fm_ref` — optional поле |
| poolAI push-гуки (vision-sync) чіпляють poolAI | GSV цим не займається; poolAI-owner крок — після GSV-first commit |

## 6. Started tickets (після owner go)

- `P4.1 fm_ref schema + claim-fm` — 1 квиток (`product: gsv`);
- `P4.2 hub filter ?fm= + MCP fm` — 1 квиток;
- `P4.3 fm-xlink --check` — 1 квиток;
- `P4.4 backfill links` — 1 квиток;
- `P4.5 gate + docs` — 1 квиток (partial of P6.5).
