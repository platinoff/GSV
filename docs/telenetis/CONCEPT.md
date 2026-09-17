# Telenetis — концепція (щоб не плутатись)

Проєкт: **telenetis** (`S:/rust/GSV/telenetis`, порт **9800**). Ресерч 2026-09-17.
Закриті тікети: `t-1789647024104829400`, `t-1789647024140541300`,
`t-1789647024169324600`, `t-1789647024193502000`.

Легенда (стоїть перед кожним пунктом, без винятків):
- ✅ — вже є в коді, працює.
- ❌ — нема, треба зробити (з тікетом або без).
- ⏳ — чекає рішення власника, не чіпати.
- 🚫 — свідомо не робимо.

## Словник: хто є хто

| Сутність | Що це | Де живе |
|---|---|---|
| Bot | Telegram-бот (команди, кнопки, webhook/polling) | `src/bot/` |
| Mini App | WebView-сторінки всередині Telegram (`/app /board /tensor /probe`) | `src/ui/` |
| OpenBot | GSV-бот Godfather-каналу (шина `sync\|presence\|claim\|done\|reclaim`) | GSV `:9999`, не тут |
| Hub | GSV-сервер: тікети, шина, grid-профілі, keep-live | `:9999` |
| PoolAI | Координатор edge-воркерів: bindings, черги, VM | `:8091` |
| llama | Інференс: `llama_serve :8080` + телефонний wllama-воркер | llama-rs / `/tensor` |

Правило: телефони говорять **тільки** з Telenetis. Напряму в Hub/PoolAI/llama — ніколи.

## 1. Telegram-аплікація

**Вердикт: створення за каноном, бекенд-верифікація закрита, 2 gaps відкриті.**

- ✅ BotFather: `/newbot` → `/newapp` → HTTPS URL; Main Mini App (`t.me/{bot}/{app}`) + Menu button (`setChatMenuButton`).
- ✅ `initData` HMAC-SHA256 (`WebAppData`), constant-time, `auth_date` ≤24h, `user` обов'язковий (`src/security/initdata.rs`).
- ✅ Webhook `secret_token` (403 без хедера); без тунелю — long polling `getUpdates`.
- ✅ Групам `web_app`-кнопки заборонені → там `send_url_button` на direct-link.
- ✅ Телефону ніколи не віддаємо `127.0.0.1` (`mini_app_base` / `lan_url`).
- ❌ `answerWebAppQuery` / `can_send_after` не використовуємо — окремий тікет, не в цій сесії.
- ❌ Фронт не ловить 401 від старої `auth_date` — показати «перевідкрий Mini App», ліміт 24h не міняти.
- 🚫 Third-party Ed25519-валідація без токена — не потрібна, не робимо.

## 2. Ресурси пристрою

**Вердикт: міряємо те, що браузер реально віддає; цифри не вигадуємо.**

- ✅ `/probe` → adapter info/limits + `deviceMemory` → hub-профіль `class=webgpu`, ключ `{peer}-{adapter}`.
- ✅ `/tensor`: wllama 3.5.1, `n_gpu_layers=99` + CPU fallback; Qwen2.5-1.5B (дефолт) / 0.5B (Redmi).
- ✅ Кеш: RAM + IndexedDB (OPFS нема на HTTP LAN); pause/resume Range; Save в Download + File-пікер; `.torrent` webseed.
- ✅ Невідоме RAM/VRAM = `0`. Ніколи не гадаємо.
- ❌ `compatibility`-режим probe для Mali/Adreno — майбутній тікет.

## 3. Мережа: Hub + PoolAI

**Вердикт: карта зафіксована, телефони ходять тільки через reverse proxy.**

```
phone → Telenetis :9800 → Hub :9999 / PoolAI :8091 / llama :8080
```

- ✅ Hub: bus poll 5s, `/api/tickets/*` через `BoardAction`, `/api/grid/profile`, keep-live.
- ✅ PoolAI: bindings/seats/tasks/VM через `PoolClient` (бюджет 5s, relogin на 401, service account Viewer).
- ✅ `/edge/upstream/{llama,poolai}` — allowlist; POST закриті initData, GET відкриті.
- ✅ LAN-first: `GSV_LOCAL_ADDR` → LAN → loopback.

## 4. OpenBot авто-керування

**Вердикт: протокол стабільний; хто відповідає в групі — вирішує власник.**

- ✅ Kinds `sync|presence|claim|done|reclaim`; маски `503…793` у відповідях; повні id лише в Mini App JSON.
- ✅ Self-presence 60s, poll 5s → WS/SSE broadcast, reconnect-політика на `/api/live/config`.
- ⏳ `t-1789606667836911200` — хто відповідає в групі (Telenetis vs GSV-бот). Не чіпати без власника.
- ⏳ `t-1789606660639448500` — rotate bot token (світився в транскрипті). Тільки власник, ASAP.
- ❌ Авто-підказки claim/next + бот-push при закритій аплікації — після ⏳-пунктів.

## 5. Згорнута аплікація

**Вердикт: фону нема і не буде — це обмеження Telegram, не наш баг.**

- 🚫 Фоновий процес у згорнутому WebView — неможливий (suspend/reload будь-коли, нема SW на iOS, нема надійного `beforeunload`).
- ✅ Замість фону: інкрементальний save → resume як повна реконструкція (`/api/snapshot`); WS лише поки відкрита; push = бот-повідомлення; WakeLock лише у відкритій `/tensor`.

## 6. Rust-стек

**Вердикт: піни актуальні, стрибків не робимо.**

- ✅ axum 0.8.x, tokio 1.x, tower-http 0.7, reqwest 0.13 native-tls, hmac 0.13 + sha2 0.11.
- 🚫 Axum 0.9 (breaking) — не стрибати. io_uring unstable — не треба.
- ✅ Бампи — хвилями як у band 234 (deps → HMAC fixtures → UI-контракти).

## 7. llama-rs

**Вердикт: Telenetis — control plane, тензори — на стороні llama.**

- ✅ `llama_serve :8080` (27B), rpc proto-5.0.0 pin, heartbeat у keep-live.
- ✅ `/tensor` ↔ poolAI `llama_chat` черга (poll → run → complete, чуже re-queue).
- ⏳ Bump `llama-cpp-2 → 0.1.156` — відкрито в llama-rs (ребилд + даунтайм), не наше.
- ⏳ WS-трекер для phone-to-phone P2P (`t-1789606673625339100`) — треба 2 живі телефони.
- 🚫 Новий протокол розподілу тензорів — не вигадуємо, канон `llama-rs/docs/DISTRIBUTED.md`.

## Черга (порядок вирішує власник)

1. ⏳ token rotate + group ownership (блокують інше).
2. ❌ BotFather-чеклист у README + 401-перевідкриття + `answerWebAppQuery`.
3. ❌ `compatibility`-probe + авто-підказки claim/next.
4. ⏳ WS-трекер P2P.
5. ✅ Квартальний refresh хвилями.
