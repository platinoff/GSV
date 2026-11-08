# OpenCode free agents — екосистема + плагіни

**Status:** ECO-9 · **Date:** 2026-11-08  
**Audience:** моделі з **безкоштовними** Zen/Go токенами в OpenCode (і будь-який MCP-агент, що працює з `gsv_mcp_openbot`).  
**Canon host:** `S:/rust/GSV` · MCP stdio `target/live/gsv-mcp.exe` (`opencode.json`).

## Owner preferred free models (2026-11-08)

| Пріоритет | Модель (UI / Zen) | Роль |
|-----------|-------------------|------|
| 1 | **Flint 40B** (`flint40b` / Flint-40B) | оркестрація drain, Rust/docs, тікети |
| 2 | **GLM 5.3** | архітектура, feasibility, довгі плани |
| 3 | **DeepSeek 1017** (Zen free, якщо доступний) | код-фікси, вузькі патчі |
| fallback | Big Pickle · DeepSeek V4 Flash Free · Nemotron 3 Ultra Free | коли preferred на 429 |

Якщо ID у Zen UI відрізняється — бери рядок з `GET http://127.0.0.1:9999/api/omni/v1/models` / Zen picker; не вигадуй Omni `model=` для Zen-only рядків. Каталог: [`GSV_OMNI_CATALOG.md`](GSV_OMNI_CATALOG.md).

**Cooldown:** на Zen free `429` / `FreeUsageLimitError` → одразу switch на наступну preferred/fallback. Не крути той самий host.

## Обов’язковий старт сесії (кожна free-модель)

1. `gsv_health` + `gsv_disk` — якщо `free_mb < 12288` → спочатку `cargo xtask disk --clean` (або owner), **не** `cargo test`.
2. `gsv_keep_live` — зафіксуй alive/down (fail-open; не піднімай OmniRoute / `:8091` без owner).
3. `gsv_products` → **питай продукт** (не хардкодити GSV). Вікно GSV ≠ продукт.
4. `gsv_tickets_presence` → `gsv_tickets` / `gsv_tickets_next` → **один** open тікет `claim`.
5. Читай HANDOFF **того** продукту + цей файл + [`GSV_AGI_PATH.md`](GSV_AGI_PATH.md).

Shell: лише `C:\msys64\usr\bin\bash.exe -lc '…'` (не PowerShell).

## Що можна робити без owner

| Дозволено | Заборонено |
|-----------|------------|
| Claim/done ECO / PH тікетів з борду | Піднімати `:8091`, tunnels, User-scope MCP |
| Docs/plans/tickets у kit або вибраному плагіні | Комітити `workspace/`, book content (rebook) |
| Patch-level dep / clippy / тести в **claim** product | `git add -A`; секрети; parallel `cargo` на один `target/` |
| Один commit у product repo + push **лише якщо** тікет каже push або owner «пуш» | Сліпий `cargo update` на `llama-cpp-2` pin |
| CRLF-шум → `git checkout -- <file>` | Видаляти example/fixture без тікета |

## Черга (логічний порядок)

1. **ECO-8.5** poolAI clippy-debt → gates green (блокер ECO-7.3).
2. **ECO-7.3** poolAI tokio 1.49→1.53 (після 8.5).
3. **ECO-8.2** llama-rs sys bump (окремий бенд, не змішувати).
4. **ECO-8.3** dep-sweep решти плагінів (по одному продукту).
5. **ECO-9.1** poolAI WSP feasibility (план нижче) — **docs-only** до owner go на код.
6. **ECO-7.4** Rust 1.100 re-gate — не раніше **2026-11-12**.
7. Owner-gated: ECO-8.4 (`:8091`), ECO-7.7 (rebook books track), token rotate.

**Rebook book tickets (RB-\*) на хаб-борді GSV не беруть.** Книжкова робота — лише в `S:/rust/rebook` локально; не claim з free OpenCode на GSV.

## WSP / RL (poolAI)

Гіпотеза owner: замінити або гібридизувати класичний RL / on-policy Decision (NanoJev-like) на **Weight-Space Programming** (MIT CSAIL–style точкове патчення ваг) для дрібних моделей у grid.

- План + промпт дослідження: [`../superpowers/plans/2026-11-08-poolai-wsp-feasibility.md`](../superpowers/plans/2026-11-08-poolai-wsp-feasibility.md)
- PoolAI brief: `S:/rust/poolAI/docs/development/WSP_WEIGHT_SPACE_PROGRAMMING_BRIEF.md`
- Тікет: **ECO-9.1** (feasibility % + blockers + hybrid) → потім **ECO-9.2** (архітектура лише якщо 9.1 ≥ порогу owner).

Flint/GLM: виконуй 9.1 як **строго технічний** feasibility (формат вердикту в плані). Код у `rusty-engine` / `llama-rs` **не** чіпати в 9.1.

## Definition of done (free drain slice)

- Тікет `done` з коротким evidence (команди + результат).
- `git status` чистий у **тому** репо **або** один осмислений commit.
- HANDOFF/NEXT **не** обов’язкові на кожен мікро-слайс; оновлюй лише якщо змінився сталий стан (HEAD, keep-live, черга).
- Самарі в чат: product · ticket · HEAD · gates · next open.

## Anti-noise

- Не чіпай `AGENTS.md` плагінів «для порядку» — це майже завжди CRLF-шум.
- Не відкривай нові RB/book тікети на `docs/gsv/tickets.jsonl`.
- Не мішай кілька продуктів в одному коміті.
