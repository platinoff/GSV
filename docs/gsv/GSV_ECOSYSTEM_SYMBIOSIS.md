# GSV_ECOSYSTEM_SYMBIOSIS.md — екосистема як пара «тато і мама» + плагіни

**Status:** Draft for owner review (agent-drafted 2026-11-17) · **Owner:** platinoff · **Companions:** [`GSV_AGI_PATH.md`](./GSV_AGI_PATH.md) (суперхаб) · [`PRODUCTS.md`](./PRODUCTS.md) (реєстр) · [`GSV_VDT_KIT.md`](./GSV_VDT_KIT.md)

## 0. TL;DR

- **GSV = мама (операційний план, хаб).** Galaxy UI, vision board, tickets board, ranks, keep-live,
  watchdog, mesh (WASM + `/api/wgpu/*`), MCP sandbox, rolling logs, products discovery, vision docs.
  GSV **не** копіює FM poolAI; poolAI **не** друкує у GSV.
- **poolAI = тато (FM/management план, генетичний фонт коду).** Function Management journal, sprint/FM board,
  концепт inference-orchestration (edge / RAID / VM), VDT-процедури, `poolai-vision-sync`,
  WSP-план (MiniCPM 35B). poolAI не тягає Galaxy; GSV не лізе в FM.
- **Решта репозиторіїв — плагіни (діти).** Власні git-дерева, власний `target/`, власні локальні
  правила. GSV дає кров (kit, MCP, tickets, logs), poolAI дає ДНК (FM/WSP/генетичні патерни).
- Цей документ — **узгоджений стан екосистеми на 2026-11-17** + **адаптований план симбіозу і
  розвантаження poolAI** + **повторюваний протокол пошуку зламаних і дубльованих речей**.
- **Owner 2026-11-17 (flip):** poolAI = **тато** (генетичний фонт коду — перший проєкт власниці, на ньому вона вчилась);
  GSV = **мама** (хаб — керує всією екосистемою, народжує плагіни/виконавців і тримає їх живими).
  Метафору перевернуто owner'ом 2026-11-17; фактичний розподіл праці не міняється.

## 0.3 L:\rust_swap — peer swap fabric (owner 2026-11-17)

**Стан:** підготовлено кінцеву інфраструктуру; пірів читають HTTP-блоби через хаб; Pi4 вже має 2 GB zram (найкращий варіант swap для неї — RAM-based, kernel-level, SD не зачіпається).

- `L:/wmswap/rust_swap.vdi` (32 GB VirtualBox-диск-образ) — **видалено owner-власником 2026-11-17** після збігу всіх доказів того, що воно не реєстроване ні в одній живій VM (порожня `.VirtualBox/Machines/`, `VBoxManage list hdds` порожній, обидві live VM records E_ACCESSDENIED з Dec 2023; `/c/Users/plati/VirtualBox VMs/` порожня). Жодні дані не втрачені. Диск звільнився на L: (53G → 21G used, 9.9G → 42G free). `L:/wmswap/` тепер порожня; `L:/rust_swap/` зберігає растущий peer-swap ledger/cache.

| Компонент | Розташування | Статус |
|-----------|--------------|--------|
| NTFS cache | L:\rust_swap\cache | ✓ існує, порожній — чекає на імпорт блобів |
| lease | L:\rust_swap\lease | ✓ існує — сюди йтимуть TTL-оригіни; життєвий цикл — hub-side |
| ledger | L:\rust_swap\ledger | ✓ існує — JSONL audit trail |
| swap_spec | L:\rust_swap\notes\SPEC.md | ✓ опис контур заготовка |
| VDI swap | L:\wmswap\rust_swap.vdi | **видал. 2026-11-17** (VirtualBox порожній каталог; Layout оновленим рішенням використати VM-режим GSV-арки — при оголошення call-to-action — має бути прошу owner-версія; тепер зберігаємо L:\rust_swap/ як fragmented blob store) |
| Pi4 zram | /dev/zram0 (2 GB, priority 100, zstd) | ✓ активне — не чіпаємо |
| Hub blob endpoint | `POST /api/wgpu/weights/chunk` / `GET .../chunk?key=<sha256>` | ✓ вже працює — простіш від monotonic route blob exchange |

**Має рішити власник:**
- чи працювати далі на HTTP-блоб проксі як ядро для реальності storage L:\rust_swap (збагачення, не ламати)?
- чи окремий Windows-сервіс для NBD/WS-шпиг — це поза сили не адмін/користувач (м больше) чи власник сам робить (owner-rights PC or server config).

