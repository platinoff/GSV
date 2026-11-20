# Format exemptions registry (EXM-1)

**Status:** active · **Date:** 2026-11-20
**Plan:** [`../superpowers/plans/2026-11-20-format-hygiene-tickets.md`](../superpowers/plans/2026-11-20-format-hygiene-tickets.md)
**Machine reader:** `cargo xtask inventory` (`--json`, `--tracked`) — the
reason strings below are the exact `by_reason` keys the scanner emits.

The GSV allowed-format canon (AGENTS.md): **`.rs` · `.md` · `.mdc` · `.json`
· `.js` · `.wasm`** (Rust 95–100 % / wasm 0–5 %). Everything else is a
violation **unless** it carries a reason from this registry. Two non-action
classes exist:

- **Exempt** — permanently documented exception (this file). No ticket.
- **Deferred** — known violation parked in a scheduled/owner-gated stream
  (plan tickets CLN/CNV/GEN, ECO-8.2, owner fences). Counted, never silent.

`tracked` violations of the **secret shapes** (`.env`, `*.pem`, `*.key`,
`*.token`) are **never** exempt — they are SEC-actionable even when a rule
below would otherwise match.

## Exempt reasons

| Reason | Scope | Why it stays |
|--------|-------|--------------|
| `rust-ci-infra` | `*.toml`, `*.lock`, `*.yml`, `*.yaml` | Cargo manifests/lockfiles and CI workflow YAML are the Rust toolchain's own formats; no `.rs` equivalent exists. |
| `ui-glue` | `*.html`, `*.css` | AGENTS.md: "UI: thin HTML/CSS/JS glue in `ui/`". Explicitly exempted in `src/boxes/ratio.rs`. |
| `repo-infra` | `.gitignore`, `.gitattributes`, `.dockerignore`, `.gitkeep`, `.npmrc`, `.nvmrc`, `.editorconfig`, `.prettierrc*`, `.eslintrc`, `LICENSE*`, `Makefile`, `CMakeLists.txt`, `Dockerfile`, `gsv.code-workspace` | VCS/tooling config read by name by external tools; `LICENSE` must keep its canonical name. |
| `metadata-ring` | `*.jsonl` | Append-only JSONL rings (tickets board, fingerprints, telemetry, sys-history) — data, not harness; per AGENTS.md `data/*` staging rules. |
| `binary-artifact` | `*.bin`, `*.gguf`, `*.safetensors`, `*.img`, `*.xz`, `*.pth`, `*.onnx`, `*.mp4`, `*.mov`, `*.webm`, `*.pdf` | Model weights and their source artifacts — owner rule: never convert/touch. |
| `release-binary` | `*.exe`, `*.dll` | Build output of registered pipelines (production output exception in AGENTS.md). |
| `runtime-output` | `*.log`, `*.pid` | Process output; gitignored by the log-hygiene pass. |
| `docs-assets` | `*.svg`, `*.png`, `*.jpg`, `*.jpeg`, `*.gif`, `*.webp`, `*.ico` | Pictures/illustrations for docs — no textual format carries pixels. (Inline `<svg>` in `.md` = CNV-5, owner decision.) |
| `db-migrations` | `*.sql` | SQL is the native dialect of migrations; no Rust replacement. |
| `os-deploy-launcher` | `*.vbs`, `*.iss`, `*.service` | Windows hidden-window launchers (watchdog/cycle-watch pattern), Inno Setup installer, systemd units — OS-native by definition. |
| `ide-http-client` | `*.http` | IDE REST-client collections (VS Code REST Client / IntelliJ HTTP) — tool-specific, not convertible to `.md` without losing executable semantics (CNV-4). |
| `llm-txt-convention` | basename `llm.txt`, `llms.txt` | LLM-crawler/landing convention (llmstxt.org style); filenames and cross-links are tooling/sync contracts (CNV-2). |
| `vale-config` | `/.vale/**` | Vale prose-linter vocab/style config (tool-specific filenames/paths). |
| `test-fixture` | `*/tests/data/**`, `**/fixtures/**` | Golden inputs and fixture samples consumed verbatim by tests (qwen25/qwen35 goldens, RTK compression samples, etc.). |
| `product-canon-doc (path-contract)` | `docs/concept/poolAI_concept_root.txt` | poolAI primary concept doc: `.txt` path is a stable contract referenced by rules/skills/docs/scripts (CNV-2); kept at canonical path. |
| `operator-example-template` | `odds.example.txt`, `sponsors.example.txt` | User-copied runtime templates (read by tests via `include_str!` / documented copy paths); keep filename and format (CNV-2). |
| `data-export` | `*.csv` | Tabular exports (usage/tokenomics data), not source. |
| `patch-set` | `*.patch` | ECO-8.2 llama-cpp patch-set applied by `cargo xtask` — the patch format is the deliverable. |
| `playwright-e2e-js` | `poolAI/e2e/**` | poolAI browser E2E — owner-gated JS suite (E2), converted only with an owner epic. |
| `env-template` | `.env.example`, `.env.sample`, `.env.template` | Documented placeholders (`CHANGEME`), never real secrets (SEC-2 verified). |
| `local-secret (untracked)` | untracked `.env` / `*.pem` / `*.key` / `*.token` | Local secrets, gitignored by design (SEC band). Tracked copies are SEC-actionable, not exempt. |

