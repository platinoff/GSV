# Format-hygiene tickets — план (2026-11-20)

**Owner-запит:** безпека → генератори не-дозволених форматів → чистка/конверсія → тести → новий інвентар + гіт-пуші з самаріс.
**Орієнтир пріоритету:** Rust 1.100 → rust-ratio → WASM → інші дозволені (`.md/.json/.js/.mdc`).
**Не чіпаємо:** книжкові продукти rebook, метадані (`.jsonl`-рings, tickets board, fingerprints), ваги моделей (`.gguf/.bin/.safetensors`), `.env`/секрети локально.
**Shell:** MSYS2 bash. **Коміти:** один осмислений на репо, без `git add -A`, пуш + самарі.

## 0. Вхідні дані (інвентар 2026-11-20)

- **42 094** файлів поза дозволеними форматами (без `target/`, `node_modules/`, `.git/`); з них **9 287 трекаються в git**.
- Трекані по репо: omniroute **8 920** (TS/TSX-джерело Next.js), poolAI 164, GSV 36, telenetis 22, LinFS 19, rusty-engine/llama-rs 18, решта ≤17.
- Найбільші локальні (untracked+ignored): omniroute `.build/` 28 206 · llama-rs `.cmake/` ~7 000 · llama.cpp-master ~3 400 · poolAI `.tmp` ~470.
- **Безпека (факти):** `mesh-node/owner_session.key` — **NOT IGNORED** (єдиний реальний ризик; історія чиста: 0 комітів) · telenetis `.env` untracked + 0 комітів ✓ · poolAI `certs/` = лише README ✓ · adoreble `.token` ignored ✓ · `debug_key.pem` ignored ✓ · freebeat `_b64_selfie.txt` untracked/ignored ✓.
- GSV ratio-аудитор уже має винятки: `ui/*.html`, `ui/*.css`, `bin|scripts/*.sh` (`src/boxes/ratio.rs:128-137`).

## 1. Порядок виконання (тікети)

### SEC — безпека (P0, першими)

| ID | Що | Файли/дія | Гейт | Est |
|----|----|-----------|------|-----|
| **SEC-1** | `owner_session.key` не в ignore | `mesh-node/.gitignore` += `owner_session.key` (поруч з уже ignored `debug_key.pem`) | `git check-ignore owner_session.key` = ok; `git log --all -- owner_session.key` = 0 | S |
| **SEC-2** | Sweep секретів у **треканих** файлах усіх 17 репо: grep патернів (`bot_token`, `api_key`, `BEGIN PRIVATE KEY`, `ghp_`, `sk-`, `passphrase`) | звіт-тікет; будь-який хіт → окремий fix-тікет; **секрети не ехоувати** | звіт у док; 0 неприпустимих хітів або зафіксовані fix-и | M |
| **SEC-3** | GSV `data/*` staging-guard | verify `.gitignore` = `data/*` з винятком `.gitkeep` (правило AGENTS вже є) | `git status --ignored data/` показує ignored | S |
| **SEC-4** | freebeat `_b64_selfie.txt` (персональні дані, untracked) | підтвердити ignored; **owner**: видалити локально чи лишити | ignored ✓ / рішення owner | S, owner |

### GEN — генератори не-дозволених форматів → виправити на дозволені (P1)

| ID | Що | Файли/дія | Гейт | Est |
|----|----|-----------|------|-----|
| **GEN-0** | **Інвентар як Rust-інструмент** (замість ad-hoc shell): `cargo xtask inventory [--tracked] [--json]` | `src/boxes/xtask.rs` + CLI-рукав: скан дозволених/заборонених, трекані/untracked, exempt-флаги; вихід — таблиця | `cargo test` + clippy 0; арм дає той самий підсумок, що shell-прохід (42 094 / 9 287) | M |
| **GEN-1** | GSV `gitkit.rs:61` обробка `.txt` | розібратись: якщо **пише** `.txt` → писати `.md`/`.json`; якщо лише читає зовнішній ввід → задокументувати exempt | `cargo test` exit 0 | S |
| **GEN-2** | GSV skills-приклади `.sh`+`.ts` → `.md` | канон `.agents/skills/systematic-debugging/{find-polluter.sh,condition-based-waiting-example.ts}` → вбудувати в `.md` skill-док; потім `cargo xtask mirrors` (перемістити в `.cursor`/`.opencode`) | 0 `.sh`/`.ts` у skills; mirrors ok; `cargo test` | M |
| **GEN-3** | poolAI `.ps1` → Rust `cargo xtask` (пріоритет Rust) | 17 треканих: `bin/*.ps1` (10) + `scripts/*.ps1` (7). Схема: **3a** аудит посилань (CI/docs хто викликає) → **3b** порт живих у xtask-рукави `.rs` → **3c** видалити мертві; виняток лише якщо Windows-only MSVC-setup неможливий у Rust → задокументувати | poolAI `fmt + clippy + check` (тести = CI-авторитет); кожен крок окремий коміт | L, owner-confirm |
| **GEN-4** | mesh-node `.sh` (25) flash-хелпери | Pi SD-dead → **не портувати**: owner-рішення archive/delete; документацію лишити в `.md` | owner-вердикт | S, owner |
| **GEN-5** | `.jsonl` (tickets, fingerprints, wsp-battery, telenetis testmode, rusty predictions) | **НЕ ЧІПАТИ** — метадані/rings (owner-правило) | exempt-запис у EXM-1 | — |

