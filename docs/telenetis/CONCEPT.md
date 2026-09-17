# Telenetis — концепція (щоб не плутатись)

Проєкт: **telenetis** (`S:/rust/GSV/telenetis`, порт **9800**). Ресерч 2026-09-17.
Бенди: T1 (Mini App correctness) + T2 (compat-probe, next-hint) + T3 (сховище, фон) — done.

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
- ✅ `answerWebAppQuery`-білдер (тестований).
- 🚫 Проводка `answerWebAppQuery` — НЕ будуємо (T6.1): `query_id` є тільки в keyboard-button flow, а наші запуски (inline, menu, direct) його не дають; в коді нема reply-клавіатури — проводка була б мертвим кодом.
- ⏳ Watch Bot API 10.x (липень 2026): origin-hardening Mini App — методи аплікації блокуються з чужого origin; стежити, щоб ngrok-URL збігався з налаштованим, інакше телефони мовчки втратять TG API.
- ✅ Фронт ловить 403 від старої `auth_date` — показує «перевідкрий Mini App» (T1.2; сервер дає саме 403, не 401).
- 🚫 Third-party Ed25519-валідація без токена — не потрібна, не робимо.

## 2. Де лежать моделі: кеш vs накопичувач

**Вердикт: IndexedDB — це кеш, а не накопичувач. Накопичувач — це папка Download.**

| Де | Що | Переживає рестарт | Переживає виселення ОС |
|---|---|---|---|
| RAM (`BYTESTORE`) | активні байти | ❌ | ❌ |
| IndexedDB `tensor-worker/models` | готові GGUF (швидкий кеш) | ✅ | ❌ (best-effort; `persist()` просимо після збереження) |
| Download (auto-save T4.1, toggle) | готові GGUF (накопичувач) | ✅ | ✅ |
| File-пікер | вантажить з Download назад без скачування/кешу | — | — |
| Вибір моделі (T5.1) | пам'ятається (localStorage), авто-Use з кешу при старті; Start лишається ручним | ✅ | — |
| Чейн (T5.2) | воркер пам'ятає останні 6 ходів (localStorage); `payload.history` сідає з хоста; complete лишається answer | ✅ | — |

- ✅ Готовий Blob після verify сам падає в Download (same-origin blob-URL; toggle Auto-save on/off).
- ✅ Недокачане губиться (в кеші тільки готові файли); докачка Range + stall-watchdog — в межах сесії.
- ✅ OPFS нема на HTTP LAN; File System Access у TG WebView нема — anchor-Download єдиний чесний шлях.
- ✅ Невідоме RAM/VRAM = `0`. `compatibility`-probe done (T2.1).

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

**Вердикт: у чаті — докачка і реконект; при згорнутому Telegram — тільки сервер і бот-push.**

фон-фактчек T3.2 (код + ресерч 2026):
- ✅ Згорнута в чаті (Telegram відкритий): сокети рвуться → WS backoff-реконект + SSE-fallback (`app.js`), воркер-poll на `setTimeout` тротлиться і продовжує по поверненню, докачка Range + stall-watchdog, WakeLock перезахоплюється по `visibilitychange`.
- 🚫 Згорнутий сам Telegram (згорнутий застосунок): WebView заморожено/вбито — в телефоні не працює НІЧОГО (ні воркер, ні докачка). Правильне налаштування тут = серверна сторона: черги PoolAI чекають, Telenetis/GSV живуть, результат приходить бот-повідомленням.
- ✅ `/probe` і `/tensor` окремо: моделі в IDB/Download чекають повернення, байти не губляться.
- 🚫 Вічного фону, Service Worker, Background Sync у TG WebView — нема і не буде.

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
- ⏳ WS-трекер для phone-to-phone P2P (`t-1789606673625339100`) — T7: сервер `GET /tracker` + announce з клієнта готові; лишився вимір байтів на двох живих телефонах.
- 🚫 Новий протокол розподілу тензорів — не вигадуємо, канон `llama-rs/docs/DISTRIBUTED.md`.

## 8. Емулятор телефона (ворота перед телефонами)

**Вердикт: спочатку зелений емулятор, потім два телефони в чаті.**

- ✅ `src/emu.rs` + `tests/phone_emulator.rs`: lifecycle, disk-vs-cache,
  свіжий підпис `initData`, сценарії claim→done / stale→403 / minimize→restore /
  kill (диск живий) / два телефони через справжній `/tracker` WS / unbound-probe.
- ✅ Герметично: in-process роутер на `127.0.0.1:0`, ніяких живих `:9800`/`:9999`.
- ✅ Показовий тур `tests/phone_tour.rs` (`--nocapture`, рядки `TOUR[…]`):
  boot → порожній снапшот → hint claim → 6 сторінок 200 → claim/done ok →
  stale з маркером → unbound-probe fail-open → рій з 2 пірів. По дорозі тур
  зловив справжню гонку (статус без читання відповідей бачить 1 пір) — виправлено
  синхронізацією на stats-відповіді.
- 🚫 Емулятор НЕ покриває реальні тапи WebView, жести, системний Download-менеджер —
  це лишається за двома телефонами в чаті.

## Черга (порядок вирішує власник)

1. ⏳ token rotate + group ownership (блокують інше).
2. ✅ T1 done 2026-09-17: BotFather-чеклист + reopen-підказка (403) + `answerWebAppQuery`.
3. ✅ T2 done 2026-09-17: `compatibility`-probe + claim/next-підказки (phone-гейт окремо).
4. ⏳ WS-трекер P2P.
5. ✅ Квартальний refresh хвилями.
6. ✅ T3 done 2026-09-17: моделі на накопичувачі (IDB + Download + persist) + фон-фактчек (чат — так, згорнутий TG — тільки сервер+push).
7. ✅ T4 done 2026-09-17: IDB визнано кешем; автосейв готового Blob в Download + toggle; File-пікер назад.
8. ✅ T5 done 2026-09-17: вибір моделі липне (авто-Use з кешу); чейн у воркері (6 ходів + `payload.history`, poolAI не чіпали — payload opaque). Host-side сесії — follow-up іншим репо.
9. ✅ T6.1 done 2026-09-17: вердикт `answerWebAppQuery` (білдер так, проводка ні — нема keyboard-flow) + аудит: worker-controls код готовий (pause/cancel/44px), лишився phone-verify.
10. ✅ T7 done 2026-09-17: WS-трекер `/tracker` (announce/offer/answer relay, latin1, cap 5, 10 тестів) + announce з tensor.js (webseed fallback цілий); вимір P2P — гейт на телефонах.
11. ✅ T8 done 2026-09-17: емулятор телефона (ядро + 6 сценаріїв, 304 тести зелені); два телефони — тільки після нього.
12. ✅ T9 done 2026-09-17: `/api/edge/tracker/status` (рой видно без peer id) + ранбук `TWO_PHONES.md`; телефони йдуть по ньому.
13. ✅ T10 done 2026-09-17: показовий тур емулятора (8 кроків, гонка знайдена і вбита); телефони — наступні.