## Deferred streams (known, scheduled)

| Reason | Ticket | Content |
|--------|--------|---------|
| `CLN-1 next-build output (untracked)` | CLN-1 | `omniroute/.build/**` — Next.js build cache (~19 k files, gitignored). **Removed 2026-11-20** (~2.2 GB). |
| `CLN-2 vendored cmake distro (owner)` | CLN-2 | `llama-rs/.cmake/**` — vendored CMake tree (~130 MB on disk). **Kept (2026-11-20, verified):** `.cargo/config.toml` pins `CMAKE` to `S:\rust\llama-rs\.cmake\cmake-3.31.9-windows-x86_64\bin\cmake.exe` and docs reference it — build dependency, not deleted; gitignored. |
| `CLN-3 test artifacts (untracked)` | CLN-3 | `poolAI/.tmp/**` — test scratch (~198). **Removed 2026-11-20** (untracked, gitignored). |
| `CNV-1 omniroute TS→Rust owner epic` | CNV-1 | omniroute `*.ts/tsx/mjs/jsx/cjs` (~8.3 k tracked) — **owner decision 2026-11-20: exempt-till-epic** (TS→Rust = separate epic, not this sweep). |
| `CNV-5 docs assets (owner)` | CNV-5 | `*.png/*.jpg/*.jpeg` exempt as `docs-assets`; `*.svg` not touched (owner 2026-11-20). |
| `CNV-6 sql/yml/yaml (confirmed)` | CNV-6 | Tracked `*.sql` = `db-migrations`, tracked `*.yml/*.yaml` = `rust-ci-infra` (already exempt in inventory/registry, 2026-11-20). |
| `owner: book-repo untouchable` | — | `rebook/**` — book products/covers (owner rule). |
| `GEN-4 mesh-node scripts/images (owner)` | GEN-4 | mesh-node `*.sh` + flash `images/` — **done 2026-11-20** (owner: doc in `.md`, `.sh` deleted, flash img/log artifacts removed; local commit `bae1008`, no remote). |
| `ECO-8.2 llama-cpp patch-set (owner)` | ECO-8.2 | `llama-rs/patches/**` — sys bump stream, separate band. |
| `vendored llama.cpp (owner)` | CLN-2/8.2 | `llama-rs/llama.cpp-master/**` — vendored upstream tree. |
| `model weights/source artifacts` | — | `rusty-engine/data/models/**` — never convert. |

## Rules for adding an exemption

1. New reason → this table **and** `exempt_reason()` in
   `src/boxes/inventory.rs` in the **same commit**, plus a unit test in
   `classify_path_rules`.
2. A deferral needs a plan ticket id in the row — no ticket, no deferral.
3. Secret shapes can never be added to the exempt table (SEC-2 invariant).
4. `cargo xtask inventory` must stay at parity with the shell baseline
   (`tracked` exact; `violating` delta explained) — VER-1 gate.