**НЕ робити:** Pi4 SD не зачіпати — RAM L2 swap достатній. NBD/NFS-сервер на цьому боксі не підняти без адміна/сервер-ліцензії; HTTP :9999 вже робить цю роботу радше саме.

**Правило:** усі тести, які виконуються на пірах, роблять це без використання їхнього диску. Пирі (Pi 4, телефони) використовуємо тільки як виконавців, не як host для збірки чи запису фікстур.

Дозволене:
- **selftest / probe sub-commands** у вже існуючих `mesh-node` бінарниках (не `cargo test` на Pi 4).
- **HTTP-тести проти хабу**: `GET /api/health`, `GET /api/sys/metrics`, `GET /api/wgpu/capability`, `GET /api/sys/history` — запити з коротким тайм-аутом.
- **mesh task execution** — dispatch/ACK воркерів через hub, без запису файлів на SD.

Заборонено:
- `cargo test` / `cargo build` на Pi 4 (SD 6.8 GB, 391 MB вільних).
- Запис фікстур, логів чи кешу на SD без явного owner go.
- `pip`/`npm install`, будь-яка інсталяція інструментів на пірів без окремого owner-рішення.

**Тести для mesh-node пишуться у `crates/mesh-node/src/main.rs` як inline `#[cfg(test)] mod tests` і виконуються на ПК/хабі (exit 0). Для делегування виконавцем — `mesh-node selftest` на Pi 4, без компіляції на ньому.**

## 0.2 Пирі не завантажувати диском (owner 2026-11-17)

**Правило:** усі тести, які виконуються на пірах, роблять це без використання їхнього диску. Пирі (Pi 4, телефони) використовуємо тільки як виконавців, не як host для збірки чи запису фікстур.

Дозволене:
- **selftest / probe sub-commands** у вже існуючих `mesh-node` бінарниках (не `cargo test` на Pi 4).
- **HTTP-тести проти хабу**: `GET /api/health`, `GET /api/sys/metrics`, `GET /api/wgpu/capability`, `GET /api/sys/history` — запити з коротким тайм-аутом.
- **mesh task execution** — dispatch/ACK воркерів через hub, без запису файлів на SD.

Заборонено:
- `cargo test` / `cargo build` на Pi 4 (SD 6.8 GB, 391 MB вільних).
- Запис фікстур, логів чи кешу на SD без явного owner go.
- `pip` / `npm install`, будь-яка інсталяція інструментів на пірів без окремого owner-рішення.

**Тести для mesh-node пишуться у `crates/mesh-node/src/main.rs` як inline `#[cfg(test)] mod tests` і виконуються на ПК/хабі (exit 0). Для делегування виконавцем — `mesh-node selftest` на Pi 4, без компіляції на ньому.**

## 0.1 Тверде обмеження флоту (owner 2026-11-17)

**Це всі вузли екосистеми — інші з'являтися не будуть.** Флот закритий:

| # | Вузол | Роль |
|---|-------|------|
| GSV `:9999` | хаб / мама (PC, 12 ядер, 7.6 GB RAM) | Galaxy UI + API + MCP + keep-live + watchdog |
| poolAI | тато (FM/менеджмент; `:8091` полісі-даун) | FM journal, менджмент плагінів |
| #35 valhall | Samsung SM-A546W (WebGPU телефон) | mesh-worker (Android) |
| #36 pi4-cpu | Raspberry Pi 4 (cortex-a72, 4 ядра, 3 GB, wlan0 192.168.2.168) | mesh + llama_serve (будуємо llama-rs) |
| #37 pc-wgpu | PC через Firefox (CPU-орієнтований, 12 ядра) | mesh (CPU/GPU test bolt) |
| #38 bifrost | Redmi M2004J19C (WebGPU телефон) | mesh-worker |
| telenetis :9800 | LAN на цьому боксі | commutation (switching) плагін |

**Наслідки (не переглядати без явного owner):**
- ~~PH-A5~~ **закрито перманентно** — dedicated ≥24 GB вузол для повного 35B forward не буде; 35B-екcперименти лишаються в межах того, що влазить (shard/partial weights: g-128..g-136 ✓; повний MoE forward — ні, і не планується).
- Плани, що бюджетують «нове залізо», — вигадані; IP-plane і внутрішню LAN (`192.168.2.0/24`) росту не буде.
- RAM екосистеми обмежений 7.6 GB PC + 3 GB Pi + RAM телефонів; боротьба за пам'ять — питання **власника** (RAM/EDR черга №1, не наша).
- mesh e2e залишається демонстрацією shared-expert shard (g-128..g-136 — доведені live), не виробничим inference.

