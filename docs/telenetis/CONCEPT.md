# Telenetis — концепція аплікації (ресерч 2026-09-17, абракадабра з-під GSV)

Виключно проєкт **telenetis** (`S:/rust/GSV/telenetis`, порт **9800**).
Джерела: `docs/telenetis/README.md`, `ops.md`, `src/{config,state,edge,net,tunnel,bot/telegram,gsv/client,security/initdata,ui/static/probe.js,tensor.js}`,
GSV roadmap band 208–234, llama-rs HANDOFF 2026-09-14, Telegram Mini Apps docs + WebGPU/Tokio ресерч 2026.
Тікети: `t-1789647024104829400` (створення), `t-1789647024140541300` (ресурси+фон),
`t-1789647024169324600` (hub/PoolAI/OpenBot), `t-1789647024193502000` (Rust/llama-rs).

## 1. Правильне створення аплікації в Telegram

Канон (BotFather): `/newbot` → токен → `/newapp` → title/desc → Web App URL
(плейсхолдер → потім прод-HTTPS). Два входи: Main Mini App (`t.me/{bot}/{app}`,
налаштовується в `/myapps` → Edit link) + Menu button (`/setmenubutton` або
`setChatMenuButton` — Telenetis вже ставить `web_app` Menu button з tunnel/public URL).
Групові чати `web_app`-кнопки відхиляють — там `send_url_button` на direct-link
(вже є в `bot/telegram.rs`).

Що вже правильно в Telenetis:
- `initData` HMAC-SHA256 `WebAppData` + constant-time compare + `auth_date`
  freshness (`DEFAULT_MAX_AGE_SECS = 86400`), `user` обов'язковий — збігається з каноном.
- Webhook `secret_token` (`X-Telegram-Bot-Api-Secret-Token`, 403 без нього).
- Long polling `getUpdates` (30s) як fallback без тунелю.
- Tunnel менеджер: `TELENETIS_PUBLIC_URL` > ngrok auto > LAN > loopback; Mini App
  ніколи не віддає `127.0.0.1` телефону (`mini_app_base`, `lan_url`).

Gaps → план:
1. `answerWebAppQuery` / `can_send_after` не використовуються — додати як окремий
   тікет (відправка результату назад у чат з Mini App).
2. Third-party Ed25519-валідація без токена (для зовнішніх SDK) — не потрібна зараз,
   зафіксувати як «не робимо».
3. `auth_date` вікно 24h — при довго відкритій аплікації бекенд почне 401;
   фронт має ловити 401 → «перезапусти Mini App» (задокументувати, не міняти ліміт сліпо).
4. Чеклист створення бота винести в `docs/telenetis/README.md` (BotFather кроки +
   webhook-secret + Menu button), зараз розкидано між README/ops.

## 2. Накопичувач / RAM / CPU / GPU пристроїв

Поточний стан:
- `/probe`: `navigator.gpu.requestAdapter()` → adapter info/limits + `deviceMemory`,
  `POST /api/edge/webgpu` (initData) → `POST /api/grid/profile {class: webgpu}`
  з ключем `{peer}-{adapter-slug}` (два телефони на одному TG-акаунті не затирають).
- `/tensor`: wllama **3.5.1** ESM CDN, `n_gpu_layers=99` + CPU fallback,
  Qwen2.5-1.5B дефолт / 0.5B для Redmi, OPFS відсутній на HTTP LAN → кастомний RAM-кеш
  + IndexedDB персист, pause/resume через Range, WakeLock, stall-watchdog,
  Save у Download + File-пікер (без повторного скачування), `.torrent` webseed
  (`/torrents/*.torrent`, bencode, 1 MiB pieces).

Матриця (що реально можна):
| Ресурс | Доступно з Mini App | Обмеження |
|---|---|---|
| GPU adapter/info/limits | так (`/probe`) | `powerPreference` на ноутах ігнорується (Chrome бере інтегровану); `featureLevel: compatibility` для Mali/Adreno |
| RAM (`deviceMemory`) | приблизно (округлено браузером) | не точна цифра — тільки бакет; `ram_mb: 0` ніколи не гадаємо |
| Storage OPFS | лише HTTPS + свіжі WebView | на LAN/старих — IndexedDB + Download-folder |
| CPU | через tok/s self-test | без прямого API — вимірюємо, не питаємо |
| VRAM | нема API | `vram_mb: 0`, місткість — з probe-профілю |

## 3. Нетворкінг: GSV hub + PoolAI сервіси

Карта: `phone → Telenetis :9800 → GSV :9999 / PoolAI :8091 / llama :8080`.
- GSV: bus v1 poll 5s (`/api/telegram/bus`), `/api/tickets/*` (claim/done/error/reclaim
  через `BoardAction`), `/api/grid/profile`, keep-live multi-probe.
