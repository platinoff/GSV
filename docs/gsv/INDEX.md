# docs/gsv — навігаційний індекс (порядок, 2026-11-19)

**Мета:** один файл, звідки видно що де лежить. Не переносимо історичні файли (там посилань багато), а даємо карту.

## Живі документи (читати першими)

| Файл | Що це |
|---|---|
| [`../HANDOFF_NEW_SESSION.md`](../HANDOFF_NEW_SESSION.md) | Сесійний HANDOFF — актуальний стан усієї екосистеми |
| [`../NEXT_SESSION_PROMPT.md`](../NEXT_SESSION_PROMPT.md) | Промпт для нової сесії |
| [`MESH_CAPACITY_2026-11-19.md`](MESH_CAPACITY_2026-11-19.md) | Виміри каналів + бенчмарк меша + тюнінг-рекомендації |
| [`ECOSYSTEM_CI_REGISTRY_2026-11-18.md`](ECOSYSTEM_CI_REGISTRY_2026-11-18.md) | CI-реєстр по всіх репо (poolAI детально, решта — станом) |
| [`ECOSYSTEM_2026_11_18_PLAN.md`](ECOSYSTEM_2026_11_18_PLAN.md) | P0/P1 план екосистеми |
| [`PRODUCTS.md`](PRODUCTS.md) | Реєстр продуктів/плагінів (drain-кандидатів) |
| [`GSV_AGI_PATH.md`](GSV_AGI_PATH.md) | AGI-path: GSV як хаб, плагіни як супутники |
| [`GSV_MCP_OPENBOT.md`](GSV_MCP_OPENBOT.md) | MCP-поверх (tools/resources/prompts) |
| [`GSV_TECH_ROADMAP.md`](GSV_TECH_ROADMAP.md) | Band-roadmap GSV |
| [`GSV_OPENCODE_FREE_AGENTS.md`](GSV_OPENCODE_FREE_AGENTS.md) | Free-token агенти (Flint/GLM/DeepSeek) + правила |

## Порядок усередині `docs/`

| Шлях | Призначення |
|---|---|
| `docs/gsv/` | GSV-kit документи, реєстри, плани, висновки (живі) |
| `docs/gsv/wsp-battery/` | WSP battery v1 + аудит/контролі/канаря (закрито) |
| `docs/superpowers/plans/` | Плани-всякі (adoreble-wifi, poolAI WSP, …) |
| `docs/archive/2026-11/` | Архів плоских доків (архів-смуга 2026-11) |
| `docs/vision/` | Vision JSON-снепшоти (lockstep з `cargo xtask sync`) |

## Свіжі акценти (тиждень)

- **adoreble_wifi** — ADO-0…ADO-3 live (хотспот GSV-Peers-6442, QR-ofбординг, /sys WAN/LAN).
- **poolAI CI** — ubuntu cloud-sdk non-blocking (винесено на windows); pa11y a11y WCAG 2.2 AA фікси; Documentation 1416/1417 green.
- **GSV /sys** — disk scale/speed read probe, WAN Wi-Fi row, LAN rows, peer mini-bars, telemetry 5 s, mesh fresh-row bugfix.
- **mesh** — усі піри alive/ready; #37 зробив перші задачі; маршрутизація EWMA працює.

## Правила порядку

1. Нова жива сесія → оновити `HANDOFF_NEW_SESSION.md` + цей індекс, якщо документи додано/перейменовано.
2. Архівні/одноразові висновки → `docs/archive/<YYYY-MM>/` за дати, не видаляти.
3. Плани-всякі зберігаються у `docs/superpowers/plans/` з датою в назві.