### CLN — чистка локального (P2, untracked+ignored; жодного git-видалення продуктів)

| ID | Що | Дія | Гейт | Est |
|----|----|-----|------|-----|
| **CLN-1** | omniroute `.build/` 28 206 | `rm -rf` (регенерується `next build`) | інвентар падає на ~28k; `git status` omniroute чистий | S |
| **CLN-2** | llama-rs `.cmake/` ~7 000 | спершу: чи посилається build.rs/CI на шлях? якщо ні → `rm -rf`; якщо так → exempt + документ | llama-rs `cargo check`/build не ламається | S, owner |
| **CLN-3** | poolAI `.tmp*/artifacts` ~470 | `rm -rf` (test output) | `git status` poolAI чистий | S |
| **CLN-4** | mesh-node `images/*.img` + flash-логи (untracked) | owner: видалити чи лишити (Pi dead) | owner-вердикт | S, owner |
| **CLN-5** | GSV `target/live` скретч | routine sweep після кожного тікета | `target/live` без `.pl/.py/.bat` і тимчасових `.sh` | S |
| **CLN-6** | rebook `build/` + book art | **НЕ ЧІПАТИ** (продукти-книги) | — | — |

### CNV — заміна треканих на дозволені формати де можливо (P2)

| ID | Що | Дія | Гейт | Est |
|----|----|-----|------|-----|
| **CNV-0** | **Реєстр винятків** `docs/gsv/FORMAT_EXEMPTIONS.md` | Cargo-інфра (`.toml/.lock`), CI (`.yml` — вимога GitHub), UI-glue (`ui/*.html|css` — AGENTS + ratio.rs), метадані `.jsonl`, ваги `.gguf/.bin`, rebook-книги, vendor (llama.cpp-master), images (`.png/.jpg` — бінарні, конверсія неможлива), `.iss` installer, `.vbs` launcher, `.service`, локальний `.env` | док затверджено; всі подальші тікети звіряються з ним | S |
| **CNV-1** | omniroute 8 920 `.ts/.tsx/.mjs` | **Окремий owner-epic** (не в цьому sweep): TS-порт у Rust = тижні; D2-канон = external gateway. Тікет лише фіксує власницьке рішення: exempt-till-epic або open epic | owner-вердикт | S (фіксація) |
| **CNV-2** | `.txt` (89 треканих) | аудит: документні prose → `.md`; конфіг-приклади/фікстури, які **читає код** → exempt (EXM-1) | кожен файл аудитований; тести читаючих кодів зелені | M |
| **CNV-3** | `.mmd` (9) | mermaid → вбудувати в `.md` | 0 `.mmd` | S |
| **CNV-4** | `.http` (8) | HTTP-коли → `.md`-док (якщо IDE-client використовує — exempt) | 0 `.http` або exempt-запис | S |
| **CNV-5** | `.svg` у docs (275 треканих) | SVG (текст) → опційно інлайн у `.md`; `.png/.jpg` (86+) → **exempt** (бінарні, production output презентацій) | рішення owner по SVG; PNG не чіпаємо | S, owner |
| **CNV-6** | `.sql` (130), `.yml/.yaml` (66) | exempt через EXM-1 (migrations job-store = production output; CI/k8s вимагають YAML) | запис у EXM-1 | — |

### VER — тести, новий інвентар, пуші (фінал, коли все грін)

| ID | Що | Гейт | Est |
|----|----|------|-----|
| **VER-1** | Гейти по кожному чіпнутому репо: GSV `fmt + clippy 0 + cargo test exit 0 + loc-audit --stretch-96`; poolAI `fmt + clippy + check`; інші як чіпали `fmt+clippy+test` | всі exit 0 | M |
| **VER-2** | `cargo xtask inventory` (GEN-0) → новий список файлів по інвайронменту: дозволені / exempt / залишкові порушення | звіт `docs/gsv/FORMAT_INVENTORY_2026-11-20.md` | S |
| **VER-3** | Гіт-пуші: один коміт на репо (тільки чіпнуті файли, без `git add -A`) + самарі в чаті | кожен пуш = зелені гейти перед ним | M |
| **VER-4** | Підсумковий самарі: до/після лічильники, SEC-звіт, exempt-реєстр | самарі owner'у | S |

## 2. Правила виконання

1. **Порядок обов'язковий:** SEC → GEN-0/EXM-1 → GEN-1..4 → CLN → CNV → VER. (Безпека першими; EXM-1 до чистки, щоб нічого exempt не зачепити.)
2. Один тікет = один коміт (+ пуш одразу після зелених гейтів), самарі в чаті.
3. Пріоритет конверсій: **`.rs` (Rust 1.100, `cargo xtask`) → `.wasm` → `.md/.json/.js/.mdc`**.
4. Ніяких секретів у комітах/самарі; SEC-2 звіт без echo значень.
5. Паралельний `cargo` на один `target/` — заборонений; poolAI повний test локально не ганяємо (CI-авторитет).
6. RB/book-тікети не відкривати; rebook — лише якщо owner явно скаже.

## 3. Статус

| Група | Статус |
|-------|--------|
| SEC-1..4 | **ready** (факти зібрані) |
| GEN-0..5 | ready (GEN-3/4 — owner-confirm) |
| CLN-1..6 | ready (CLN-2/4/6 — owner) |
| CNV-0..6 | ready (CNV-1/5 — owner) |
| VER-1..4 | blocked by попередні |
