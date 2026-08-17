# Ролі GSV (Galaxy StarWalker Vision VDT)

Канон ролей для проєкту GSV — окремого Rust-first репозиторію `S:\rust\GSV`.

**Точка входу VDT:** цей репо тримає спільні правила/скіли для будь-якого
зареєстрованого Rust-продукту. Відкривати Cursor на `S:\rust\GSV` (або `gsv.code-workspace`), далі
`абракадабра` питає **який продукт** дренити. Реєстр — [`gsv/PRODUCTS.md`](gsv/PRODUCTS.md).
Канон розділення kit vs product — [`gsv/GSV_VDT_KIT.md`](gsv/GSV_VDT_KIT.md).

Дзеркало історичних PoolAI rules лишається в дереві PoolAI як **продуктовий шар**
(`poolai-agent-roles.mdc`, `poolai-testing-policy.mdc`, FM). Shared kit більше не копіювати сюди.

## Ролі

| Роль | Хто | Відповідальність |
|------|-----|------------------|
| **Власник / креативний директор** | Людина | Візія (Galaxy StarWalker Vision), пріоритети, BLOCKED/Deferred, фінальний push за бажанням |
| **Оркестратор** | Головний агент Cursor / OpenCode | Звичайна сесія: один **PH-S***. **`абракадабра`:** AskQuestion продукт → S0 (**диск/clean першим**) → project scan (**warnings першими**) → drain band продукту → **один commit + `git push` + самарі в кінці** |
| **Субагенти** | Task tool | Вузькі підзадачі (explore/shell/generalPurpose); результат повертається оркестратору |

## Канон сесії (GSV)

1. **S0 — диск/git першим**: `df -h /s` → `cargo clean` якщо <5G (12G дешево) → `git fetch` у **репо продукту** → HANDOFF того продукту ([`PRODUCTS.md`](gsv/PRODUCTS.md)).
2. **Project scan — warnings першими**: clippy / diagnostics, потім roadmap продукту (`GSV_TECH_ROADMAP` або PoolAI FM §5.12).
3. **Drain**: до 10 PH-S*. **Rust-first** тести. GSV: stop `gsv-server` перед `cargo test`. **без Python**.
4. **Speeds + Rust panel**: GSV vision `gsv_speed_index.json` / `gsv_rust_diagnostics.json` (empty-tolerant); PoolAI — `record-test-ci-speed.sh` + `record-rust-diagnostics.sh`.
5. **Vision-sync**: GSV `gsv-vision-sync --check`; PoolAI `poolai-vision-sync --check`.
6. **Один commit + `git push` + самарі** в кінці сесії **в репо продукту**. **Не** mid-push. GSV GitHub remote — опційно, доки власник не додасть.

**Не делегувати:** фінальний `git push`, закриття спринту в FM §5.12, оновлення `NEXT_SESSION_PROMPT.md`, amend після push.

## Rust ratio канон (band 108)

- **Rust 95–100% / wasm 0–5% (завжди), без Python/Java.** Bins — лише `src/bin/`.
- Ratio тримаємо через `GSV/src/boxes/ratio.rs` + bin `gsv-loc-audit` (дзеркало `poolai_loc_audit.rs`):
  ```
  cargo run --bin gsv-loc-audit                 # write GSV/data/rust_ratio.json
  cargo run --bin gsv-loc-audit -- --print      # print report, no write
  cargo run --bin gsv-loc-audit -- --min-ratio 0.95 --advisory
  ```
- Gate: `rust_ratio >= 0.95` (формальна смуга). Нижче — **compact UI/CSS** (тонкий JS/DOM glue), не додавати Rust-обхід.
- JSON звіт: `GSV/data/rust_ratio.json` (gitignored, не комітимо); live UI-бейдж через `GET /api/ratio`.

## Бокси GSV (панелі/можливості)

Tracker · SLI console · Toolchain · IDE · Update · Box preview · SLI terminal ·
Tests/bench hooks · **Ratio** · **OmniRouter** (Rust AI-проксі/роутер). Канон —
[`GSV/docs/gsv/GSV_BOXES.md`](gsv/GSV_BOXES.md), архітектура —
[`GSV/docs/gsv/GSV_ARCHITECTURE.md`](gsv/GSV_ARCHITECTURE.md).

## Збірка / тести

- Terminal — **MSYS2 bash** для `cargo`/`git`; з кореня репо:
  ```
  export PATH="/c/Users/${USER}/.cargo/bin:$HOME/.cargo/bin:/ucrt64/bin:/usr/bin:$PATH"
  export RUSTUP_TOOLCHAIN="stable-x86_64-pc-windows-gnu"
  cd GSV && cargo build --all-targets && cargo test && cargo clippy --all-targets
  ```
- Запущений `gsv-server` **блокує `target/debug/gsv-server.exe`** → `cargo test`/`build` падає
  з `Access is denied (os error 5)` → спочатку зупинити сервер (PID), потім build/test.
- Роутинг: `--repo-root S:/rust/GSV --data-dir S:/rust/GSV/data --port 9999` (default);
  опційно `--repo-root S:/rust/poolAI` щоб сканувати FM / `bin/` PoolAI.
  `data/*` gitignored (секрети/API-ключі безпечні).

## Поза чергою

- **BLOCKED:** ні (band 102 повністю ✅).
- **Deferred:** Vision docs sync / migration → `GSV/docs/gsv/GSV_MIGRATION.md` (future sprints).
