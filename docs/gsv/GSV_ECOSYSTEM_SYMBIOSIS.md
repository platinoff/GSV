# GSV_ECOSYSTEM_SYMBIOSIS.md — екосистема як пара «тато і мама» + плагіни

**Status:** Draft for owner review (agent-drafted 2026-11-17) · **Owner:** platinoff · **Companions:** [`GSV_AGI_PATH.md`](./GSV_AGI_PATH.md) (суперхаб) · [`PRODUCTS.md`](./PRODUCTS.md) (реєстр) · [`GSV_VDT_KIT.md`](./GSV_VDT_KIT.md)

## 0. TL;DR

- **GSV = тато (операційний план).** Galaxy UI, vision board, tickets board, ranks, keep-live,
  watchdog, mesh (WASM + `/api/wgpu/*`), MCP sandbox, rolling logs, products discovery, vision docs.
  GSV **не** копіює FM poolAI; poolAI **не** друкує у GSV.
- **poolAI = мама (FM/management план).** Function Management journal, sprint/FM board,
  концепт inference-orchestration (edge / RAID / VM), VDT-процедури, `poolai-vision-sync`,
  WSP-план (MiniCPM 35B). poolAI не тягає Galaxy; GSV не лізе в FM.
- **Решта репозиторіїв — плагіни (діти).** Власні git-дерева, власний `target/`, власні локальні
  правила. GSV дає кров (kit, MCP, tickets, logs), poolAI дає ДНК (FM/WSP/генетичні патерни).
- Цей документ — **узгоджений стан екосистеми на 2026-11-17** + **адаптований план симбіозу і
  розвантаження poolAI** + **повторюваний протокол пошуку зламаних і дубльованих речей**.

## 0.1 Тверде обмеження флоту (owner 2026-11-17)

**Це всі вузли екосистеми — інші з'являтися не будуть.** Флот закритий:

| # | Вузол | Роль |
|---|-------|------|
| GSV `:9999` | хаб / тато (PC, 12 ядер, 7.6 GB RAM) | Galaxy UI + API + MCP + keep-live + watchdog |
| poolAI | мама (FM/менеджмент; `:8091` полісі-даун) | FM journal, менджмент плагінів |
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
| mesh-node | `S:/rust/mesh-node` | main | `bc102c3` | 0 / **немає upstream** | — | **no** | n/a | Pi 4 agent; очікує `platinoff/mesh-node` (owner) |
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
| `gsv-mcp` (stdio `target/live/gsv-mcp.exe`) | GSV | **live** | MCP 60/60 інструментів; OpenCode/Grok |
| `gsv-watchdog` | GSV | **live** (PID 24292) | самопідняття хаба; auto-sys-history |
| `:9999` loopback portproxy (iphlpsvc) | система | live | 127.0.0.1:9999 → 192.168.2.238:9999 (owner-gated) |
| `:8080/:8082` llama_serve/llama_edge | **llama-rs** | **Pi: live**, PC: dormant | OpenAI-compat serving; `lama-2.8` |
| `:8090/:8092` rusty_serve | **rusty-engine** | dormant | MoE/MiniCPM serve |
| `:9800` telenetis | **telenetis** | live | commutation (switching) плагін |
| `:20128` omniroute | **omniroute** | policy-down | usage dashboard (Node) |
| `:8091` poolAI | **poolai** | **policy-down** (ECO-8.4) | inference оркестрація; мама-план |
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
- **mesh-node**: без HANDOFF (AGENTS.md + README) — прийнятно до реєстрації; після remote — дзерна
  в PRODUCTS.md.
- **GSV-WGPU**: HANDOFF/NEXT родяться в ньому, але ніхто їх не читає (orphan). → ескорити в archive.
- **GSV HANDOFF**: актуальний (ревізію оновлено 2026-11-17).

## 4. Розвантаження poolAI та симбіоз (адаптований план)

**Цільова модель:** GSV — «тато» сервіси (сенсори, пам'ять, дія, безпека, мережа), poolAI —
«мама» сервіси (план, ризик, FM, ритм релізів, концепт), плагіни — виконавці (engines/commutation).
 poolAI **розвантажується** від того, що вже живе краще в GSV (vision-sync вірогідно, keep-live,
 products state/wiring, hygiene checks) і **концентрується** на власній FM-лінії, WSP/35B-дослідженнях
 та менеджменті плагінів.

| Фаза | Дія | Де | Власник | Ворота |
|------|-----|----|---------|--------|
| **P0** | Цей канон + state/wiring знято + tickets-борд синхронізовано | GSV | agent | коміт + push |
| **P1** | Нормалізація доків: poolAI session-мусор → `docs/archive/2026-11/`; poolAI HANDOFF/NEXT банер → цей канон; telenetis HANDOFF/NEXT створити; PRODUCTS.md future rows | poolAI, telenetis | agent | owner go + один коміт до репо |
| **P2** | Єдині сервіси: keep-live = only GSV watchdog (poolAI stubs `*.sh` → pointer); FM §5.12 → GSV tickets (`gsv_tickets*`); vision rev → GSV; `products --state/--wiring` як S0-перевірка poolAI | poolAI | agent | P1 + owner go |
| **P3** | Симбіоз-перевірка: `cargo test-ci` (poolAI) + `cargo test` (GSV) + live `:9999/api/health` + MCP 60/60 + stand-smoke + products --state/--wiring зелені; 1 FM-міні-спринт у poolAI через claim у GSV борд | обидва | agent | все P1–P2 + owner |
| **P4** | Глибока інтеграція: FM→ticket federation (poolAI FM rows auto-render з GSV tickets); poolAI keep-live advisory; розвантаження `docs/` poolAI до ≤ 30; ECO-8.2 після owner-рішення | poolAI | agent | P3 + owner |

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
2. **mesh-node remote** (`github.com/platinoff/mesh-node`) — плюс register у PRODUCTS.md.
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
