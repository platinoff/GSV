# Список завдань можливих у планах — mesh-ecosystem (2026-11-19, scope: хаб + bifrost #38)

**Owner ask 2026-11-19:** «зробити список завдань можливих у планах … біофрост буде тільки підключен» +
«потім гіт пуші з самарі відповідно».

**Контекст виміряно 2026-11-19:**

| Складова | Стан |
|---|---|
| Hub `gsv-server` | живий, `:9999`, crate 0.248.0, RAM ~88–90 %, TTFB 2–3 s |
| `#38 bifrost` (M2004J19C) | online, charging, proofs_fresh ≥ 8, battery ~76 %, **єдиний підключений peer** |
| `#35 valhall` (Galaxy A54) | ~9 год offline; повернеться — re-peer через `device_key` SHA-256 |
| `#37 FF-PC` | не присутня |
| `#36 Pi 4` | **мертва** (SD), поза грою |
| 35B ваги | blk.0…39 розселені на флот раніше (40/40 block-verified, max_err ≤ 3.5e-6); mirror на `L:/gsv_mirror/wgpu_weights` (~520 MB, 127 chunks) |

---

## Обмеження (жорсткі, з owner-рішень)

1. **Лише розв'язувані задачі** — нічого, що вимагає Pi, другого телефона, нового заліза.
2. **Жодних записів на диски пірів** (нічого в телефон, нічого в SD) — policy.
3. **RAM-хаб ~90 %** — жодна задача не сміє додавати фонового навантаження без виграшу.
4. **Wi-Fi 2.4 GHz — стелею** — dispatch-кандидати тільки такі, що переживають клієнтський таймаут.
5. Будь-яке торкання коду → `cargo fmt` → `clippy` → `cargo test` → **один commit** → push.
6. Owner-gated пункти не виконуються без явного «го» у сесії.

---

## A. Документи й консолідація (без коду, без owner)

| # | Завдання | Зусилля | Gate | Залежності |
|---|---|---|---|---|
| A1 | **SESSION_PRESET.md** — стислий операторський пресет (100–200 рядків): як підняти хаб, як з’єднатись з bifrost, команди health/mesh/console/telemetry, чекліст відновлення після падіння | S | no | `docs/gsv/INDEX.md` |
| A2 | **INDEX.md refresh** — додати `MESH_POLISH_AUTOMATION`, `MESH_ROADMAP_TASKS`, `MESH_PI_BOOT_INCIDENT`, актуальну таблицю «що де лежить» | S | no | A1 |
| A3 | **k8s-план → OBSOLETE banner** — `2026-11-19-mesh-k8s-maxload-plan.md` позначити застарілим (Pi dead) або перенести в `docs/archive/2026-11/` | S | no | — |
| A4 | **HANDOFF декомпресія** — винести історичні банди (< band 220) у `docs/archive/2026-11/`, лишити «сталий стан + черга» ≤ 300 рядків | M | no | A2 |
| A5 | **NEXT_SESSION_PROMPT** — синхронізувати з реальністю (Pi dead, тільки bifrost, hot path = AP 2.4 GHz) | S | no | A1 |

## B. Телефон #38 bifrost — стабільність конекту (ключова боліль)

Симптом: `worker: hub unreachable (retrying…)` / `hub: connection restored` кожні кілька секунд у
`/api/wgpu/console`. Можливі причини: Android Doze/App standby вбиває вкладку Chrome; 2.4 GHz congestion;
push-канал (RFC 6455) розрахований на LAN, телефон — на межі дальності AP.

