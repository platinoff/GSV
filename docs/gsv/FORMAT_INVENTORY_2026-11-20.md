# Format inventory (2026-11-20, hygiene sweep)

Generated: `cargo xtask inventory` / `cargo xtask inventory --tracked` from `S:\rust\GSV` (root `S:\rust`, 17 repos).  
Canon allowed formats: `.rs`, `.md`, `.mdc`, `.json`, `.js`, `.wasm`. Classification/exempt reasons: `src/boxes/inventory.rs` + `docs/gsv/FORMAT_EXEMPTIONS.md`.

## Baseline → final → recheck (regrep)

| Metric | Baseline (sweep start) | Final (sweep close) | **Recheck 2026-11-20 (post-CNV1 W1–W16)** |
|---|---:|---:|---:|
| Violating (all non-allowed) | 42,094 | 22,560 | **22,570** |
| Tracked violating (∩ index) | 9,287 | 9,257 | **9,266** |
| Exempt (all) | — | 1,321 | **1,329** |
| Deferred (all) | — | 21,239 | **21,241** |
| Actionable (all) | — | 0 | **0** |
| Untracked (all) | — | 13,303 | **13,304** |
| **Tracked actionable** | — | 0 | **0** |

**Regrep result:** `cargo xtask inventory --tracked` has **0** `|actionable` entries. All remaining non-compliant (violating) files are **exempt** or **deferred** by documented reasons; none are actionable.

**Delta vs final (logical):** +10 violating / +9 tracked / +8 exempt / +2 deferred / +1 untracked / +18 allowed. Changes are concentrated in **GSV** (+1 allowed: plan/docs updates) and **omniroute** (+17 allowed, +8 exempt, +2 deferred, +9 tracked, +1 untracked) from bounded CNV-1 W1–W16 work: added/updated Rust crates and docs (`.rs/.md` allowed), crate manifests/config (`Cargo.toml`, `.gitignore` exempt as rust-ci-infra/repo-infra), and opt-in bridge artifacts that remain under CNV-1 deferral in omniroute (`scripts/cnv1-smoke-rust-bridge.mjs` as `.mjs`, `open-sse/services/compression/nativeRust.ts` as `.ts`). Generated `crates/omniroute-wasm/pkg/` scratch was removed before recheck (gitignored build output).

## Per-repo summary (`cargo xtask inventory`, recheck)

| repo | allowed | viol | tracked | exempt | defer | action | untracked |
|---|---:|---:|---:|---:|---:|---:|---:|
| CoreBoard_Labs_shop | 38 | 9 | 9 | 9 | 0 | 0 | 0 |
| GSV | 536 | 187 | 30 | 187 | 0 | 0 | 157 |
| GSV-WGPU | 19 | 6 | 6 | 6 | 0 | 0 | 0 |
| LinFS | 68 | 19 | 19 | 19 | 0 | 0 | 0 |
| ORR_DESKTOP | 25 | 13 | 9 | 13 | 0 | 0 | 4 |
| Tokenomica | 9 | 5 | 0 | 5 | 0 | 0 | 5 |
| adoreble_wifi | 18 | 20 | 17 | 20 | 0 | 0 | 3 |
| freebeat.ai | 5 | 16 | 2 | 16 | 0 | 0 | 14 |
| llama-rs | 230 | 10409 | 18 | 19 | 10390 | 0 | 10391 |
| mesh-node | 9 | 15 | 5 | 0 | 15 | 0 | 10 |
| omniroute | 1697 | 8944 | 8920 | 679 | 8265 | 0 | 24 |
| platinoff-profile | 1 | 2 | 2 | 2 | 0 | 0 | 0 |
| poolAI | 1401 | 295 | 155 | 295 | 0 | 0 | 140 |
| rebook | 262 | 2555 | 17 | 0 | 2555 | 0 | 2538 |
| reebeat.ai | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| rusty-engine | 97 | 34 | 18 | 18 | 16 | 0 | 16 |
| sport-forcast | 22 | 17 | 17 | 17 | 0 | 0 | 0 |
| telenetis | 57 | 24 | 22 | 24 | 0 | 0 | 2 |
| **TOTAL** | **4494** | **22570** | **9266** | **1329** | **21241** | **0** | **13304** |

## Remaining (recheck, non-compliant = violating; actionable 0)

- **Tracked actionable:** **0**. **All actionable:** **0**.
- **Violating total: 22,570** — усі **exempt (1,329)** або **deferred (21,241)**, коротко:
  - **Deferred:** CNV-1 omniroute TS/JS family (**8,265**, owner-epic TS→Rust), vendored llama.cpp (**2,140**), vendored CMake (**7,985**) + LLVM/clang (**256**, owner toolchains), ECO-8.2 patch-set (**9**), rebook (**2,555**, untouchable), mesh-node scripts/images (**15**, owner), rusty-engine model artifacts (**16**).
  - **Exempt:** repo/CI/config (`rust-ci-infra`, `repo-infra`, `repo-config`, `template-config`), UI glue (`.html/.css`), docs/assets/media/binary, DB migrations (`.sql`), ops/deploy/native sources & scripts (`.sh/.ps1/.py`, build includes, OS launchers/quadlet), IDE/test fixtures, metadata/runtime artifacts, env templates, untracked local secrets, editor/stray artifacts тощо (див. `FORMAT_EXEMPTIONS.md`).
- **Untracked violating:** **13,304** (здебільшого vendored/deferred trees + runtime/local artifacts; **0 actionable**).

## Логічна заміна на дозволені формати (план дій, без порушення owner-gates)

