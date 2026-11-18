# Unused dependencies audit — 2026-11-18

**Метод:** parse `[dependencies]` + `[dev-dependencies]` + `[build-dependencies]` з `Cargo.toml`, grep `use <crate>` у `src/`/`tests/`/`benches/`.
**Статус після уточнення:** _док синхронізовано з реальністю після першої перевірки. `memchr` **не була задекларована** взагалі; нічого не видаляємо без машинно-підтвердженого green._

## Реальний стан (GSV)

Усі прямі залежності з `Cargo.toml` використані в `src/`
(grep підтверджує присутність `use <crate>::` або прямого імпорту в кожен):

| Crate | Використання |
|---|---|
| `base64` | weight-store chunk encoding + WS accept_key |
| `sha2` / `sha1` | chunk sha256 + WS handshake |
| `mimalloc` | global allocator у сервері |
| `rayon` | ratio/LOC audit |
| `reqwest` | OmniRouter proxy |
| `tower-http` | compression-gzip tunnel |
| `tower` | util layers (GenBody/Compression) |
| `httparse` | HTTP/1 custom loop |
| `tokio-stream` | SSE стріми |
| `futures-util` | SSE + WS-fragment streams |
| `tracing` / `tracing-subscriber` / `tracing-appender` | logging |
| `chrono` | heartbeat/lease/JSONL timestamps |
| `toml` | cargo metadata + workspace_scanner |
| `sha2` | weight_store, wgpu |
| `serde_json` | transitive |
| `tokio` | runtime |
| `axum` | web framework |
| `futures` | streams/poll |

## Як виконати

Для кожного кандідат, якого дійсно немає у `src/`:

1. Видалення рядка з `Cargo.toml`.
2. `cargo check` — green.
3. `cargo test --lib` — green.
4. Окремий commit на кожне видалення.

## Коментар

На поточний момент жодного кандидата не знаходжено — усі залежності активні.
`Cargo.lock` чистий (не містить memchr чи інших незапланованих).
