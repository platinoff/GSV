# Plan: poolAI Weight-Space Programming (WSP) — pre-engineering feasibility

**Date:** 2026-11-08 · **Product:** `poolai` · **Ticket:** ECO-9.1  
**Related:** rusty-engine (GGUF MoE), llama-rs (llama.cpp), Solana adapter / grid  
**Brief (plugin):** `S:/rust/poolAI/docs/development/WSP_WEIGHT_SPACE_PROGRAMMING_BRIEF.md`  
**Inspiration:** MIT CSAIL weight-space bugfix via targeted ablation (looping / behavioral patches without full retrain).

## Goal

Пре-інжиніринг (валідація гіпотези): чи доцільно інтегрувати **WSP** у розподілену poolAI grid як заміну або гібрид до класичного RL / on-policy Decision (NanoJev-like one-pass policy) для **точкового фіксу ваг менших моделей**.

## Non-goals (ECO-9.1)

- Не писати production patcher у `rusty-engine` / `llama-rs`.
- Не змінювати Solana on-chain контракти.
- Не комітити ваги / GGUF / book content.
- Не замінювати RL у коді «наосліп» — лише feasibility + рекомендований гібрид.

## Deliverable format (строго)

Агент (GLM 5.3 preferred; Flint 40B ok) відповідає **технічною мовою** і закінчує:

```
- ЙМОВІРНІСТЬ УСПІХУ (в %): …
- КРИТИЧНІ БЛОКЕРИ: …
- РЕКОМЕНДОВАНИЙ ГІБРИДНИЙ СЦЕНАРІЙ ДЛЯ ПІДТРИМКИ ПЛАНУ РОЗРОБКИ: …
```

Записати вердикт у:
1. кінець цього файлу під `## Verdict (ECO-9.1)`  
2. короткий абзац у poolAI brief  
3. тікет ECO-9.1 → `done` з % і blockers; якщо % ≥ owner-порогу (default 55) — відкрити ECO-9.2 (arch sketch only).

## Agent prompt (copy-paste / claim body)

Ти — провідний AI-архітектор та експерт із Mechanistic Interpretability. Наша мета — провести пре-інжиніринг (валідацію гіпотези) для впровадження концепту Weight-Space Programming (WSP), натхненного експериментом MIT CSAIL (виправлення багів зациклювання через точкову абляцію ваг).

Ми хочемо оцінити доцільність інтеграції цього підходу в нашу розподілену grid-систему "poolAI" на Rust.

Контекст нашої екосистеми:
1. Архітектура: Децентралізована pool grid система для шарингу AI-потужностей та майнінгу на блокчейні Solana.
2. Інференс-стек (Rust): Кастомний MoE-двигун (GGUF-first) "rusty-engine", а також обгортка "llama-rs" навколо llama.cpp. Цільові моделі підтримують українську мову, що критично важливо при аналізі структури ембеддінгів та токенізаторів.
3. Поточний підхід до адаптації: Розглядається заміна або гібридизація класичного навчання (RL / On-policy адаптація політик прийняття рішень в один прохід, схожих на NanoJev / Decision Models) на інтервенції в простір ваг (WSP).

Твоє завдання — провести критичний аналіз ймовірності успіху (Feasibility Study) за наступними пунктами:

1. КОНЦЕПТУАЛЬНИЙ МАТЧИНГ ТА МУЛЬТИМОВНІСТЬ:
- Наскільки математично виправдане перенесення WSP на Decision-моделі типу NanoJev, які видають розподіл ймовірностей за один forward-pass (без текстових циклів)?
- Як підтримка української мови моделями в "rusty-engine" та "llama-rs" впливає на топологію ваг? Чи існує ризик, що WSP-абляція аномалій випадково зламає мовні/семантичні кластери в MoE-експертах (MoE experts)?

2. ОБМЕЖЕННЯ ФОРМАТІВ ТА ЗАЛІЗА В RUST:
- Наш двигун "rusty-engine" орієнтований на GGUF. Оскільки GGUF використовує блочне квантування (наприклад, Q4_K_M або Q8_0), наскільки технічно можливо проводити хірургічну зміну поодиноких FP16/FP32 параметрів без повного деквантування тензора? Які будуть накладні витрати (overhead) по пам'яті та CPU/GPU на льоту?

3. АГЕНТСЬКИЙ ЦИКЛ ОЦІНКИ (ПЛАН ІМПЛЕМЕНТАЦІЇ):
- Якщо гіпотеза WSP-патчингу для нашого стеку життєздатна, запропонуй архітектуру p2p-оркестрації: як ноди пулу можуть фіксувати аномалії активацій, пакувати мікро-патчі ваг (дельта-вектори) та валідувати їх через Solana-мережу/p2p без повної зупинки інференсу двигуна.

Видай вердикт у форматі:
- ЙМОВІРНІСТЬ УСПІХУ (в %): [Твоя оцінка]
- КРИТИЧНІ БЛОКЕРИ: [Що зробить імплементацію неможливою]
- РЕКОМЕНДОВАНИЙ ГІБРИДНИЙ СЦЕНАРІЙ ДЛЯ ПІДТРИМКИ ПЛАНУ РОЗРОБКИ: [Як схрестити це з поточним RL/майнінг пайплайном, якщо пряма заміна неможлива]

Відповідай строго технічною мовою, без вступних фраз та «води».

## Evidence sources (read before judging)

| Area | Path |
|------|------|
| Job / mining layer | `S:/rust/poolAI/docs/development/JOB_LAYER_CONCEPT_2026-03-17.md` |
| Grid protocol | `S:/rust/poolAI/docs/development/GRID_PROTOCOL_CONCEPT_2026-04-06.md` |
| Solana adapter | `S:/rust/poolAI/docs/development/SOLANA_ADAPTER_CONCEPT_2026-04-06.md` |
| rusty-engine HANDOFF | `S:/rust/rusty-engine/docs/HANDOFF_NEW_SESSION.md` |
| llama-rs pin / patches | `S:/rust/llama-rs` + ticket ECO-8.2 |
| GGUF / quant kernels | `S:/rust/rusty-engine` (`src/` kquant / forward) |

## Acceptance

- [ ] Вердикт з % / blockers / hybrid у цьому файлі
- [ ] Brief у poolAI оновлено одним абзацом
- [ ] ECO-9.1 `done`; ECO-9.2 створено лише якщо рекомендовано йти далі
- [ ] Жодних бінарних ваг у git

## Verdict (ECO-9.1)

_Pending — free OpenCode agent (GLM 5.3 / Flint 40B)._
