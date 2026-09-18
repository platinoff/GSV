# Два телефони в чаті — ранбук сесії (після зеленого емулятора)

**Не шлях воркера:** Mini App / Chrome tensor (Host tests, Start worker, KVM
скріни) — костиль. Phone peer = **Rust-ratio APK** (`cargo run --bin gsv-apk --
register --json`, `origin=apk_edge`). Telegram лише проксі до APK. Диск/settings
хаб читає JSON (`gsv-apk --json`), не PNG. Нижче Mini App кроки лишаються як
probe path (b).

Передумова: `cargo test --test phone_emulator` зелений. Інакше телефони тикатимуть всліпу.

## Передумови на хості

- Live `:9800` піднято (`cargo xtask telenetis-live`), health `ok`.
- **Live свіжий, не дрейфований**: `GET /api/edge/tracker/status` ≠ 404 і
  `/static/tensor.js` містить `trackerUrl` (T11.1: live-бінарь оновлюється тільки
  респауном супервізора з debug — респаун тільки у вікні власника, не посеред сесії).
- Телефони і хост: або один Wi-Fi (LAN-URL з `/app`), або тунель (повільний — тільки для кнопок, великі файли качати по LAN).
- `TELENETIS_BOT_TOKEN` бойовий (після rotate-тікету), бот відповідає на `/start` в приваті.
- Відкрита ця сторінка спостерігача на хості: `GET /api/edge/tracker/status` (рой формується — видно).

## Телефон A (A54, WebGPU)

1. Відкрий Mini App → `/probe` → Run. Очікуєш: SUPPORTED + mode core.
2. На `/tensor` увімкни **Host tests** (галочка згоди — без неї хост не тестить).
3. Хост тисне **Ping** на `/devices` → телефон виконує `test_ping` (без моделі);
   відповідь видно в лозі `/tensor`, задача закривається в черзі.
4. `/tensor` → у моделі Download → дочекайся done → **автосейв сам покладе файл у Download** (дивись лог сторінки).
3. File… не чіпай (це шлях B). Use → модель в движку → Start.
4. Пришли хосту: скрін лога + tok/s self-test.

## Телефон B (Redmi 9, Mali)

1. `/probe` → Run. Очікуєш: SUPPORTED + mode **compatibility** (або UNSUPPORTED — теж результат, записати).
2. `/tensor` → Download меншої моделі (0.5B) → done → автосейв у Download.
3. **Видали кеш перевірки**: закрий Mini App повністю, відкрий знову → має бути авто-Use з кешу (лог "restoring saved model").
4. **Перевірка накопичувача**: очисти кеш браузера/дані WebView → відкрий → кешу нема → File… → вибери GGUF з Download → Use без скачування.
5. Use → Start. Пришли хосту tok/s.

## P2P-вимір (обидва з моделями, воркери запущені)

1. На хості дивись `/api/edge/tracker/status`: має з'явитись 1 swarm, `peers: 2`.
2. На сторінках `/tensor` в лозі шукай `via torrent · peers N` (замість `via http`).
3. Швидкість і паузи: Pause → Resume (докачка), Cancel (скидання).
4. Пришли хосту: скріни логів обох + `peers` зі status + чи рій розпався після done.

## Що прислати назад (мінімум для дебага)

- `peers` з `/api/edge/tracker/status` до/після.
- Хто що качає: `GET /api/edge/downloads` на хості (IP, файл, байти, Range).
- Рядки лога `torrent …`, `auto-saved …`, `restoring saved model`, `cache evicted`.
- tok/s self-test кожного.
- Hub-профілі (`class=webgpu`, mode) — з'явились чи ні.

## KVM-доступ хоста (ADB, одноразовий сетап)

Хост вміє ADB (`target/adb/platform-tools/adb.exe`). **Піддослідний кролик:**
Redmi 9 Mali, `product=galahad_global` `model=M2004J19C`, LAN **`192.168.2.89`**,
екран 1080×2340 @440dpi. Ping з `/devices` зараз іде в bound peer **`a54-01`**
(той самий Telegram id), не в окремий `redmi-01`. Агент грає з GSV-хаба
(`:9999` + Telenetis `:9800` + `/api/edge`).

**Wi-Fi debug 2026-09-17 ~20:43:** paired `192.168.2.89:37643` code `349305`
(guid `adb-f83c90810410-LQiMeQ`). Connect-порт потім став **`192.168.2.89:33223`**
(mdns `_adb-tls-connect`; `:46147` після reconnect відмовив). `plati@PLATINOV`
Currently connected. Екран 1080×2340 @440dpi.

KVM-сесія Mini App/Chrome tensor **не шлях**: модель у Chrome — костиль.
Phone client = **Rust-ratio APK**; Telegram = проксі/проброс до APK; Wi-Fi ADB
лишається debug/disk/settings. Контракт на хабі: `cargo run --bin gsv-apk --
register --json` (`origin=apk_edge`). Тікети `hub-apk-client`. T27.2 / T23.1 blocked.

Щоб знову підключитись після disconnect: Wireless debugging лишай увімкненим;
connect-порт дивись на екрані (зараз `:33223`). Пейринг уже є (`plati@PLATINOV`).

Без бездротового налагодження (стара прошивка) — той же результат кабелем USB.

## Траблшутинг

| Симптом | Дія |
|---|---|
| Тунель повільний, закачка висить | Той же Wi-Fi → відкрий LAN-URL з `/app` |
| Кнопки мовчать / 403 | Mini App старий хендшейк — закрий і перевідкрий (reopen-підказка) |
| Кеш порожній після чистки | Так і має бути — File… з Download, качати заново не треба |
| `via http` замість torrent | Трекер не бачить рій — перевір `/api/edge/tracker/status`; нема рою — логи сюди |
| test_ping з'їдено (`llama_edge: ... ignored` у результаті) | Виправлено в T25 (requeue замість complete). Якщо бачиш знову — логи полерів сюди |
| Save мовчить (нічого не відбувається) | 1) Модель з host-каталогу, не HF (HF cross-origin — one-tap нема). 2) Той же URL в Chrome на телефоні: качає → WebView ковтає завантаження, качай в Chrome, File… підхопить. 3) Перевір файл у Download вручну |
| WebGPU UNSUPPORTED на Redmi | Норма для старого WebView — CPU fallback, записати і йти далі |