| # | Завдання | Зусилля | Gate | Залежності |
|---|---|---|---|---|
| B1 | **30-хв пасивний запис** `/api/wgpu/console` + `/api/sys/metrics` (age/proofs/battery) → таблиця «тріггер / розрив / відновлення» | S | no | live хаб |
| B2 | **Push vs poll A/B** — клієнт `/wgpu` PWA: перевірити що WS-реконект з backoff працює, і чи допомагає fallback на SSE/poll кожні 5 s при розриві push | M | no | B1 |
| B3 | **PWA keep-alive гайд** у docs — Chrome → shortcut «Add to Home screen», Battery optimization → «Not optimized» для Chrome, Screen-on під час soak | S | no (інструкція), owner для реальних змін на телефоні | B1 |
| B4 | **Heartbeat тюнінг** — `wspush` ping interval (зараз 2 s metrics push): підняти до 5 s для 2.4 GHz, зменшити розрив під Doze | S | no | B1 |
| B5 | **Auto re-register після рестарту** — перевірити: `device_key` відновлюється, `peer_id` #38 не мігрує після reload сторінки | S | no | B1 |
| B6 | **Дзеркало 35B-чанків у phone cache** — IndexedDB LRU 128 MiB вже є (R3); перевірити hit-rate після 24 h soak; за потреби підняти ліміт | M | no | B1 |
| B7 | **RSSI/екранний стан** — телефон не дає доступ до системи; найближче: `navigator.getBattery()` + NetworkInformation (`effectiveType`, `downlink`, `rtt`) вже в телеметрії → винести в /sys peer-рядок | S | no | telemetry вже логує |

## C. Автоматизація операційної діяльності (Rust-first, без нових сервісів)

| # | Завдання | Зусилля | Gate | Залежності |
|---|---|---|---|---|
| C1 | **`cargo xtask status`** — один рядок: hub health · version · `/api/wgpu/mesh` alive/ready/tasks_done · last console line · disk free · RAM % | S | no | `src/xtask.rs` |
| C2 | **`cargo xtask alive-watch`** — foreground watchdog: probe `/api/health` кожні 30 s; FAIL → rc≠0 + лог у `target/live/logs/`; годиться для cron/CI | S | no | C1 |
| C3 | **`cargo xtask cycle-watch`** — раз на N хв: лічильник розривів у console ring; якщо > 10 за годину → dump у `data/cycle_watch.jsonl`; (alert — лише з owner-Telegram-ключем, інакше лише warn у лог) | M | owner для Telegram-алерту | console ring |
| C4 | **`cargo xtask report`** — тижневий markdown-звіт із `sys_metrics_history.jsonl`: cpu/ram avg, peer uptime, dispatches ok/fail, battery trend | M | no | `sys-history` |
| C5 | **Dispatch gate аудит** — пройти `wgpu.rs` і переконатись: heavyweight `weights_matvec` на не-ready peers блокується (PH-A1), probes — ні; додати тест якщо знайдено діру | S | no | wgpu.rs |
| C6 | **EWMA-парітет** — для телефону #38 EWMA-tier: переконатись що `ewma_elapsed_ms` потрапляє в `/api/sys/metrics` і UI (вже частково зроблено, d83c35e) | S | no | d83c35e |
| C7 | **Stand-smoke у CI** — GitHub workflow `gsv`: запуск `gsv-http-stand-smoke` проти тимчасового хаба (skip якщо репозиторій не може підняти `:9999`) | M | no | stand-smoke bin |

## D. Mesh-hardening з одним телефоном (біфрост як єдиний worker)

| # | Завдання | Зусилля | Gate | Залежності |
|---|---|---|---|---|
| D1 | **Golden-shard canary** — periodic (owner-triggered) `weights_matvec` blk.5 shexp на #38, звірка `max_err_model ≤ 1e-6`; дельта → лог, не більше | S | owner для змін dispatch | L: mirror |
| D2 | **40/40 re-verify одним peer** — прогін усіх blk-shardів тільки через #38 (доказ single-phone 35B транспорту); ~35–40 хв, батарея | M | owner (power) | D1 |
| D3 | **EWMA single-peer sanity** — при одному peer router вироджується; перевірити що немає ділення на нуль/паніки в tier-сортуванні; unit-тест | S | no | wgpu.rs |
| D4 | **IndexedDB eviction audit** — 128 MiB LRU переповнення → чанки витісняються; логувати eviction count; якщо > N/добу → docs-нотатка | S | no | B6 |
| D5 | **L: mirror integrity sweep** — sha256 127 chunks проти `wgpu_weights` manifest (offline, read-only); розбіжність → fail loud у лог | M | no | L: drive |
| D6 | **Proof TTL review** — з одним peer «proof_gate» стає single-point-of-failure для heavyweight; задокументувати деградацію (fail-open для probes, fail-loud для heavyweight) | S | no | watchdog/wgpu |
| D7 | **Failover документовано** — відсутність peers → hub віддає 503 на `/api/wgpu/task/poll` з чесним тілом; UI показує «no workers», без зависання | S | no | server code |

