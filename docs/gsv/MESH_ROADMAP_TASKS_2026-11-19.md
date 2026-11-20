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
| A1 | **SESSION_PRESET.md** — стислий операторський пресет (100–200 рядків): як підняти хаб, як з’єднатись з bifrost, команди health/mesh/console/telemetry, чекліст відновлення після падіння — **✅ done 2026-11-19** | S | no | `docs/gsv/INDEX.md` |
| A2 | **INDEX.md refresh** — `SESSION_PRESET.md` у живі документи — **✅ done 2026-11-19** | S | no | A1 |
| A3 | **k8s-план → OBSOLETE banner** — `2026-11-19-mesh-k8s-maxload-plan.md` позначено застарілим (Pi dead) — **✅ done 2026-11-19** | S | no | — |
| A4 | **HANDOFF декомпресія** — історичні блоки (2026-10-24 стан · band 239–244 · історія боксів 109–126) винесено в `docs/archive/2026-11/HANDOFF_ARCHIVE_2026-11-19.md`; HANDOFF 450 → **117 рядків**, лишається «Сталий стан + черга» — **✅ done 2026-11-19** | M | no | A2 |
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
| B4 | **Heartbeat тюнінг** — push mesh-metrics cadence 2 s → **5 s** за замовчуванням (env `GSV_WGPU_WS_PUSH_SECS` 1..=30) — **✅ done 2026-11-19** (менше контеншн на 2.4 GHz + менше навантаження на хаб; reversible флагом) | S | no | B1 |
| B5 | **Auto re-register після рестарту** — перевірити: `device_key` відновлюється, `peer_id` #38 не мігрує після reload сторінки | S | no | B1 |
| B6 | **Дзеркало 35B-чанків у phone cache** — IndexedDB LRU 128 MiB вже є (R3); перевірити hit-rate після 24 h soak; за потреби підняти ліміт | M | no | B1 |
| B7 | **RSSI/екранний стан** — телефон не дає доступ до системи; найближче: `navigator.getBattery()` + NetworkInformation (`effectiveType`, `downlink`, `rtt`) вже в телеметрії → винести в /sys peer-рядок | S | no | telemetry вже логує |

## C. Автоматизація операційної діяльності (Rust-first, без нових сервісів)

| # | Завдання | Зусилля | Gate | Залежності |
|---|---|---|---|---|
| C1 | **`cargo xtask status`** — один рядок: hub health · version · mesh alive/ready(alive) ids · tasks done · console last line · RAM/disk (`--json`, `--url`) — **✅ done 2026-11-19** | S | no | `src/xtask.rs` |
| C2 | **`cargo xtask alive-watch`** — probe `/api/health` N разів (`--every N`, `--count N`, default 1 probe); rc 0/1; stdout only (scheduler-friendly) — **✅ done 2026-11-19** | S | no | C1 |
| C3 | **`cargo xtask cycle-watch`** — консольний watchdog reconnect-storm-ів телефона: ring `/api/wgpu/console` + rolling 24 h стан (`data/cycle_watch_state.json`), пороги `--hourly 10` / `--ten-min 3` / `--blackout-secs 180` / `--quiet-secs 900`, rate-limit алертів 30 хв; за замовчуванням **dry** (друкує, нічого не надсилає), `--alert` → Telegram у **Godfather-канал** через `sendMessage` (без Telenetis) — **✅ done 2026-11-19**. `cargo xtask cycle-watch-install` — периодичний запуск: **HKCU Run `GSV-cycle-watch`** (`"target\debug\gsv-xtask.exe" cycle-watch --every 300 --count 0`, вічний loop, на логон), доступний усі (try_schtasks → access denied, без admin) — жодної системної зміни, лише персональний registry — **✅ done 2026-11-19**. lib 541 | M | done | console ring |
| C4 | **`cargo xtask report`** — markdown-звіт із `sys_metrics_history.jsonl`: CPU/RAM avg/min/max, disk trend, tasks_done/failed Δ (counter-reset-safe), per-peer alive/ready share, done Δ, proofs fresh, EWMA avg; `--days N`, stdout або `--out` — **✅ done 2026-11-19**; перший зразок у `docs/gsv/reports/MESH_REPORT_2026-11-19.md` | M | no | `sys-history` |
| C6 | **EWMA-парітет** — `ewma_elapsed_ms` + `proofs_fresh` вже в `/api/sys/metrics` + /sys UI ewma-колонка (d83c35e); статус-чесність перевірено живцем (report відображає `ready_alive`-share) | S | no | d83c35e |
| C5 | **Dispatch gate аудит** — пройдено `wgpu.rs`: enqueue (`restrict_ready = is_heavyweight`, ranked alive∧proof_gate; single-shard `best_peer_for_task().filter(ready_of)`) ↔ `peer_poll` (heavyweight відкидається для не-ready пірів + `readiness_hint`; `weights_matvec_block` — лише browser-класи; sticky route) ↔ `proof_gate` (fresh→true; stale+fresh-alive→ghost demoted; нічого fresh ніде → any-proof pass, no lockout). **Дір не знайдено; fail направлений у `readiness_hint`, не у мовчазний "застряг" — ✅ verified 2026-11-19** | S | no | wgpu.rs |
| C7 | **Stand-smoke у CI** — GitHub workflow `gsv`: запуск `gsv-http-stand-smoke` проти тимчасового хаба (skip якщо репозиторій не може підняти `:9999`) | M | no | stand-smoke bin |

