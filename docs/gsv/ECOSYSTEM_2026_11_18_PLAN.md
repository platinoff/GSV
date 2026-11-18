# Екосистема — зведення стану та план дій (2026-11-18)

**Контекст:** `абракадабра`/`agi` + free-drain. Rust **1.100.0** (обидва канали), 35B-A3B mesh
(gate/up/down shared-expert тензори), мобільний `/wgpu` PWA + WS-push + JS-телеметрія +
remote-debug console channel, повний block-op прогін 40/40. **Усі репо запушені/чисті**
(крім GSV-WGPU та mesh-node — нема remotes, owner gate). Тут: стан + ресерч Rust 1.100 +
peers bench + план дій.

Частини документа:
1. [Ресерч Rust 1.100.0](#1-ресерч-rust-11000-що-реально-дає-швидкість)
2. [Стан екосистеми (products --state)](#2-стан-екосистеми)
3. [Peers bench — швидкість/стабільність](#3-peers-bench)
4. [poolAI FM — k8s/balancer/network verification](#4-poolai-fm--k8sbalancernetwork)
5. [Що зроблено](#5-що-зроблено-2026-11-17--18)
6. [План дій (P0/P1/P2)](#6-план-дій)

---

## 1. Ресерч Rust 1.100.0 (що реально дає швидкість)

Офіційно: **never type** (`!`) finally stable після 10 років; **`Allocator` API**
(Vec/Box/Arc/Rc з `Global`/`System` + `new_in`/`with_capacity_in`) — фундамент кастомних аллокаторів;
**`DropGuard`** (`core::mem`) — closure на кожному exit path (транзакції/rollback);
**`core::mem::conjure_zst`**; **Cargo**: `unused_dependencies` warn (lints.cargo), `build-dir` layout v2
(cast hashing consistent → менше зайвих перебілдів), **min-publish-age** (`[registry] global-min-publish-age = "3 days"`),
`bool::toggle`. Перформанс-компілятора (nnethercote 2026-07-31): mean wall-time **−5.59%**; rustdoc значно
прискорився. Breaking change: never-type fallback `() → !` у edition < 2024 (soft future-compat warnings).

**Зв'язок із нашою екосистемою:**
- **GSV уже на mimalloc** (global allocator у `gsv-server`/`gsv-mcp`, band 231) — ми попереду цієї кривої;
  1.100 не вимагає змін.
- **`Allocator` API** — кандидат на окремий band: перевірити `Vec::new_in` у hot paths
  (`wgpu_tasks.jsonl` ring, weight-store index build) замість глобального allocator для точкових пулів.
- **Never type**: `fn fut() -> impl Future` комбінація з `Infallible`→`!` — дає простіші сигнатури в нових
  peer/ops API без breaking змін.
- **Cargo unused-dependencies**: `cargo check` в 11 репо вже показує warnings — журнали маленькі,
  але `unused_dependencies` варто вмикати поступово (GSV: `[lints.cargo]` за бажання owner).
- **Breaking risk**: edition 2021 + never-type fallback — усі 11 репо збираються чисто на 1.100.0
  (clippy 0 + тести зелені), тож тривоги немає; але при апгрейдах edition міgrantся — тримати в голові.
- **CI note**: `poolAI` CI в обидва OS висить на `toolchain: stable` = вже 1.100.0; doc-ревізії AM не потрібні.

**Блезінг 1.100 — що брати:** (1) mimalloc + Allocator API для mesh/dispatch paths; (2) `DropGuard`
для транзакцій `settings/tickets/state files`; (3) unused-deps lint як CI- gate warning; (4) min-publish-age
в `poolAI` (з поправкою на `CARGO_RESOLVER_INCOMPATIBLE_PUBLISH_AGE=allow` для дерзань).

---

## 2. Стан екосистеми

`cargo xtask products --state` (2026-11-18, після пушу сьогодні):

| Репо | Гілка | HEAD | dirty | Версія |
|---|---|---|---|---|
| GSV | main=origin | f8fdffc + a9ea1ca + 8deb9da + latest | 0 | 0.248.0 |
| poolAI | main=origin | 592f1200a | 0 | 0.2.2 |
| telenetis | main=origin | a9e3407 | 0 | 0.2.0 |
| llama-rs | master=origin | 9d840b1 | 0 | 0.1.0 |
| rebook | master=origin | 6424975 | 0 | 0.1.0 |
| ORR_DESKTOP | master=origin | 40298ab | 0 | 0.1.0 |
| omniroute | release/v3.8.50=origin | e3d202c | 0 | — |
| rusty-engine | main=origin | 579b693 | 0 | 0.1.0 |
| sport-forcast | main=origin | 884004c | 0 | 0.1.0 |
| CoreBoard | main=origin | e020b93 | 0 | 0.1.0 |
| GSV-WGPU | main (no remote) | c381a4f | 0 | 0.1.0 |
| mesh-node | main (no remote) | 1cf0c8f fmt | 0 | — |
| freebeat.ai / platinoff-profile | main, clean | — | 0 | — |

**Безremote (owner gate):** GSV-WGPU, mesh-node (історія була «owner створити GitHub repo»).

---

## 3. Peers bench

З `data/wgpu_tasks.jsonl` (`done` rows, mean MFLOPS):

| Peer | op | n | mean MFLOPS | max MFLOPS | mean elapsed |
|---|---|---|---|---|---|
| valhall #35 | weights_matvec | 5 | 45.2 | 57.6 | 13.1 ms |
| valhall #35 | weights_matvec_block | 96 | 10.0 | 99.1 | 4861 ms |
| bifrost #38 | weights_matvec | 3 | 1.6 | 4.0 | 1189 ms |
| bifrost #38 | weights_matvec_block | 68 | 4.8 | 102.3 | 9112 ms |
| pi4-cpu #36 | weights_matvec | 8 | 0.0 (CPU-untimed) | 0.0 | 4192 ms |
| FF-PC #37 | — | 0 (ghost) | — | — | — |

**Інтерпретація:** два блок-класи в одному тесті (матвеки trivial ×1 tasks vs блоки ×3 тензори).
Макс. MFLOPS одного shard'а ≈ 100 — mesh уже працює на рівні телефонного WebGPU.
Stability: 164 блоки за 24 h, max_err до 1.58e-6, жодного хаб-стороннього mismatch — golden-x
verify працює. Pi4 залишається повільним хвостом; FF-PC не робить нічого (proofs_fresh=0, ghost).

---

## 4. poolAI FM — k8s/balancer/network

Запитуване «кубернетіс балансер нетворк» в FM-таблиці **не виділено як окремі рядки**, але є фізичні модулі:

| Компонент | Шлях | Тести | Стан на box |
|---|---|---|---|
| Kubernetes client | `src/cloud/kubernetes.rs` | `tests/cloud_kubernetes.rs` | **не збирали тут** — RAM/EDR; `cargo check --features cloud,cloud-sdk` стартуваний (фон), CI = авторитет |
| Load balancer | `src/cloud/loadbalancing.rs` | `tests/cloud_loadbalancing.rs` | те саме |
| Network profiles | `src/grid/galaxy_network_profile*.rs` + `/admin/network_profiles` | — | той самий клас |
| CI matrix (з `ci.yml`) | `Run tests (default)` ≡ `test-ci` (`ml,enterprise,cloud,test-utils,job-store-sqlite,prometheus`) на ubuntu+win | stable=1.100.0 | **живе на GitHub Actions**, не тут |
| FM-011 | `cargo test-ci` alias ≡ CI steps 94–99 | ок | задокументовано 2026-04..05 |

**Інше (нетворк):** `src/network` + `src/ui/admin/network_profiles.rs`. FM §5.12 каже: якщо <10 відкритих —
project scan (концепт → FM §5.1 → roadmaps). Наразі §5.12 <10 — план на нову смугу готуємо нижче.

**Висновок:** poolAI k8s/balancer/network функціонал **у коді є**, але: тести — CI-bound (`cloud_kubernetes`,
`cloud_loadbalancing`), на цьому боксі ставка у 7.7 GB RAM фізично неможлива; FM-таблиця про ці модулі
не говорить — рекомендується окремий FM-редизайн (див. план P2-13).

---

## 5. Що зроблено (2026-11-17…18)

### Rust 1.100.0 усюди
- обидва канали → **1.100.0**; 11 rust-репо на stable-gnu (floats, без змін); застарілі doc-піни доків осучаснені
- гайт 1.100: GSV 531/531 lib + контракти 46+10, telenetis 314/314, rusty-engine 256/256, llama-rs 43/43,
  ORR 33/33, CoreBoard 513/513, sport-forcast 39/39, GSV-WGPU 20/20, mesh-node `cargo check`
- poolAI: `cargo check` OK (78 s); test-ci → **CI** (`7d65c505c`+`592f1200a` RAM guard)

### Mesh / 35B-A3B / 40/40 block-op на всьому 35B (2026-11-18)
- Мобільний `/wgpu` PWA + service worker `gsv-shell-v3`; WS-push `/api/wgpu/ws` (`a718bc3`); JS-телеметрія `/api/wgpu/telemetry` (`855ab65`); **remote-debug console** `/api/wgpu/console` (`a9ea1ca`, phone window.error/unhandledrejection → hub ring)
- **Block-op ×40 шарів verified live** (max_err 3.87e-7…1.58e-6), `2026-11-18 09:35→10:10`; логи: `docs/gsv/R4_FULL_FLEET_DISPATCH_2026-11-18.md`
- Баг-фікси: `e9ab6ca` (reduce toy-reference), `7da54de` (block route browser-only), `2dca721` (claim-time guard), `a718bc3`-ера minifacts (`gsv_sw` pin fix, weight_store env-race hermetic)
- **RAM guard** у poolAI script (`592f1200a`); **scratch cleanup** 473 файли; **disk 26 GB GSV + ~8 GB інших репо** звільнено (S: 44→**72 GB** вільно)

### Свіжий крок (2026-11-18)
- `f8fdffc` — ECOSYSTEM plan + HANDOFF: Rust 1.100 everywhere, poolAI CI offload, disk cleanup, push-summary, план P0/P1/P2
- `a9ea1ca`+`8deb9da` — remote-debug console channel doc+код

---

## 6. План дій

### P0 — негайно / наступна сесія
1. **RAM боксу** — тихі смерті хаба (MemFree <0.75 GB перед смертю). Варіанти: закрити Firefox/Telegram,
   системний reboot, або апгрейд RAM. DoD: 7 днів без тихої смерті АБО повний перехід дуби компіляції на CI.
2. **poolAI cloud/k8s на CI** — після `7d65c505c` перевірити, що CI green на `main` (ubuntu+win, 1.100.0);
   якщо червоний — фікси ОКРЕМИМ бандом.
3. **rebook stale-тест `interior_plan_is_130_english`** — owner рішення (осучаснити/видалити); не чіпав.
4. **FF-PC #37** — ghost (proofs_fresh=0, ~5 днів); route-виключати продовжувати; видалити peer row — тільки owner.

### P1 — екосистемне
5. **mesh-node remote** — owner створити `github.com/platinoff/mesh-node` + push.
6. **GSV-WGPU remote** — owner створити `github.com/platinoff/GSV-WGPU.git` + push.
7. **telenetis full test** — лише CI (live `telenetis-live.exe` lockdebug-exe); локально lib-only.
8. **Pi4 = тільки matvec-класи** (layernorm/softmax/mlpblock/minilm/matvec); block-op — тільки браузери (static guards: enqueue + claim; два фільтри `is_browser_peer_class`).
9. **Cargo unused-deps lints** (1.100 feature) — позначити як candidate у GSV next bands; не вмикати пакетом без узгодження.
10. **Allocator API bring-up** (окремий band): `Vec::new_in` у гарячих точках mesh dispatch, замкнути з mimalloc.

### P2 — бэклог / owner
11. **poolAI FMредизайн**: окремі рядки для `cloud::kubernetes` + `network::loadbalancing` з CI статусом кожного; FM v0,2 п. до GOAL/n.
12. **ECO-8.2/8.3/8.4/9.x** — див. GSV board (`tickets.jsonl`); **ECO-7.7** rebook books (owner).
13. **/sys conky RAM pill** — візуальний сигнал free_mb (малий band).
14. **Telenetis keep-live hang** — `cargo test` при запущеному keep-live висить на `Access denied`; документовано; owner не хоче це влаштовувати.
15. **Pi4 toolchain 1.100** — Pi arm64 toolchain на Pi, зроблено окремо; не GSV блок.

### Власник-гейти
- `:8091` OmniRoute — policy-down; не піднімати.
- llama-rs Pi 4 `llama-cpp-sys-2` 0.1.156/0.1.161 — MTP draft блокує; лише owner.
- CoreBoard owner-gated 91480c2 (ECO-8.3) — будь-яке розширення понад doc-коміти за owner.
- RAM upgrade / системний reboot.

### Команди
```bash
cd /s/rust/GSV && cargo xtask products --state
curl -s http://192.168.2.238:9999/api/health | head -c 80
bash /s/rust/poolAI/scripts/check_target_disk.sh
curl -s http://192.168.2.238:9999/api/wgpu/console | head -c 300   # remote debug (phones)
```
