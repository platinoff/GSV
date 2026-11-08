# Plan: OpenCode free-token ECO drain (Flint 40B / GLM 5.3 / DeepSeek 1017)

**Date:** 2026-11-08 · **Product:** gsv (kit) + any registered plugin via Ask  
**Canon:** [`../../gsv/GSV_OPENCODE_FREE_AGENTS.md`](../../gsv/GSV_OPENCODE_FREE_AGENTS.md)

## Goal

Дати free Zen-моделям у OpenCode виконувати екосистему й продуктові тікети без Cursor paid pool: presence → claim → drain → one commit → (push якщо дозволено) → summary.

## Non-goals

- Піднімати owner-gated infra (`:8091`, tunnels).
- Rebook KDP / `workspace/books` з хаб-борду.
- Реалізація WSP у коді (лише ECO-9.1 docs feasibility).

## Steps

1. Wire OpenCode на `S:/rust/GSV/opencode.json` (stdio live `gsv-mcp.exe`).
2. Обери модель: Flint 40B → GLM 5.3 → DeepSeek 1017 → Zen free fallbacks.
3. `gsv_disk` / `gsv_keep_live` / `gsv_products` / `gsv_tickets_presence`.
4. Claim **один** open ECO/PH тікет (не RB-\*).
5. Виконай acceptance тікета; gates з `PRODUCTS.md` для продукту.
6. Один commit у product repo; push лише за тікетом/owner.
7. `gsv_tickets_done` + short Godfather/chat summary.

## Acceptance

- Free-агент може закрити ≥1 ECO тікет end-to-end без Cursor tokens.
- RB-\* відсутні як `open` на хаб-борді (cancelled).
- HANDOFF/NEXT відображають реальний keep-live + чергу ECO-8.5→7.3→8.2→8.3→9.1.
