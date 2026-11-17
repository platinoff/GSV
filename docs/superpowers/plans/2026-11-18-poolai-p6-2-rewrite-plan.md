# P6.2 — poolAI tests rewrite plan (symbiosis gate)

**Driven by:** [`2026-11-17-tests-symbiosis.md`](./2026-11-17-tests-symbiosis.md) (§P6.2) · **Inventory:** [`./2026-11-17-tests-symbiosis-inventory.md`](./2026-11-17-tests-symbiosis-inventory.md) · **FM canon:** `poolAI/docs/catalog/FUNCTION_MANAGEMENT.md`
**Ticket:** `t-1794882494678007700` (product: poolAI) · **Out of scope:** running anything on this box (RAM ~756 MB free; poolAI test-ci needs a CI/separate runner)
**Evidence date:** 2026-11-18 (read-only snapshot; no builds run)

## 0. Що є зараз (live snapshot)

- `tests/` = **365** `.rs` файли.
- Розбивка за групами (grep by name):
  - `galaxy_horizon_s*.rs` — **157** (~43% усієї теки; ceremony-per-sprint suites — головний кандидат на згортання);
  - `audit_* / *_audit* / depth_audit / ci_canon_audit / e2e_scope_audit` — ~30 (FM-mode verifier suites, ті самі шаблони);
  - `raid_* / raft_* / distributed_raid_* / distributed_replication_*` — ~20;
  - `discovery_*` — ~11;
  - `vm_* / gpu_limits_* / saml_* / sso_*` — багато «domain contract» suites, які працюють; лишати без змін, окрім порушень дублю-каналу;
  - `galaxy_horizon_s{4xx..1xxx}` — видимі іди SUPERSEDED FT-s рядків (FM-ladder, band 150+) — звірити проти FM/FM-HISTORY перед видаленням.
- `tests/fixtures/` — живуть у git (не чіпати без вироку власника; легалізує фул-стенди й parity suites).

**Звірка з FM:** FM «Implemented» рядки посилаються на конкретні тести (`tests/distributed_raid_wire_integration.rs` etc.) — кожен FM-рядок мусить слідкувати за тестом, який пережине прибирання. Fetch list з FM (`grep '^FM-' docs/catalog/FUNCTION_MANAGEMENT.md`) як **source-of-truth mapping tests↔FM**.

## 1. Порядок модулів (per ticket)

`orchestrator` → `raid` → `discovery` → `ui-admin`. Один коміт у poolAI на модуль; FM-рядок на кожен закритий модуль.

### 1.1 orchestrator (модуль 1)
Метод: залишити behavior-контракти (`runtime_orchestrator_integration.rs`, `job_*`, `core_*`), згорнути ceremony suites в параметричні модулі таблицею FM-S-рядків. Виходить з FM §5.12 canonical map (FM-документи §5.12/§5.10x).
Gate: див. §2.

### 1.2 raid (модуль 2)
Метод: `distributed_raid_wire_integration.rs` (FM-007/008) — лишити as-is (канонічні позитивні контракти). Згорнути `raid_*` / `raid_smallworld_*` / point-in-time-`raft_*` дублікати в один table-driven suite.

### 1.3 discovery (модуль 3)
`discovery_*` — 11 контрактів; більшість потрібні, лишати 1:1, except doc-only дублікати.

### 1.4 ui-admin (модуль 4)
`admin_ui_api_contracts.rs`, `sso_*`, `tenant_*`, `ui_components_integration.rs` — лишати; перетин з FM-рядками звіряється вручну раз на модуль.

## 2. Definition of done (per module PR)

1. `cargo test-ci` (CI-features) **exit 0** у poolAI CI/вузлі — НЕ на цьому боксі;
2. `products --state` + `--wiring` зелёні (GSV keeps this gate);
3. `stand-smoke 86/86` (actual case count; 87→86 FM correction: CARDS=45 + core/vision/SVG 41);
4. FM row added (модуль, дата, FM-id, tests kept/merged/dropped списком);
5. **ratio ≥ 96% stretch** (poolAI local gate) або еквівалент за їхнім `--stretch-96`;
6. FM-ID → test-file mapping оновлено у `FUNCTIONALITY_DIGEST_2026-04-06.md` де доречно.

## 3. Жорсткі правила різання

- Тести, що замикають **канонічні негативні контракти** (never-leak, ніяких `:8091` посилань у GSV-інвентарях, User-scope MCP) **лишаються** — регресійні алярми, не legacy.
- `galaxy_horizon_s*`-згортання допустиме лише в параметричний suite зі **збереженням усіх FM-S-ID асертів** (кожний FM-S ід лишається рядком таблиці з власним очікуваним RFC), інакше не згортати — ризик пустої покриттєвості.
- Duplicate `raid_smallworld`/`raid_small_world` — консолідувати в один канонічний за перспективним неймінґом FM §5.13.
- жодного рядка з GSV у poolAI тести; GSV код не копіювати (§ envelope).
- `fixtures/` не чіпаємо без окремого власниківського вироку.

## 4. Уроки, перенесені з плагінів (P6.3)

- **EDR/%TEMP%**: будь-який fixture у `%TEMP%` → перенесення у `target/test-tmp/` (ель poolAI: `tests/fixtures/` залишається як є).
- **`--cfg test` harness-детекція**: `is_cargo_test_harness()` через `current_exe()` ламається на цьому боксі (EDR перейменовує екзешники); виправлено в telenetis (`9a8ae5b`) та llama-rs (`9d840b1`) тим самим способом — cfg!(test) short-circuit. Застосувати і в poolAI у тому самому fix-band PR.
- **unique nanos tmp** + **ENV_GUARD** для process-global env.

## 5. Що НЕ робити на цьому боксі

- `cargo test-ci` повним прогоном (RAM 756 MB free — не відбудеться);
- будь-які fixture-видалення без вироку власника;
- FM/vision rewrite — FM журнал лишається татом-планом, GSV нічого не пише в нього.

## 6. Перший крок, коли є CI/вузол

1. orchestrator-файл з §1.1 як PR #1; очікуваний ефект: ≥ 40 merged-away `galaxy_horizon_s*` (target ≥ 100 згорнутих по 4 модулям) при curve-direction green CI;
2. відкритий фіксація FM-рядка MODULE-P62-ORCH → PR → CI → merge → next module;
3. після останнього модуля — P6.5-повтор воріт.

**Статус плану:** готовий до виконання на CI/вузлі; увесь аналіз зроблено read-only. Жодної зміни коду poolAI тут не зроблено.