| Категорія (залишок) | Стан | Логічна заміна / дозволений шлях |
|---|---|---|
| **CNV-1 omniroute TS/TSX/MJS/CJS/MTS/CTS (8,265 deferred)** | TS runtime — source of truth; epic owner-gated | Продовжувати bounded Rust-порти в `.rs` (`crates/*`) + opt-in інтеграції; де доречно — **`.wasm`** (policy-allowed output) і тонкий **`.js`**-адаптер; інкрементальний strangler-fig без ламання npm/Node/CI. Нові bridge-файли в omniroute (`.ts/.mjs`) лишаються deferred до міграції/перенесення логіки в Rust. |
| **Vendored toolchains/upstream (CMake/LLVM/llama.cpp, ECO-8.2 patches)** | owner-managed build deps | **Не конвертувати/не видаляти**; заміна можлива лише разом з owner-рішенням про апгрейд/перепакування toolchain/upstream (окремі стріми). |
| **rebook (2,555)** | untouchable | **Не чіпати** (owner). |
| **mesh-node scripts/images (15, GEN-4)** | owner-gated | За owner-рішенням: перенести інструкції в **`.md`**, прибрати скрипти/артефакти flash; інакше лишається deferred. |
| **rusty-engine model artifacts (16)** | weights/binaries | **Не конвертувати** (binary/model). |
| **Exempt нативні формати** (`.yml/.yaml`, `.toml/.lock`, `.sql`, `.html/.css`, `.sh/.ps1/.py` тощо) | required by tools/CI/runtime | Залишаються exempt; заміна на `.rs`/`cargo xtask`/`.md`/`.json`/`.wasm` лише де семантично можливо і без ламання контрактів (точкові GEN/CNV задачі, owner-confirm для чужих репо). |

## By reason (recheck, all)

| count | reason |
|---:|---|
| 7985 | CLN-2 vendored cmake distro (owner) |
| 256 | CLN-2 vendored llvm/clang distro (owner) |
| 8265 | CNV-1 omniroute TS→Rust owner epic |
| 9 | ECO-8.2 llama-cpp patch-set (owner) |
| 15 | GEN-4 mesh-node scripts/images (owner) |
| 134 | binary-artifact |
| 1 | config-template |
| 2 | data-export |
| 130 | db-migrations |
| 5 | deploy-config (quadlet) |
| 350 | docs-assets |
| 2 | editor-backup |
| 2 | env-template |
| 8 | ide-http-client |
| 42 | llm-txt-convention |
| 5 | local-secret (untracked) |
| 16 | metadata-ring |
| 16 | model weights/source artifacts |
| 1 | native-build-config |
| 1 | native-source (build) |
| 2 | operator-example-template |
| 5 | ops-script (python) |
| 3 | os-deploy-launcher |
| 1 | os-ops-script |
| 2555 | owner: book-repo untouchable |
| 48 | playwright-e2e-js |
| 1 | product-canon-doc (path-contract) |
| 7 | release-binary |
| 86 | repo-infra |
| 22 | runtime-data-artifact |
| 10 | runtime-output |
| 163 | rust-ci-infra |
| 186 | shell-ops-script |
| 2 | stray-artifact (tracked) |
| 1 | template-config |
| 54 | test-fixture |
| 1 | typescript-source |
| 37 | ui-glue |
| 1 | vale-config |
| 2140 | vendored llama.cpp (owner) |

## CNV status (recheck)

- **CNV-1:** omniroute `*.ts/tsx/mjs/jsx/cjs/mts/cts` — **exempt-till-epic** (deferred **8,265** після CNV-1 W1–W16, incl. opt-in bridge `.ts/.mjs`). Логічний шлях: bounded Rust `.rs` + opt-in `.wasm`/`.js` інтеграції, strangler-fig. Runtime TS лишається source of truth.
- **CNV-2:** tracked `.txt` audit closed — 0 actionable `.txt` (exempt/deferred).
- **CNV-3:** omniroute 9 `.mmd` → fenced-mermaid `.md` (`82e6fcf`).
- **CNV-4:** `*.http` = `ide-http-client`.
- **CNV-5:** png/jpg/docs-assets exempt; svg not touched.
- **CNV-6:** tracked `*.sql` = `db-migrations`, tracked `*.yml/*.yaml` = `rust-ci-infra` confirmed.

## Follow-up classifier pass (2026-11-20)

- Added/clarified: untracked local secrets exempt (`local-secret (untracked)`), `llama-rs/.llvm/**` deferred as vendored toolchain, `runtime-data-artifact` (`poolAI/data/**`, e2e visual tmp), compressed/binary extensions (`.gz/.bz2/.zst/.zip/.7z/.tar/.tgz/.qch/.inv/.hex/.sig`), native build includes (`.inc/.def/.modulemap`), `pid` basename repo-infra.
- Result: **tracked actionable 0, all actionable 0**.

## Notes (recheck)

- **Regrep:** `cargo xtask inventory` / `--tracked` (root `S:\rust`) → **actionable 0, tracked-actionable 0**. Generated `omniroute/crates/omniroute-wasm/pkg/` (gitignored, transient) прибрано перед recheck.
- **CNV-1 progress (post-sweep):** omniroute W1–W16 pushed on `release/v3.8.50` (`8e6a304`…`efb4bec`), additive Rust crates/opt-in napi+WASM; default runtime/build flows не змінювались. Omniroute npm/npx hooks у цьому env пропускались при комітах.
- poolAI full tests = **CI-authority**; local GEN-3 gate `fmt+clippy+check`.
- mesh-node hygiene commits **local-only** (no remote): `e4d2793`, `bae1008`.
- Tracked actionable list: `cargo xtask inventory --tracked` → **(empty, 0 `|actionable`)**.