## D. Mesh-hardening з одним телефоном (біфрост як єдиний worker)

| # | Завдання | Зусилля | Gate | Залежності |
|---|---|---|---|---|
| D1 | **Golden-shard canary** — periodic (owner-triggered) `weights_matvec` blk.5 shexp на #38, звірка `max_err_model ≤ 1e-6`; дельта → лог, не більше | S | owner для змін dispatch | L: mirror |
| D2 | **40/40 re-verify одним peer** — прогін усіх blk-shardів тільки через #38 (доказ single-phone 35B транспорту); ~35–40 хв, батарея | M | owner (power) | D1 |
| D3 | **EWMA single-peer sanity** — додано тест `single_peer_router_degenerates_cleanly` (tier-0 single peer chosen; tier-3 probation routable; saturated → None; dead → None; no panics) — **✅ done 2026-11-19** (ділення на нуль/NaN-key не існує в коді — sort використовує `unwrap_or(Equal)`; зафіксовано тест-контрактом) | S | no | wgpu.rs |
| D4 | **IndexedDB eviction audit** — 128 MiB LRU переповнення → чанки витісняються; логувати eviction count; якщо > N/добу → docs-нотатка | S | no | B6 |
| D5 | **L: mirror integrity sweep** — sha256 усіх файлів у `L:/gsv_mirror/wgpu_weights/chunks/` проти `index.json`: **31/31 chunks OK** (size + sha256 == filename, 116 433 397 B), `MIRROR-OK`; loose `blk*_f32` (поза wgpu_weights) — без manifest, не перевірялись; **✅ done 2026-11-19** (нюанс: зараз 31 chunk, не 127 — дзеркалo тримуровано/LRU після 35B-розселення) | M | no | L: drive |
| D6 | **Proof TTL review** — з одним peer «proof_gate» стає single-point-of-failure для heavyweight; задокументувати деградацію (fail-open для probes, fail-loud для heavyweight) | S | no | watchdog/wgpu |
| D7 | **Failover документовано** — відсутність peers → hub віддає 503 на `/api/wgpu/task/poll` з чесним тілом; UI показує «no workers», без зависання | S | no | server code |

## E. poolAI CI / а11y (зовнішній трек, не GSV-код)

| # | Завдання | Зусилля | Gate | Залежності |
|---|---|---|---|---|
| E1 | **Run 1711 моніторинг** — виконано 2026-11-19: Pa11y WCAG ✅ green (a11y-фікс верифіковано), Playwright admin E2E ❌ failure («Build poolai and run smoke tests», логи 403 anonymous); ubuntu ✅, windows ✅ (cloud-sdk green), Док. 1418 ✅. Деталі: `ECOSYSTEM_CI_REGISTRY` §6.1 — **✅ done 2026-11-19** | S | no | GitHub |
| E2 | **a11y.yml вручну / poolAI CI re-run** — якщо Playwright E2E знову червоний: authorized re-run або `workflow_dispatch` a11y.yml (owner-клац) | S | owner | E1 |
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

1. **A1–A3** (сьогодні, docs) → commit `docs: ...` → push — **✅ зроблено**
2. **B1** (запис трашшінгу) → висновок B2/B4 → дрібний код-фікс якщо підтверджено — **✅ зроблено (B4)**
3. **C1 + C2** (xtask status/alive-watch) → commit — **✅ зроблено**
4. **D3** (single-peer EWMA unit-тест) → commit — **✅ зроблено**
5. **D1** (golden canary, owner) або ~~A4~~ — **A4 ✅ зроблено 2026-11-19**.
6. **E1** — перевірено 2026-11-19 (Pa11y ✅; Playwright ❌ — owner, див. §6.1 реєстру).

## Критерії готовності кожного пункту

- Код → `cargo fmt` + `cargo clippy --all-targets` + `cargo test` (exit 0) + `--stretch-96` для GSV-репо.
- Docs → посилання живі, `cargo xtask sync` (vision lockstep) не ламається.
- Один commit на пункт; push тільки після зеленого gate.
