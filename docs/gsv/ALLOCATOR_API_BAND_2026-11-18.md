# Allocator API bring-up — design note (2026-11-18)

**Це design-note, не реалізація.** Розділ плану `ECOSYSTEM_2026_11_18_PLAN.md` §6 (P1.10).

## Що доступне у Rust 1.100

- `std::alloc::Allocator` trait + `System` / `Global` impls
- `Vec<T, A>`, `Box<T, A>`, `Arc<T, A>`, `Rc<T, A>` де `A: Allocator`
- `String::new_in`, `Vec::new_in`, `Vec::with_capacity_in`, `Box::new_in`
- Можливість писати власні allocator'и (наприклад, arena на `mimalloc`) без unsafe у std

## Гарячі точки нашого hub стеку

| Гаряча точка | Що можна покращити |
|---|---|
| `mesh_wire` `read_jsonl` + `serde_json::from_slice` | Borrowed slices vs String — нульові алокації на парсингу JSONL stream'ом |
| `wgpu_push_metrics` loop | `json!` body every 2 s — нульові алокації на сериалізації |
| `record_report` / `record_telemetry` | `Vec<u8>` writes — одна алокація на виклик |
| `update::record_time_series` | `String::format!` per entry — zero-alloc через `write!` |

**Примітка:** у всіх точках уже працює `mimalloc` як global allocator (channel stable gsv, band 231).
Явне `Allocator` API — опціональний бустер, не обов'язок. Спершу виміряти місця, потім код.

## Можливий хід band'у

1. Додати `crates/mesh-wasm/benches/` з микро-бенчмарком, що показує кількість алокацій на виклику.
2. Рефакторинг `mesh_wire::read_jsonl` на borrowed `&[u8]`-парсинг.
3. `record_telemetry`/`record_report` — `Vec::with_capacity_in(_)` або `String::with_capacity`.
4. Порівняти цифри бенчмарку до/після.
5. Рух у HANDOFF — один commit per repo.

## DoD band'у (коли вирішиться)

- `cargo bench --bench=deser_jsonl` — показує baseline vs after
- `cargo test` — green
- `cargo clippy --all-targets -- -D warnings` — green

## Примітки

- Поділ на Фазу 1 (виміряти + маленький експеримент) і Фазу 2 (розлити нарости).
- Не братися за це у спішці — кожен рядок коду з кастомним алокатором ускладнює future-debugging.
- Редизайн стоїть на боці `crates/mesh-wasm` тpotter на `crates/mimalloc-arena` (новий crate).
- Тестування `Vec<T, mimalloc::MiMalloc>` замість стандартного `Global` лише там, де різниця вимірювана.
