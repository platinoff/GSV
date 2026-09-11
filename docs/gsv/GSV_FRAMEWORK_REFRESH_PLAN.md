# GSV Framework Refresh Plan (band 231 →)

Ресерч 2026-09-11: inventory (`cargo xtask`-стиль зчитування Cargo.toml × crates.io API) +
веб (Rust 1.98 release, reqwest 0.13 changelog). Тікети: `t-1789142746161269100`
(інвентар), `t-1789142748862823100` (ресерч), `t-1789142750508129800` (GSV),
`t-1789142751937166000` (telenetis), + queue per-product.

## Середовище

- rust **1.98.1** stable-gnu (оновлено цієї сесії; 1.98: `format_into` (= itoa-шwindigkeit),
  stabilized `core::range`, Send/Sync Command args; 1.98.1 = fix vtable miscompile)
- MSYS2 gcc 15.1, cmake pinned per-product; Windows-only host → **io-uring/monoio поза межами**

## Матриця версій (direct deps, ⚠ = major break)

| crate | GSV | telenetis | latest | break / міграція |
|-------|-----|-----------|--------|------------------|
| tokio | 1.49 | 1 | **1.53.1** | minor — lock refresh |
| axum | 0.8 | 0.8(+ws) | **0.8.9** | patch |
| reqwest | 0.12 ⚠ | 0.12 ⚠ | **0.13.5** | default TLS → rustls/aws-lc (**NASM-вимога, на gnu не збирається**) → `default-features=false + native-tls` (schannel, як було раніше через default-tls); `query` тепер окремий feature (5 юс. у telegram.rs); `json,stream` без змін |
| toml | 0.9 ⚠ | — | **1.1.6** | serde-шлях (`from_str`/`Value`/`to_string_pretty`) сумісний — API check пройдено |
| serde / serde_json | 1.0 | 1 | 1.0.229 / 1.0.151 | patch |
| chrono | 0.4 | 0.4 | 0.4.45 | patch |
| futures-util / tokio-stream | 0.3 / 0.1 | +futures | 0.3.34 / 0.1.19 | patch |
| tower (dev) | 0.5 | 0.5 | 0.5.3 | patch |
| tower-http | — | 0.6 | **0.7.1** ⚠ | telenetis: cors/trace/limit API стабільні між 0.6/0.7 — перевірити збіркою; відкладається на наступний тікет |
| askama | — | 0.13 | **0.16.1** ⚠⚠ | 3 мейджори, синтаксис шаблонів дрейфує — окремий тікет telenetis-UI |
| hmac / sha2 | — | 0.12/0.10 | **0.13/0.11** ⚠ | RustCrypto major wave — telenetis security, окремим кроком |
| uuid | — | 1 | 1.26.1 | patch |

Інші продукти (queue-тікети): rebook **zip 0.5→8.6 ⚠⚠** (API-rewrite),
LinFS **axum 0.7→0.8 ⚠ + windows 0.58→0.62 ⚠**, ORR (rfd 0.15→0.17 ⚠, dirs 6→7 ⚠;
windows/windows-sys/cpal/muda/tray-icon/openh264 — вже свіжі), llama-rs
(sysinfo 0.38→0.39 ⚠, clap 4.3→4.6; llama-cpp-sys 0.1.154 pinned — patch-reverify),
poolAI вже на reqwest 0.13 (є орієнтир feature-set).

## Perf-адопція (shortlist win/risk)

| Кандидат | Куди | Очікуаний виграш | Ризик | Рішення |
|----------|------|------------------|-------|---------|
| **mimalloc 0.1.52** global allocator | gsv-server + telenetis | allocator-heavy JSON wiring, −5–15% latency на cards-штучних бенчах | низький (cc+bundled C, gcc є) | **band 231** |
| rayon 1.12 | gsv-loc-audit / ratio walk (паралельні скани файлів) | −3–6× на wall LOС-audit | середній (детермізм порядку) | band 232+ |
| hashbrown 0.17 | tickets/ranks stores | дрібний | низький | skip (std вже swiss) |
| divan | benches | швидші бенчі | **ламає історію speed_index** (criterion baseline) | skip, зберегти criterion |
| PGO/BOLT (cargo-pgo) | release-бінарники | 5–15% runtime | needs llvm-profdata + окремий реліз-профіль | backlog-дослідження окремим бендом |
| std `format_into` (1.98) | hot fmt шляхи | пікі | нуль | micro-followup |

## Секвенція

1. **band 231 (ця сесія):** інвентар ✅ → ресерч ✅ → GSV (reqwest 0.13+native-tls+query,
   toml 1.1, lock refresh, mimalloc) → telenetis (reqwest 0.13, lock refresh) →
   gate fmt/clippy/test/stretch-96 + record-speed (порівняння з baseline 69s) +
   gsv-ui-probe smoke → bump 231 + fingerprint → один commit GSV + push.
2. Queue після 231 (по одному тікету, по черзі): **rebook** → **LinFS** → **ORR_DESKTOP** →
   **llama-rs** (patch-reverify) → **poolAI** (test-ci, найбільший blast radius).
3. Окремі follow-up-тікети створювати з plan-секцій (askama/hmac-wave telenetis,
   rayon loc-audit, PGO research).

## Gate-протокол кожного продукту

`cargo update` → явні major-правки → `cargo check` (латаємо API-дрейф) → fmt →
clippy 0 → tests → (loc-audit stretch-96 де є) → один commit у репо продукту + push.
No `git add -A`; Cargo.lock комітиться лише разом із мейджор-правкою в межах тікета.

## Статус хвилі 1 (2026-09-11, band 231 queue)

| Продукт | Результат | Коміт |
|---------|-----------|-------|
| GSV + telenetis | ✅ reqwest 0.13 / toml 1.1 / tokio 1.53 / axum 0.8.9 + mimalloc + tracker-race fix | `cebea81` |
| rebook | ✅ zip 0.5→**8.6.0** — API-rewrite: `FileOptions<'_, ()>` ×3 у `src/epub.rs` (writer), читач без змін; tokio 1.53; `en/` (live-книга) у .gitignore; 26 тестів + build-epub/check green (WIP приземлився: `932fca6`) | `8002b24` |
| LinFS | ✅ windows 0.58→0.62 + axum 0.7→0.8 — нуль дрейфу коду; + clippy fix (checked_div) | `c965a34` |
| ORR_DESKTOP | ✅ dirs 6→7 + rfd 0.15→0.17; решта вже свіжа. 6 `chunks_exact_to_as_chunks` → окремий тікет (panic-семантика as_chunks у encode-циклах) | `b75103f` |
| llama-rs | ✅ sysinfo 0.38→0.39 + clap 4.6 + **pin `=0.1.154`** (берігає 5 регістрових патчів); 0.1.156 — owner-gated тікет | `0f05ffa` |
| poolAI | ⛔ окремий бенд: вже reqwest 0.13 / zip 7 / rustls-ring; лишилось toml 0.9→1.1 + lock + **kube/k8s-openapi за cluster-політикою FM**; гейт — повний `cargo test-ci` | — |