## E. poolAI CI / а11y (зовнішній трек, не GSV-код)

| # | Завдання | Зусилля | Gate | Залежності |
|---|---|---|---|---|
| E1 | **Run 1711 моніторинг** — Pa11y/Playwright WCAG 2.2 AA `#de3434` fix верифікація (run черкнутий > 6 год на GitHub runners) | S | no | GitHub |
| E2 | **a11y.yml вручну** — якщо 1711 не стартує 24 год → `workflow_dispatch` на a11y.yml (owner-клац) | S | owner | E1 |
| E3 | **Ubuntu leg** — non-blocking уже (`4ebb1e410`); лишити як є | — | done | — |
| E4 | **ECO-8.3 dep-sweep продовження** — решта плагінів по одному (LinFS, ORR, rusty-engine, sport-forcast, CoreBoard) | L | no, але по одному за сесію | ECO board |

## F. adoreble_wifi / інфра власника (плагін, не GSV)

| # | Завдання | Зусилля | Gate | Залежності |
|---|---|---|---|---|
| F1 | **ADO-7 кандидати (docs-only поки)** — battery-friendly hotspot auto-off timer; QR-сторінка status page; SLA probe → Telegram (owner-key) | — | owner | ADO-6 |
| F2 | **PRODUCTS.md row** — `adoreble_wifi` зареєстровано; підтримувати актуальним (HEAD, тесты count) | S | no | PRODUCTS.md |
| F3 | **`ado` CLI parity with UI** — `hotspot status` вже є; перевірити `--json` output для скриптів | S | no | adoreble_wifi |
| F4 | **HKCU Run entry audit** — `AdorebleWiFi` автозапуск: задокументувати як видалити (owner інструкція) | S | owner | AdorebleWiFi |

## G. WSP / research (owner-gated, не зараз)

| # | Завдання | Зусилля | Gate |
|---|---|---|---|
| G1 | WSP Phase 2 — **не відкривати** без явного owner go (Astra-стиль patch не має робочої гіпотези на MiniCPM-1B; Granite-A800M serve-розслідування не завершене) | — | owner |
| G2 | Granite-A800M serve retry на `:8085` (UCRT-dll вже скопійовані в `target/release/`) — тільки після owner go | M | owner |
| G3 | NanoJev port feasibility — лишається doc-only; Pi dead → нікуди деплоїти; skip | — | owner |

## H. Явно виключено (не пропонувати)

- k3s/k8s/Docker на Pi — **мертва SD**; policy «диски пірів не чіпаємо».
- Будь-які зміни на телефоні (ADB, flags, system settings) без owner go per action.
- 35B повний forward на хабі — RAM не тягне (7.7 GB total).
- Підняття `:8091` (OmniRoute), tunnels, User-scope MCP — owner-gated завжди.
- Реанімація Pi (reflash, edit `cmdline.txt`, нові образи) — owner-only, після фізичного доступу.

---

## Рекомендований порядок

1. **A1–A3** (сьогодні, docs) → commit `docs: ...` → push.
2. **B1** (запис трашшінгу) → висновок B2/B4 → дрібний код-фікс якщо підтверджено → commit.
3. **C1 + C2** (xtask status/alive-watch) → commit.
4. **D3** (single-peer EWMA unit-тест) → commit.
5. **D1** (golden canary, owner) або **A4** (декомпресія HANDOFF) — за настроєм власника.
6. **E1** перевірити при кожному push.

## Критерії готовності кожного пункту

- Код → `cargo fmt` + `cargo clippy --all-targets` + `cargo test` (exit 0) + `--stretch-96` для GSV-репо.
- Docs → посилання живі, `cargo xtask sync` (vision lockstep) не ламається.
- Один commit на пункт; push тільки після зеленого gate.
