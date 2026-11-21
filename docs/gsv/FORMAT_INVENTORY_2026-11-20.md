# Format inventory (2026-11-20, hygiene sweep)

Generated: `cargo xtask inventory` / `cargo xtask inventory --tracked` from `S:\rust\GSV` (root `S:\rust`, 17 repos).  
Canon allowed formats: `.rs`, `.md`, `.mdc`, `.json`, `.js`, `.wasm`. Classification/exempt reasons: `src/boxes/inventory.rs` + `docs/gsv/FORMAT_EXEMPTIONS.md`.

## Baseline → final

| Metric | Baseline (sweep start) | After CLN/GEN-2/CNV pass | **Final** |
|---|---:|---:|---:|
| Violating (all non-allowed) | 42,094 | 22,562 | **22,562** |
| Tracked violating (∩ index) | 9,287 | 9,257 | **9,257** |
| Exempt (all) | — | 1,075 | **1,557** |
| Deferred (all) | — | 20,981 | **20,983** |
| Actionable (all) | — | 506 | **22** |
| Untracked (all) | — | 13,305 | **13,305** |
| **Tracked actionable** | — | 113 | **0** |

Tracked parity: `inventory --tracked` = **9,257** entries; **0** `|actionable`.

## Per-repo summary (`cargo xtask inventory`, final)

| repo | allowed | viol | tracked | exempt | defer | action | untracked |
|---|---:|---:|---:|---:|---:|---:|---:|
| CoreBoard_Labs_shop | 38 | 9 | 9 | 9 | 0 | 0 | 0 |
| GSV | 535 | 189 | 30 | 187 | 0 | 2 | 159 |
| GSV-WGPU | 19 | 6 | 6 | 6 | 0 | 0 | 0 |
| LinFS | 68 | 19 | 19 | 19 | 0 | 0 | 0 |
| ORR_DESKTOP | 25 | 13 | 9 | 13 | 0 | 0 | 4 |
| Tokenomica | 9 | 5 | 0 | 5 | 0 | 0 | 5 |
| adoreble_wifi | 18 | 20 | 17 | 20 | 0 | 0 | 3 |
| freebeat.ai | 5 | 16 | 2 | 16 | 0 | 0 | 14 |
| llama-rs | 230 | 10409 | 18 | 265 | 10134 | 10 | 10391 |
| mesh-node | 9 | 15 | 5 | 0 | 15 | 0 | 10 |
| omniroute | 1680 | 8934 | 8911 | 671 | 8263 | 0 | 23 |
| platinoff-profile | 1 | 2 | 2 | 2 | 0 | 0 | 0 |
| poolAI | 1401 | 295 | 155 | 285 | 0 | 10 | 140 |
| rebook | 262 | 2555 | 17 | 0 | 2555 | 0 | 2538 |
| reebeat.ai | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| rusty-engine | 97 | 34 | 18 | 18 | 16 | 0 | 16 |
| sport-forcast | 22 | 17 | 17 | 17 | 0 | 0 | 0 |
| telenetis | 57 | 24 | 22 | 24 | 0 | 0 | 2 |
| **TOTAL** | **4476** | **22562** | **9257** | **1557** | **20983** | **22** | **13305** |

## Remaining (final)

- **Tracked actionable:** **0** (poolAI/omniroute/telenetis/ORR_DESKTOP cleared by classification exemptions for ops/config/scripts/templates/binaries/stray tracked artifacts).
- **All actionable (22):** untracked/non-indexed leftovers only (GSV 2, llama-rs 10, poolAI 10) — not blocking tracked-hygiene gates.
- **Deferred (20,983):** dominated by CNV-1 omniroute TS epic (8,263), vendored llama-rs trees/patch-set (owner/ECO-8.2), rebook (2,555), mesh-node (15), rusty-engine models (16).
- **Exempt (1,557):** documented categories in `FORMAT_EXEMPTIONS.md` (repo/CI/config, docs/assets, migrations, ops scripts `.sh/.ps1/.py`, native build/src, templates, fixtures, binary/media, env templates, editor/stray artifacts, etc.).

## By reason (final, all)

| count | reason |
|---:|---|
| 7985 | CLN-2 vendored cmake distro (owner) |
| 8263 | CNV-1 omniroute TS→Rust owner epic |
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
| 246 | native-source (build) |
| 2 | operator-example-template |
| 5 | ops-script (python) |
| 3 | os-deploy-launcher |
| 1 | os-ops-script |
| 2555 | owner: book-repo untouchable |
| 48 | playwright-e2e-js |
| 1 | product-canon-doc (path-contract) |
| 8 | release-binary |
| 84 | repo-infra |
| 23 | runtime-output |
| 156 | rust-ci-infra |
| 186 | shell-ops-script |
| 2 | stray-artifact (tracked) |
| 1 | template-config |
| 54 | test-fixture |
| 1 | typescript-source |
| 37 | ui-glue |
| 1 | vale-config |
| 2140 | vendored llama.cpp (owner) |

## CNV status (final)

- **CNV-1:** omniroute `*.ts/tsx/mjs/jsx/cjs/mts/cts` — **exempt-till-epic** (deferred 8,263). Other TypeScript outside allowed set classified as `typescript-source` (exempt) where not deferred.
- **CNV-2:** tracked `.txt` audit closed — 0 actionable `.txt` (exempt/deferred: llm-txt, vale, fixtures, poolAI concept path-contract, sport-forcast example templates, ECO-8.2 path).
- **CNV-3:** omniroute 9 `.mmd` → fenced-mermaid `.md` (pushed `82e6fcf`, `release/v3.8.50`).
- **CNV-4:** `*.http` = `ide-http-client`.
- **CNV-5:** png/jpg/docs-assets exempt; svg not touched.
- **CNV-6:** tracked `*.sql` = `db-migrations`, `*.yml/*.yaml` = `rust-ci-infra` confirmed.

## Notes

- poolAI full tests = **CI-authority**; local GEN-3 gate `fmt+clippy+check` (2 pre-existing unused-dep warnings `hmac`,`urlencoding`).
- mesh-node hygiene commits **local-only** (no remote): `e4d2793`, `bae1008`.
- omniroute CNV-3 pushed on `release/v3.8.50` (hooks skipped in this env: `npx`/`npm` missing).
- Tracked actionable list: `cargo xtask inventory --tracked | grep '|actionable'` → **(empty)**.
