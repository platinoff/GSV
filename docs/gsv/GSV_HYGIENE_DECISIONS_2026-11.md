# GSV Ecosystem Hygiene Decisions — 2026-11 (ECO-4 follow-up)

Документ класифікує файли екосистеми для sweep-мислення (ECO-4 + follow-up).
Метод: окремими класами, щоб не чистити production/concept-артефакти як junk.

## Scope

- Product repos under `/s/rust/*` (GSV, poolAI, GSV-WGPU, llama-rs, …).
- Ecosystem surface: docs, skills, rules, MCP, tickets.

## Classification

1. CLEANABLE (auto-sweep allowed, owner-confirmed earlier):
   - `*.zip`, `*.mp4` — junk (rust-sdk-main, recordings).
   - scratch/tmp, `*.py`, `*.ps` — non-allowed formats / scratch.

2. OWNER-GATED (flagged for owner review — NO auto-delete / auto-move without owner command):
   - `poolAI_concept.txt` (+ similar concept/draft artifacts).
   - Non-registered dirs: `/s/rust/{freebeat.ai, reebeat.ai, Tokenomica, platinoff-profile}`.

3. CANON-PROTECTED (never sweep):
   - Allowed source formats: `*.rs`, `*.md`, `*.mdc`, `*.json`, `*.js`, `*.wasm`.
   - Kit docs, HANDOFF/Routes/MCP/vision canon, tickets (`docs/gsv/tickets.jsonl`).

## ECO-6.7 py/ps survey (2026-11-07) — classification of the 30 found files

Full survey (`target/eco67_survey.sh`): 5 `.py` (omniroute), 24 `.ps1`
(poolAI 17, rebook 7), 0 scratch. **None auto-sweepable — all functional:**

| Class | Files | Verdict |
|---|---|---|
| omniroute product tooling | `cdp-bridge.py`, `scripts/i18n/*.py` (3), `scripts/sre/tcp-close-analyzer.py` | KEEP — tracked, referenced by Dockerfile / svc-de-run / i18n mjs / docs; omniroute is TS-first with own local rules |
| poolAI .sh/.ps1 twin pairs (9 ps1) | `bin/{capture-p2b-single-host-metrics,e2e-playwright,open-docs-vision,run-lan-nodes,run-poolai,run-virtual-node-dev,verify-dev-stand,verify-lan-prep}.ps1`, `scripts/test_deployment.ps1` | KEEP (owner-gated port-to-sh band if owner wants .sh-only) — heavily referenced: AGENTS.md, .cursor/commands, docs (up to 18 refs) |
| poolAI solo ps1 (7) | `bin/{open-docs-vision-cursor,poolai-msys}.ps1`, `scripts/{run,run-lan-nodes,setup_msvc_environment,setup_msys2_path,setup_rust_environment}.ps1` | KEEP — Windows bootstrap/env setup for the box (needed before bash exists) |
| rebook book pipelines | `scripts/coloring-png-to-jpeg.ps1` (tracked, invoked from `src/coloring_kdp.rs`) + 6 untracked `workspace/books/*/{binarize-lines,paint-cover-back}.ps1` | KEEP — KDP book production; no .sh twin; port = product work |

Decision: ECO-6.7 remaining surface shrinks to **0 deletions**; the "cleanup
txt/ps/py" ask is satisfied for scratch (done in ECO-4/earlier ECO-6.7 partial).
Tracked py/ps tooling is product-local, per plugin-local-rules law — no global
sweep. If owner wants format discipline bands (delete .ps1 twins / port py),
each becomes its own ticket.

## Decisions

- `poolAI_concept.txt` → **owner-gated**: keep / move pending owner decision (ECO-4).
- All owner-gated classes require explicit owner instruction before mutation.
- Canon-protected classes require owner instruction before any mutation.
- New junk classes → new ticket + owner confirmation before action.

## Status

- ECO-4 owner-gated items: open (owner decision pending): token rotation, `poolAI_concept.txt` keep/move, infra keep-live (`:8091` down).
- Cleaned (previously approved): zips, mp4, scratch (ECO-4).
- ECO-6.7 cleanup: **closed 2026-11-08** — full py/ps survey повернув 30 файлів, 0 sweepable (усі функціональні; таблиця вище). Відкриті опції повідомлені як окремі тікети, якщо owner захоче формат-дисципліну.
- `target/` scratch-скрипти (probe/verify/dry-run) автоматично не чистяться — `target/` gitignored, не в реєстрі hygiенно поверхні; ручна періодична зачистка за потреби.
