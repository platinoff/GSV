# Plan: Полірування + автоматизація екосистеми — 2026-11-19

**Поточний стан (виміряно 2026-11-19):**

| Складова | Стан |
|---|---|
| Hub `gsv-server` | живий, `:9999`, RAM ~90 %, TTFB 2–3 s, disk free ~65 GB |
| `#38 bifrost` (phone) | alive але з реконект-трешшінгом (`worker: hub unreachable` / `hub: connection restored` кожні n сек) — конект дуже нестабільний |
| `#35 valhall` (phone) | `last_seen ~9 год` — глибоко офлайн; коли повернеться — буде автоматично переселений як #35 знову (device_key SHA-256) |
| `#36 Pi 4` | мертва SD-картка; поки не відновимо/замінимо — Pi не в грі |
| `#37 FF-PC` | не присутня; firefox-mix оновлення не оновились після AP-перемикання |

**Чого неможливо зараз (виключено з плану):**
- k3s / k8s на Pi — мертва SD + "диски пірів не чіпаємо" (policy)
- Docker Desktop на цьому ноуті — RAM ~90 % (хаб живий), без WSL2 — фізично неможливо без серйозних системних змін
- 35B forward — нічия не тягне (RAM хаба ~6.8 GB, 35B може)
- Нові телефони / new HW

**Три власника-гейти змушені дефолтом:**
1. Автоматизація повинна працювати на 90 %+ RAM-хабі без OOM.
2. Кожен сайлентний записл дискатиться до вентиляції.
3. Жодних записів на телефоні-диску (SD), нічого більше за /tmp / log RPC.

---

## Фази плану

### Phase 1 — Санітет документації (без зміни коду)

1. **Консолідація HANDOFF** — зараз `docs/HANDOFF_NEW_SESSION.md` — 1300+ рядків з багато історичних контекстів. Записати "Preset сесії" інтерактивний док: `docs/gsv/SESSION_PRESET.md` — стислий (100–200 рядків), який оновлюється кожен 10 банд.
2. **INDEX update** — user-facing операторська панель: один page з ключовими лінками до live-ap та phone-операторської сторінки.
3. **Aктивнішка перевірка индексів** — пройти `cargo xtask sync` і переконатись що всі індекси живі, всі канонічні truth-linki не мають 404.

### Phase 2 — Телефонна стабільність (пом'якшення реконект-трashшінгу)

**Що зараз:** `#38 bifrost` постійно втрачає конект до `/api/wgpu/console` і `mesh-node` — це агресивний cellular / Wi-Fi (3G/4G, DIM 400–1800 ms) або процес-вбивця телефону (Android Doze/App standby).

**Можливі задачі:**
- Перевірити `.localStorage` Hub: чи має `peer_id` tokens, чи телефон може щоб разово пере-реєструватись автоматично.
- Потести: 30 хв passive монітор: рахувати `push channel open/close` лічильники в hub telemetry (чи постійно падає?).
- Якщо трашшінг безперервний — налагодити інший механізм push (WebSocket vs SSE топологія, чи push upgrade works on bifrost-проводці).
- Вирішити: чи треба переводити конфіг телефону на background-tasks (Chrome PWA keep-alive throttling mitigation).

### Phase 3 — Автоматизація операційної діяльності

1. **`xtask record-speed` → `recordspeed`** — зараз `record-speed` працює як bash-скрипт що викликає cargo test-ci. Зробити viewport automatized watchdog: `alive-check` (ping хаба кожні 30 с, echo якщо FAIL) — окремий bin `gsv-alive-watch` що годиться без хаба живого.
2. **`cargo xtask status`** — створити стисле резюме: alive/ready peers, перевірка AP-підключення, stand-smoke один рядок, цілі навантаження.
3. **`cycle_phone_watch`** — скрипт/xtask що: (а) перевіряє `/api/wgpu/console` кожні 5 хв, (б) в разі перевищення 10-трохи втрат хвилин — SMS/Telegram alert; (в) автоматичний restart `gsv-server` якщо watchdog-loop не спрацьовує.
4. **`xtask report`** — генератор markdown-репорту раз на тиждень: plots (lives/fail, peak RAM, disk-garbage), sprint-close, summary of cycle.

### Phase 4 — Extras (залежно від часу власника)

1. **k3s-wipe persistence** — будь-яка можлива перерегулярна підтримки Pi (після фізичного відновлення або нового картки)
2. **poolAI CI 1711** — ДОДАТИ MV-розпусти для ARM64 artifacts одночасно з x86_64
3. **EOL replay** — зберегти для кожного біну xtask його `--json` payload зробити для авто-трансформації в `"task"` JSON-pipeline
4. **Human-readable Telegram log** — prefix/out з hub telemetry на Telegram channel use slots per piece.

---

## Контрольні точки

- Усі задачі Phase 1 виконані (docs only) → commit `docs: ...` → push
- Phase 2 розв'язаний → push
- Phase 3 → push
- OK/NO-GO рішення на кожен банд

## Що нам треба від власника

- Чи потрібен Telegram alert для phone-трashшінга, з якими ключами? (Поза scope — кожен раз треба запит)
- Чи варто розпочинати Phase 1 (docs) одразу, чи спершу Phase 2 (телефон)?