## 0.4 Hot-swap tolerance (owner 2026-11-17)

Окремі вузли флоту **періодично відвалюються й підключаються знову** — це очікувана поведінка, не поломка. Правило для agents/тестів/інвентарів:

- **Valhall (#35 Samsung SM-A546W, Android)** — очікує періодичного хот-свопу. Втрата `alive` до 8 год — **не** привід для видалення піра, не зупиняти дрени, не піднімати алерт. Поки `ready_proofs` накопичено, після повернення `alive=true` peer переуходить автоматично (auto-rejoin + rejoin on `start`).
- **Bifrost (#34/#38 Redmi M2004J19C)** — аналогічно, періодично offline (сплячий телефон / USB відключення).
- **Pi4 (#36)** — сам хаб нетерпить жодних толерантних вимкнень; живий постійно.

**Тести/agent:**
- Не використовувати `alive=false` як *відмову* від виконання завдання; воркер має дочекатися повернення або прозивити інші готові піри.
- Не вимати автоматичного «cache-flush» пірів з реєстру тільки через вік `last_seen_age`; відмова проводиться на hub-стороні (PH-A1 dispatch gate / pre-enrolled proof), не на агентах.

**Що НЕ робити:** не відключати phones від мережі заради «зелених показників». Хаб стабільність `alive=1` не ціль — ціль: `ready_alive ≥ 2` та completion actually happening.

## 1. Фактичні стани (знято live 2026-11-17)

`cargo xtask products --state` + `--wiring` + локальні перевірки:

| id | дерево | branch | head | dirty/↑↓ | crate | зареєстровано | wiring | примітка |
|----|--------|--------|------|----------|-------|---------------|--------|----------|
| **gsv** | `S:/rust/GSV` | main | `e3fa5ec` | 0 / synced | **0.248.0** | yes (host) | n/a | **hub** `:9999`, live |
| **poolai** | `S:/rust/poolAI` | main | `964628ee9` | 0 / synced | **0.2.2** | yes (plugin) | ok | **:8091 policy-down** (ECO-8.4); HANDOFF 8.4 днів без руху |
| linfs | `S:/rust/LinFS` | main | `141aab5` | 0 / synced | — | yes | ok | — |
| telenetis | `S:/rust/telenetis` | main | `7efad09` | 0 / synced | **0.2.0** | yes | ok | `:9800` live; HANDOFF ледь присутній |
| llama-rs | `S:/rust/llama-rs` | master | `84ec9b2` | 0 / synced | 0.1.0 | yes | ok | `master`≠`main`; Pi smoke ok; llama.cpp-master/ в дереві (vendored, gitignored) |
| rebook | `S:/rust/rebook` | master | `6424975` | 0 / synced | 0.1.0 | yes | ok | — |
| orr_desktop | `S:/rust/ORR_DESKTOP` | master | `40298ab` | 0 / synced | 0.1.0 | yes | ok | — |
| omniroute | `S:/rust/omniroute` | release/v3.8.50 | `e3d202c` | 0 / synced | — | yes | ok | node, `npm test`, не Rust |
| rusty-engine | `S:/rust/rusty-engine` | main | `06f8063` | 0 / synced | 0.1.0 | yes | ok | `rusty_serve` `:8090/:8092`; WSP-трейл |
| gsv-wgpu | `S:/rust/GSV-WGPU` | main | `c381a4f` | 0 / **немає upstream** | 0.1.0 | yes | ok | лише MC-2 docs; **orphan-кандидит на архів** |
| sport-forcast | `S:/rust/sport-forcast` | main | `3d8341e` | 0 / synced | 0.1.0 | yes | ok | приватний |
| coreboard_labs_shop | `S:/rust/CoreBoard_Labs_shop` | main | `91480c2` | 0 / synced | 0.1.0 | yes | ok | приватний |
| freebeat.ai | `S:/rust/freebeat.ai` | main | `87daf9e` | 0 / synced | — | **no** | n/a | безартефактного продукту; 18д тише |
| mesh-node | `S:/rust/mesh-node` | main | `449726a` | 0 / **немає upstream** | — | **зареєстровано 2026-11-17** | n/a | Pi 4 agent; remote owner-gated |
| platinoff-profile | `S:/rust/platinoff-profile` | main | `5d81430` | 0 / synced | — | **no** | n/a | 19д тише |
| reebeat.ai | `S:/rust/reebeat.ai` | — | — | **не git-репо** | — | n/a | n/a | порожнє/лишилось після QK |
| Tokenomica | `S:/rust/Tokenomica` | — | — | **не git-репо** | — | n/a | n/a | одноразовий rust→svg конвертер usage-даних |

**Висновки живого знімання:**
- Справна історія чиста всюди (всі `0 dirty`); хаб і watchdog **живі**; RAM **750–810 MB вільних**
  (хронічний EDR/OOM pattern — **власник №1**, хаб впав тричі за сьогодні).
- Wiring зелений: всі зареєстровані → `ok` (Cursor/Grok/OpenCode configs), 3 незареєстровані → `na`.
- 0 ahead/behind скрізь, крім `mesh-node` (немає remote) і `GSV-WGPU` (немає upstream).
- **Плагінів без руху ≥ 7 днів:** poolai (7.3д), linfs (7.4д), telenetis (8.9д), rebook (8.2д),
  orr_desktop (7.4д), sport-forcast (7.4д), coreboard (7.4д), omniroute (9.9д), freebeat.ai (18д),
  platinoff-profile (18.9д) — стан «лежить», але здоровий; drain за командою власника.

## 2. Аплікації в екосистемі (що реально крутиться)

| Порт / сервіс | Власник | Стан 2026-11-17 | Призначення |
|---------------|---------|-----------------|--------------|
| `:9999` gsv-server (+ `/sys`, `/wgpu`, `/api/*`) | GSV | **live** | Galaxy UI + mesh control; `version_lag=false` |
| `gsv-mcp` (stdio `target/live/gsv-mcp.exe`) | GSV | **live** | MCP 61/61 інструментів (2026-11-24); OpenCode/Grok |
| `gsv-watchdog` | GSV | **live** (PID 24292) | самопідняття хаба; auto-sys-history |
| `:9999` loopback portproxy (iphlpsvc) | система | live | 127.0.0.1:9999 → 192.168.2.238:9999 (owner-gated) |
| `:8080/:8082` llama_serve/llama_edge | **llama-rs** | **Pi: live**, PC: dormant | OpenAI-compat serving; `lama-2.8` |
| `:8090/:8092` rusty_serve | **rusty-engine** | dormant | MoE/MiniCPM serve |
| `:9800` telenetis | **telenetis** | live | commutation (switching) плагін |
| `:20128` omniroute | **omniroute** | policy-down | usage dashboard (Node) |
| `:8091` poolAI | **poolai** | **policy-down** (ECO-8.4) | inference оркестрація; тато-план |
| `:9741` sport-forcast | **sport-forcast** | offline | Poland football forecast (private) |
| mesh (Android ×2 + Pi + Firefox) | GSV/agents | **3/4 live** (bifrost, pi4, pc-wgpu) | WASM SIMD shards, verified `max_err_model ≤ 2.5e-6` |
| `apk_edge` (Android phones) | engines | degraded | phone-edge worker |
| Tokenomica converter | standalone | offline | одноразовий usage→SVG (не сервіс) |

## 3. Де дублюється і де зламано (аудит 2026-11-17)

### 3.1 Канонічні намірені пари (майстер-правило)
- **poolAI FM §5.12 ↔ GSV tickets board**: обидві борди. Норма: **GSV `docs/gsv/tickets.jsonl`
  — операційний борд** (claim/done, `gsv_tickets*`, сесії `абракадабра`). poolAI FM-* це
  **архівно-архітектурний журнал** (ризики, FMs, bands) — не операційний. Нові робочі одиниці
  створюються квитками GSV; FM-рядки посилаються на них, не дублюють.
- **poolAI `docs/vision/` ↔ GSV `docs/vision/`**: два vision-знімки (реви розходяться: poolAI rev 498,
  GSV rev 516). Норма: **GSV — канонічна ревізійна цифра** для екосистеми; poolAI rev — внутрішнє
  дзеркало (наступний drain poolAI = `poolai-vision-sync`, оновлює обидві сторони в одному коміті).
- **loc-audit / ratio**: кожен плагін тримає свій — це не дубль, це per-repo gate. Лишається.
- **rolling logs**: уніфіковано 2026-11-16 (GSV `log.rs`, rusty-engine std-only, llama-rs default,
  telenetis journald). Більше ніяких плоских `*.log`.

### 3.2 Сесійне сміття poolAI (81 файл у `docs/`)
~45 файлів — одноразові `*_2026-01-22`, `PUSH_*`, `FINAL_*`, `EXECUTE_*`, `FIX_*`, `ROOT_CLEANUP`,
`FINAL_PUSH_READY`, `GIT_PUSH_NOW_*`… — артефакти аварійних сесій, не документи. **Не видаляти.**
**Правило:** все таке → `docs/archive/<date>/` (private batch); керовані документи `docs/` лишаються
≤ 30 канонічних (FM, roadmap, concept, HANDOFF/NEXT, vision/, troubleshooting/).

### 3.3 Сирітські дерева (кандидати на впорядкування)
- `GSV-WGPU` (registered, sync, **без upstream**): лише MC-2 mesh test plan; функцію вхоплено
  у GSV `crates/mesh-wasm/` + `/wgpu` + plugins. **Кандидат: архівувати в GSV docs/archive** або
  злити план у GSV, сама репо — закрити (owner рішення).
- `freebeat.ai`, `platinoff-profile` (sibling git, unregistered, 18–19 днів тише): поки не
  зареєстровані — drain неможливий; вирішити: **register** (тоді HANDOFF+NEXT+owner adopt) чи
  **archive** (owner рішення).
- `reebeat.ai`, `Tokenomica`: **не git-репозиторії** — чистий scratch. Видалення/впорядкування —
  owner-рішення.
- `llama.cpp-master/` у llama-rs (vendored upstream, gitignored): шум у пошуку; прибрати після
  ECO-8.2 (owner).

### 3.4 Зламані / stale документи
- **poolAI HANDOFF** (`docs/development/HANDOFF_NEW_SESSION.md`): стан 2026-08-17 (kit band 127,
  rev 493) — **stale**; реальність: rev 498 пізніше, GSV на 0.248.0/rev 516; poolAI позадреначу
  ~7.3 дні. → потребує оновлення банером на цей канон (фаза P1).
- **telenetis**: HANDOFF/NEXT відсутні (лише `AGENTS.md`) — overnight band 227+ не знав про 228/233
  станів. → P1.
- **mesh-node**: без HANDOFF (AGENTS.md + README) — зареєстровано в `PRODUCTS.md` 2026-11-17 як Pi-агент; remote `platinoff/mesh-node` лишається owner-gated.
- **GSV-WGPU**: HANDOFF/NEXT родяться в ньому, але ніхто їх не читає (orphan). → ескорити в archive.
- **GSV HANDOFF**: актуальний (ревізію оновлено 2026-11-17).

## 4. Розвантаження poolAI та симбіоз (адаптований план)

**Цільова модель (owner 2026-11-17 — метафору перевернуто):** GSV — «мама» сервіси (сенсори, пам'ять, дія, безпека, мережа — хаб, що керує екосистемою, народжує плагіни й тримає все живе; власниця вчилась на poolAI, тому генетичний фонт коду — тато), poolAI —
«тато» сервіси (план, ризик, FM, ритм релізів, концепт — генетичний фонт коду, перший проєкт власниці), плагіни — виконавці (engines/commutation).
 poolAI **розвантажується** від того, що вже живе краще в GSV (vision-sync вірогідно, keep-live,
 products state/wiring, hygiene checks) і **концентрується** на власній FM-лінії, WSP/35B-дослідженнях
 та менеджменті плагінів.

| Фаза | Дія | Де | Власник | Ворота |
|------|-----|----|---------|--------|
| **P0** | Цей канон + state/wiring знято + tickets-борд синхронізовано | GSV | agent | коміт + push |
| **P1** | Нормалізація доків: poolAI session-мусор → `docs/archive/2026-11/`; poolAI HANDOFF/NEXT банер → цей канон; telenetis HANDOFF/NEXT створити; PRODUCTS.md future rows | poolAI, telenetis | agent | owner go + один коміт до репо |
| **P2** | Єдині сервіси: keep-live = only GSV watchdog (poolAI stubs `*.sh` → pointer); FM §5.12 → GSV tickets (`gsv_tickets*`); vision rev → GSV; `products --state/--wiring` як S0-перевірка poolAI | poolAI | agent | P1 + owner go |
| **P3** | Симбіоз-перевірка: `cargo test-ci` (poolAI) + `cargo test` (GSV) + live `:9999/api/health` + MCP 61/61 + stand-smoke + products --state/--wiring зелені; 1 FM-міні-спринт у poolAI через claim у GSV борд | обидва | agent | все P1–P2 + owner |
| **P4** | Глибока інтеграція: FM→ticket federation — **P4.1–P4.3 ✅ 2026-11-18** (GSV: `Ticket.fm_ref` optional wire-незалежне поле; `POST /api/tickets/link {id, fm}` · `POST /api/tickets/claim{fm}` · `GET /api/tickets?fm=<id>` · MCP `gsv_tickets{fm}` / `gsv_tickets_claim{fm}` · advisory `cargo xtask fm-xlink` + MCP `gsv_xtask{task:fm-xlink}` read-only gate від poolAI FM до `fm_ref`; committed `5979f0b` → `af8023a`). **P4.4 backfill: жодного FM-cited `t-*` без FM-### нумерації не існує** — єдиний FM-cited квиток (`t-1794880501219432800`) цитується з symbiosis-блока без FM-### номера; встановлення fm_ref було б вигаданим лінком; лишено для першого реального FM-cited квитка. **P4.5 ✅** (fmt → clippy --all-targets 0 warnings → `cargo test` exit 0, ELAPSED 657s · stand-smoke **86/86** vs живий хаб · products --wiring ok ×14 + na (freebeat.ai/platinoff-profile)). **P3 ✅ 2026-11-17** (MCP 60/60, stand-smoke **86/86** — виправлення 87/87 закомічено poolAI `14a95033c`; GSV cargo test exit 0 ELAPSED 688s). | poolAI | agent | P3 + owner |

**Не робити:** не переписувати poolAI код; не видаляти доки (тільки archive); не чіпати `data/*`,
`.env*`, ключі; не рушати omniroute/phone edge без owner; не реєструвати orphans без owner-рішення.

## 5. Протокол пошуку зламаних/дублів (run-book, 10 хв)

Запустити з GSV (MSYS2 bash):

```bash
export PATH="/c/Users/plati/.cargo/bin:/ucrt64/bin:/usr/bin:$PATH"; unset RUSTUP_TOOLCHAIN
cd /s/rust/GSV

# 1) стан всіх продуктів (git + wiring + version)
cargo xtask products --state
cargo xtask products --wiring

# 2) TODO/FIXME/XXX/HACK per repo (топ-40)
for r in GSV poolAI llama-rs rusty-engine telenetis rebook LinFS ORR_DESKTOP mesh-node; do
  printf "== %s\n" "$r"; rg -n --no-heading -e TODO -e FIXME -e XXX -e HACK /s/rust/$r \
    --glob '!**/target/**' --glob '!**/llama.cpp-master/**' | head -10
done

# 3) Python  ban (має бути порожньо у git)
for d in GSV poolAI llama-rs rusty-engine mesh-node telenetis; do
  printf "== %s\n" "$d"; git -C /s/rust/$d ls-files '*.py' | head -5
done

# 4) dead bins: [[bin]] у Cargo.toml без src/bin/<name>.rs
for c in GSV poolAI llama-rs rusty-engine; do
  rg -n --no-heading '^name = ' /s/rust/$c/Cargo.toml | head -20; ls /s/rust/$c/src/bin 2>/dev/null | head -20
done

# 5) зламані doc-посилання (усі [x](../y) на неіснуючі файли)
for d in GSV poolAI; do
  printf "== %s\n" "$d"; rg -o --no-heading '\]\(([^)]+)\)' /s/rust/$d/docs --glob '*.md' | head -60
done
# (звірка з реальними файлами)

# 6) свіжість SpeedIndex (≤ 31 дня)
ls -la /s/rust/GSV/docs/vision/speed_index.json /s/rust/GSV/docs/vision/rust_diagnostics.json

# 7) мовчазні mem-рейди: watchdog.json + heartbeat json (усі <= 60 s)
cat /s/rust/GSV/target/live/watchdog.json; ls -la /s/rust/llama-rs/target/live/llama_heartbeat.json

# 8) basics: hub health + MCP tool count
curl -s http://192.168.2.238:9999/api/health | head -c 200
curl -s http://192.168.2.238:9999/mcp | head -c 200
```

**Вихід протоколу = список квитків** (кожен «знайдене X» → рядок у `docs/gsv/tickets.jsonl`
або TODO-запис §6). «Зелене»: 1–4 == порожнє або з тре'ками; wiring == ok; speed_index ≤ 31д;
heartbeats ≤ 60с; health ok.

## 6. Єдиних сервісів ≥ per-plan (після симбіозу)

| Сервіс | Єдиний власник |
|--------|----------------|
| Galaxy UI + `/api/*` + SSE | **GSV** |
| MCP sandbox (`gsv_mcp_openbot`) + tool count 60 | **GSV** |
| Tickets board (claim/done) | **GSV** (`tickets.jsonl`) |
| Keep-live / watchdog / sys-history | **GSV** |
| Rolling logs (tokio daily, 7d) | **GSV** (усі продукти посилаються) |
| Vision docs + rev | **GSV** (`docs/vision/`) |
| FM journal / risk rows (FM-*) | **poolAI** |
| MiniCPM / WSP research, 35B evals | **poolAI** (feature) + **rusty-engine** (serve) + **GSV** (mesh shards) |
| Phones agents / WebGPU WASM | **GSV** (`/api/wgpu/*`, `mesh-wasm`) |
| Inference serving (:8090/:8080/:8092) | **rusty-engine** / **llama-rs** |

## 7. Owner-рішення (потрібні для P1+)

1. **RAM/EDR** — блокер №1 (хаб падав тричі 2026-11-17; RAM 750–810 MB вільних). Swап
   Firefox +/або перенесення `cargo`/serve на інший вузол; або EDR allowlist на `gsv-server.exe`.
2. **mesh-node remote** (`github.com/platinoff/mesh-node`) — register у PRODUCTS.md **зроблено 2026-11-17**; remote лишається owner-gated.
3. **Orphan-дерева**: `GSV-WGPU` архів чи прибрати; `freebeat.ai`/`platinoff-profile` register чи
   архів; `reebeat.ai`/`Tokenomica` видалити чи перетворити в репо.
4. **P1 go** — дозволити poolAI docs archive sweep + HANDOFF/NEXT банер + telenetis HANDOFF.
5. **ECO-8.2** — 0.1.156/0.1.161 bump (MTP blocker) — owner рішення.
6. **PH-A5** — dedicated ≥24 GB node для 35B forward.
7. **WSP Phase 2** — лише після явного go.
8. **Loopback proxy** — `iphlpsvc` `127.0.0.1:9999 → 192.168.2.238:9999` (`netsh interface portproxy` — потрібні права адміна).

## 8. Квитки для цього канону (у борді)

Створені див. нижче (§9): `t-…P1 docs normalization`, `t-…P2 keep-live/FM unification`,
`t-…P3 ecosystem verification`. Також див. `docs/superpowers/plans/2026-11-16-next-priority-queue.md`.

## 9. Чого не робити нічого (не зроблено цим каноном)

- Жодна кодова зміна в плагінах — лише GSV docs + ticket rows.
- Жодного переміщення файлів — P1 цього не робить без owner go.
- Жодного скидання ревізій vision — rev 516 лишається.
- Жодного touch на poolAI коді/target/.

## 10. Live L: mirror persistent (added 2026-11-18)
- **`GSV_WEIGHTS_STORE_DIR` landinged in GSV code** (`src/boxes/weight_store.rs` + watchdog spawn + `xtask run_live`) — 2 incr unit tests, `cargo test --lib 503/503` green, clippy 0 warnings.
- **Auto-detect on boot:** if `GSV_WEIGHTS_STORE_DIR` is unset, watchdog/live auto-fills from `/l/gsv_mirror/wgpu_weights` when that path exists.
- **Windows boot chain confirmed end-to-end:** `HKCU\CurrentVersion\Run\GSV-watchdog` (`wscript.exe → target/live/gsv_watchdog_start.vbs → gsv-watchdog.exe`) — reboot-simulation passed: after `kill` of both processes, running the registered command alone spawned live hub (`disk_ok=true`) correctly serving L: mirror (`86 chunks` in manifest). **No registry/schtasks change needed.**
- **Default stays unchanged:** without L: or without folder, falls back to `{repo-root}/data/wgpu_weights/`.
- **Owner gate resolved for persistence** (closed the prior owner-懸; not a Windows-side ops change).
