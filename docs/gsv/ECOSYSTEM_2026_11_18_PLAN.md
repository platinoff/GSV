# Екосистема — зведення стану та план дій (2026-11-18)

**Сесійний контекст:** `абракадабра`/`agi`-цикл + free-drain, Rust 1.100 rollout, 35B-A3B mesh
(gate/up/down shared-expert tensors), мобільний `/wgpu` PWA, WS-push, JS-телеметрія.
**Автор:** стан після серії комітів 2026-11-17…18; очищено диск/гіт-стани; запушено все, що не owner-gated.

---

## 1. Що зроблено (коміти)

### Rust 1.100.0 усюди (2026-11-18)
- `stable-x86_64-pc-windows-msvc` → **1.100.0** (gnu був 1.100.0 з 12.11; обидва канали тепер на 1.100.0).
- Усі `rust-toolchain.toml` плавають на stable-gnu → **1.100.0** без змін пінів.
- Застарілі текстові піни `1.92.0` → `1.100.0`: GSV (README, `.cursor/rules/*` (4 файли), `GSV_BOXES.md`, `gsv-install.svg`, `ticket_scenarios.json`), telenetis (HANDOFF), rusty-engine (README, AGENTS.md, concept/HANDOFF), sport-forcast (README, PLAN, RELEASE), CoreBoard (AGENTS.md, concept).
- Гейт 1.100.0 (cargo `1.100.0 (3acc79c0c)`): **GSV 526→529/529**, telenetis 314/314 lib, rusty-engine 256/256, llama-rs 43/43, ORR 33/33, CoreBoard 513/513, sport-forcast 39/39, GSV-WGPU 20/20, LinFS green, mesh-node `cargo check`, poolAI `cargo check` (78 s).
- History/тикети/архіви історичні піни НЕ чіпав.

### poolAI: test-ci → CI (2026-11-18, owner ask)
- `7d65c505c`: повний `test-ci` (`ml,enterprise,cloud,test-utils,job-store-sqlite,prometheus`) **тільки GitHub Actions** (ubuntu+windows, `stable`=1.100.0, кроки 94–99 `ci.yml` ≡ alias `test-ci`). Локально: `fmt + clippy + check`.
- `Cargo.toml`: `[profile.dev] incremental=false, debug=1` (mirrors `[profile.test]`).
- `AGENTS.md`: pin 1.92→**1.100.0**, owner-policy блок (локальний бокс НЕ запускає test-ci).
- `592f1200a`: `scripts/check_target_disk.sh` — RAM guard (`POOLAI_MIN_FREE_RAM_GB=3`, читає `/proc/meminfo` MemFree; `--enforce` → exit 1).
- `target/` debug: **2.4 GB → 12 MB**.

### 35B mesh / R4 block-op (2026-11-18)
- Мобільна PWA `/wgpu` (viewport-fit, safe-area, `<details>`, KPI-стрічка, sticky Start/Stop) — `b993af6` (жваво).
- **WS push-канал** `/api/wgpu/ws` (RFC 6455 власноруч у `wspush.rs`+`serve.rs`; accept = `s3pPLMBiTxaQ9kYGzzhZRbK+xOo=`) — `a718bc3` (жваво, 2 s `mesh-metrics`).
- **JS-телеметрія** `/api/wgpu/telemetry` (без ADB; UA/cores/mem/network/battery; hub sanitize → ring 100) — `855ab65` (жваво).
- **k8s synthesis** — `d8023d9` (`docs/gsv/K8S_WGPU_SYNTHESIS_2026-11-18.md`; fleet ≥16 GB вузол потрібен; phones=peers; не k8s).
- **Block-op ×6 верифіковано live**: blk.31 g-287 · blk.32 g-290 · blk.33 g-289 · blk.34 g-294 · blk.35 g-292 · blk.36 g-293 — усі `complete:true ok:true`, max_err 4.0e-7..9.8e-7.
- Три баги закрито: `e9ab6ca` (reduce_groups toy-reference для block-задач → per-task `max_err_model`), `7da54de` (route для block включає pi4-cpu/pc-cpu → browser-only), `2dca721` (sticky window лише 5 с ⇒ claim-time браузер-guard; Pi4 «обробив» shard за 0.007 ms → fail) — причини g-288/g-291 ліквідовані.
- `mesh_wire` peers тепер несуть `ready` (proof_gate).

### Дискова гігієна (2026-11-18) — звільнено ~30+ GB
| Репо | До | Після |
|---|---|---|
| GSV | 26.0 G | **5.3 G** (debug deps 20 G + incremental 3.5 G; `target/live`+release лишені) |
| llama-rs | 4.6 G | 2.6 G (release лишені) |
| rusty-engine | 2.9 G | 2.7 G (release 1.9 G лишені) |
| telenetis | 2.0 G | 317 M (debug deps/build; live service лишено) |
| rebook | 1.1 G | 112 M |
| ORR_DESKTOP | 641 M | 9.0 K |
| LinFS | 1.1 G | 22 M |
| sport-forcast | 263 M | 9.0 K |
| CoreBoard | 833 M | 80 M |
| GSV-WGPU | 1.2 G | 135 M |
| mesh-node | 595 M | 65 M |
| poolAI | 789 M | 9.0 M |

S: 44 GB → **73 GB вільно**. Scratch `Temp/opencode` (473 файли, 31 MB) очищено.

---

## 2. Зведення станів репозиторіїв (cargo xtask products --state, 2026-11-18)

