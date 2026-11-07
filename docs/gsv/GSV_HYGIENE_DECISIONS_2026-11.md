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

## Decisions

- `poolAI_concept.txt` → **owner-gated**: keep / move pending owner decision (ECO-4).
- All owner-gated classes require explicit owner instruction before mutation.
- Canon-protected classes require owner instruction before any mutation.
- New junk classes → new ticket + owner confirmation before action.

## Status

- ECO-4 owner-gated items: open (owner decision pending).
- Cleaned (previously approved): zips, mp4, scratch (ECO-4).
- ECO-6.7 cleanup: partial (txt/ps/py sweep in progress).