- PoolAI: `bindings` + `seats` + `tasks/status` + `vm/*` через `PoolClient`
  (5s бюджет, bearer з relogin на 401, service account Viewer `telenetis`).
- Reverse proxy `/edge/upstream/{llama,poolai}` — allowlist, телефони не ходять напряму.
  POST-и вже закриті initData (sec follow-up done), GET-и відкриті для статусів.
- LAN-first (band 233): `GSV_LOCAL_ADDR` → LAN IP → loopback; телефони — LAN або тунель.

## 4. GSV OpenBot авто-керування + Telenetis

Протокол: kinds `sync|presence|claim|done|reclaim`, Godfather-канал, маски id
(`503…793`) у бот-відповідях, повні id лише в Mini App JSON.
Автономність Telenetis: self-presence heartbeat 60s, poll loop 5s → `push_bus` +
broadcast WS/SSE, `live_reconnect` політика на `/api/live/config`.
Відкриті рішення власника (не чіпати без нього):
- `t-1789606667836911200` group reply ownership (хто відповідає в групі).
- `t-1789606660639448500` rotate bot token (токен світився в транскрипті).
Автоматизувати далі: авто-`claim/next` підказки в `/board`, бот-повідомлення як
push коли Mini App закрита (див. §5), seat-policy бейджі.

## 5. Автономна робота якщо згорнута (ключовий висновок ресерчу)

Telegram WebView **не гарантує фон**: suspend/reload будь-коли, нема надійного
`beforeunload`, нема Service Worker на iOS, пам'ять ріжуть агресивно.
Отже «автономна робота згорнутої аплікації» **неможлива як фоновий процес** —
правильна архітектура вже закладена в Telenetis і її треба тримати:
- Інкрементальний save (localStorage + backend по ходу, не на submit).
- Resume = повна реконструкція (`/api/snapshot` + localStorage), ніколи не вірити пам'яті.
- WS — тільки поки активна; на фоні — polling при поверненні + **бот-повідомлення
  як push** (єдиний надійний out-of-band канал коли аплікація закрита).
- WakeLock тримає екран лише у відкритій `/tensor`; BackgroundFetch — тільки для
  великих файлів, не для «вічного воркера».
- `initData` старіє → 401 через ~24h → фронт каже «перевідкрий».

## 6. Rust фреймворки (2026) + що бампити

Піни Telenetis зараз: axum **0.8** (актуальний 0.8.9, MSRV 1.80), tokio 1.x full,
tower-http **0.7**, reqwest **0.13** native-tls, hmac 0.13 + sha2 0.11, chrono-tz.
Band 231/234 хвилі вже пройшли (reqwest 0.13, tower-http 0.7, askama видалена як мертва).
Нове з ресерчу: Tokio 1.52–1.53 (schedule-latency metrics, `LocalRuntime` стабільний,
io_uring ring-per-thread — unstable, нам не треба), Axum 0.9 готується (breaking:
`main` гілка) — **не стрибати**, лишаємось на 0.8.x.
План: квартальний `cargo xtask products`-інвентар дрейфу, бампи окремими хвилями
(deps → HMAC fixtures → UI-контракти), як у band 234.

## 7. llama-rs взаємодія

llama-rs HANDOFF 2026-09-14: `llama-cpp-sys-2 0.1.154`, MTP 4/5/6 green,
`llama_serve :8080` (27B Qwen3.8 mmap), `rpc` feature + ggml-rpc proto-5.0.0 pin,
heartbeat → GSV keep-live. Відкрито: bump `llama-cpp-2 → 0.1.156` (6 registry-патчів,
повний ребилд + даунтайм serve).
Зв'язка з Telenetis: `/tensor` (wllama на телефоні) ↔ poolAI `llama_chat` черга
(poll → run → complete, non-chat re-queue) через `/edge/upstream/poolai`;
PC-поллери конкурують за ту ж чергу (PoC). Torrent webseed знімає повторні скачування;
phone-to-phone P2P чекає WS-трекер (`t-1789606673625339100`).
Правило: тензори лишаються на ggml-rpc / `llama-rs/docs/DISTRIBUTED.md`, Telenetis —
control plane + browser worker, не новий протокол.

## Роадмап-пропозиції (наступні бенди, власник вирішує порядок)

1. Чеклист BotFather + `answerWebAppQuery` тікет + 401-перевідкриття в `app.js`.
2. `compatibility` probe-режим для Mali/Adreno + RAM-бакети в hub-профіль.
3. OpenBot авто-підказки (claim/next) + бот-push при закритій аплікації.
4. WS-трекер для P2P (після двох живих телефонів з моделями).
5. Квартальний framework-refresh хвилями (не стрибати на Axum 0.9).
6. Owner-gated: token rotate + group ownership (вже відкриті, блокують інше).