| Репо | Гілка | Вперед | Дірті | Версія | Remote |
|---|---|---|---|---|---|
| GSV | main = origin | 0 | 0 | 0.248.0 | ✅ push щодня |
| poolAI | main = origin | 0 | 0 | 0.2.2 | ✅ |
| telenetis | main = origin | 0 | 0 | 0.2.0 | ✅ |
| llama-rs | master = origin | 0 | 0 | 0.1.0 | ✅ |
| rebook | master = origin | 0 | 0 | 0.1.0 | ✅ |
| ORR_DESKTOP | master = origin | 0 | 0 | 0.1.0 | ✅ |
| omniroute | release/v3.8.50 = origin | 0 | 0 | — | ✅ |
| rusty-engine | main = origin | 0 | 0 | 0.1.0 | ✅ |
| sport-forcast | main = origin | 0 | 0 | 0.1.0 | ✅ |
| CoreBoard_Labs_shop | main = origin | 0 | 0 | 0.1.0 | ✅ (`e020b93` запушено сьогодні) |
| GSV-WGPU | main, **no remote** | — | 0 | 0.1.0 | ❌ owner gate |
| mesh-node | main, **no remote** | — | 0 (fmt `6d15c85`+fmt коміт) | — | ❌ owner gate |
| freebeat.ai | main | — | 0 | — | не зареєстр. |
| platinoff-profile | main = origin | — | 0 | — | не зареєстр. |

---

## 3. План — що робити (за пріоритетом)

### P0 — негайно / наступна сесія
1. **RAM боксу — тихі смерті хаба.** Free Mem ~0.6–0.75 GB; хаб помирав ≥9 разів за 48 h (watchdog вмирав разом). Варіанти: (a) перекрити `firefox`/`Telegram` перед важкими роботами; (b) перенести важкі збірки на CI/окремий вузол (вже зроблено для poolAI); (c) owner просить reboot або апгрейд RAM. DoD: 7 днів без тихої смерті або перехід на CI-only збірки.
2. **Огляд GitHub Actions poolAI** — після `7d65c505c` перевірити, що CI на `main` проходить (ubuntu+windows). Якщо червоний — фіксити окремо (build warnings).
3. **rebook `interior_plan_is_130_english`** — stale-тест (assert до введення «partial KDP preview»: `art_page` навмисно повертає `Blank` для відсутніх plates). Лише owner: свідомо осучаснити assert (рекомендовано) або видалити тест. Не чіпав.
4. **FF-PC #37** — ghost (proofs_fresh=0, 5+ днів). Виргати не можна — лише виключати з route (вже працює). Якщо ПК взагалі не потрібен у mesh — прибрати peer row вручну (owner).

### P1 — екосистемне
5. **mesh-node remote** — owner створити `github.com/platinoff/mesh-node`, запушити (DNS/Pi образ). Без цього образ Pi4 + агент лише на цьому боксі.
6. **GSV-WGPU remote** — owner створити `github.com/platinoff/GSV-WGPU.git`, запушити.
7. **telenetis повний `cargo test`** — заблокований релізним `telenetis-live.exe` (PID 13360, keep-live). Локально lib-312/314 зелений + CI; відкривати повний тест лише через CI або після controlled stand-down сервісу.
8. **Pi4 = тільки доказ-free ops** (`weights_matvec`, `layernorm`, `softmax`, `mlpblock`, `minilm`); **block-op — тільки браузери** (guard у коді: enqueue + claim + browser-class). Розширювати mesh-node block-op НЕ планувати без окремого банду.
9. **Руководство RAM-guard** (`check_target_disk.sh`) — вже живий у poolAI; при бажанні owner перенести в GSV `cargo xtask disk`.
10. **HANDOFF/NEXT** — не обов'язково кожну сесію; оновлювати лише сталий стан (HEAD, keep-live, черга).

### P2 — бэклог / рішення owner
11. **ECO-8.2** (llama-rs `llama-cpp-sys-2` bump; заблокований MTP draft), **ECO-8.3/8.4/9.x** — див. GSV board (`docs/gsv/tickets.jsonl`); **ECO-7.7** rebook books (owner).
12. **Додати RAM-alarm** у `/sys` conky-панель (watched free_mb + візуальний pill).
13. **Telenetis keep-live hang** — `cargo test` висів на `Access denied` debug-exe (PID 13360). Задокументувати для owner (ніколи не запускати `cargo test` при запущеному keep-live в тому ж дереві).

### Власник-гейти (не робити без owner)
- `:8091` (poolAI OmniRoute) — policy-down, не піднімати.
- llama-rs Pi 4 0.1.156/0.1.161 bump — MTP draft блокує; лише owner.
- CoreBoard будь-що понад doc-коміти — owner-gated 91480c2 і далі.
- Системний reboot / RAM upgrade — owner рішення.

---

## 4. Корисні команди

```bash
# стан усіх репозиторіїв одним знімком
cd /s/rust/GSV && cargo xtask products --state
# перевірка RAM guard (poolAI)
bash /s/rust/poolAI/scripts/check_target_disk.sh
# жвавий стан хаба
curl -s http://192.168.2.238:9999/api/health | head -c 80
```

**Push підсумок:** GSV `3fc1a5d`+ (завтра оновиться цим документом), poolAI `592f1200a`, telenetis `a9e3407`, rusty-engine `579b693`, sport-forcast `884004c`, CoreBoard `e020b93` — **усі на origin**; mesh-node/GSV-WGPU — локальні (owner-gate remote).
