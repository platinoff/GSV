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
| `repo-infra` | `.gitignore`, `.gitattributes`, `.dockerignore`, `.cbmignore`, `.gitkeep`, `.npmrc`, `.nvmrc`, `.editorconfig`, `.prettierrc*`, `.eslintrc`, `.npmignore`, `.node-version`, `.trivyignore`, `.vale.ini`, `.cursorrules`, `CODEOWNERS`, `LICENSE*`, `gsv.code-workspace`, `Makefile`, `CMakeLists.txt`, `Dockerfile`, `sonar-project.properties`, `flake.nix`, `.husky/**` | VCS/CI/tooling hooks and repo config read by name by external tools. |
| `rust-ci-infra` | `*.toml`, `*.lock`, `*.yml`, `*.yaml` | Cargo manifests/lockfiles and CI workflow/config YAML — native formats. |
| `repo-config` | `*.properties`, `*.ini`, `*.conf`, `*.nix` | Tool/runtime config in native formats. |
| `ui-glue` | `*.html`, `*.css` | Thin UI glue (AGENTS.md; `ratio.rs` exempt). |
| `docs-assets` | `*.svg`, `*.png`, `*.jpg`, `*.jpeg`, `*.gif`, `*.webp`, `*.ico` | Binary/pixel assets for docs (CNV-5: png/jpg exempt, svg not touched). |
| `metadata-ring` | `*.jsonl` | Append-only rings/telemetry/tickets metadata. |
| `binary-artifact` | `*.exe`, `*.dll`, `*.bin`, `*.gguf`, `*.safetensors`, `*.img`, `*.xz`, `*.pth`, `*.onnx`, `*.mp4`, `*.mov`, `*.webm`, `*.pdf`, `*.icns`, `*.mp3`, `*.wav`, `*.m4a` | Build/model/media binary artifacts (weights never converted). |
| `release-binary` | `*.exe`, `*.dll` | Covered by `binary-artifact` (kept for registry clarity). |
| `runtime-output` | `*.log`, `*.pid` | Process output. |
| `db-migrations` | `*.sql` | Native migration dialect. |
| `os-deploy-launcher` | `*.vbs`, `*.iss`, `*.service` | OS-native launchers/installers/units. |
| `shell-ops-script` | `*.sh` | Shell ops/CI/deploy wrappers (MSYS2/bash); long-term prefer Rust `cargo xtask`, but tracked foreign-repo scripts exempted in this sweep. |
| `os-ops-script` | `*.ps1` | OS-specific ops scripts. |
| `ops-script (python)` | `*.py` | Python ops/CI/tooling scripts in foreign repos. |
| `native-build-config` | `*.gyp` | Native addon/build config. |
| `native-source (build)` | `*.c`, `*.h` | Native sources required by build/tooling. |
| `deploy-config (quadlet)` | `*.container`, `*.volume`, `*.network` | Podman Quadlet units. |
| `template-config` | `*.tpl` | Template files (e.g. Helm helpers). |
| `config-template` | `*.example`, `*.sample`, `*.template` (non-env basename patterns) | Documented config templates/placeholders. |
| `ide-http-client` | `*.http` | IDE REST-client collections (executable semantics). |
| `llm-txt-convention` | basename `llm.txt`, `llms.txt` | LLM-crawler landing convention / path contract. |
| `vale-config` | `/.vale/**` | Vale vocab/style config. |
| `test-fixture` | `*/tests/data/**`, `**/fixtures/**` | Golden/fixture inputs consumed verbatim by tests. |
| `product-canon-doc (path-contract)` | `docs/concept/poolAI_concept_root.txt` | poolAI primary concept doc (path contract). |
| `operator-example-template` | `odds.example.txt`, `sponsors.example.txt` | User-copied runtime templates read by tests/docs. |
| `data-export` | `*.csv` | Tabular exports. |
| `patch-set` | `*.patch` | Applied patch artifacts (ECO-8.2 stream). |
| `editor-backup` | `*.new`, `*.bak`, `*.orig` | Editor/merge backup artifacts. |
| `stray-artifact (tracked)` | specific tracked oddities (e.g. `poolAI/ -5`, `poolAI/0.0`) | Known tracked artifacts with non-canonical names; exempted to avoid false-actionable, not auto-deleted. |
| `typescript-source` | `*.ts`, `*.tsx`, `*.mts`, `*.cts` (when not deferred) | TypeScript sources outside allowed set; omniroute TS family deferred by CNV-1. |
| `playwright-e2e-js` | `poolAI/e2e/**` | poolAI browser E2E (owner-gated). |
| `env-template` | `.env.example`, `.env.sample`, `.env.template`, `.env.*.example|sample|template` | Documented placeholders, never real secrets (SEC-2). |
| `local-secret (untracked)` | untracked `.env` / `*.pem` / `*.key` / `*.token` | Local secrets, gitignored (SEC). Tracked secret shapes are **SEC-actionable**. |

## Deferred streams (known, scheduled)

| Reason | Ticket | Content |
|--------|--------|---------|
| `CLN-1 next-build output (untracked)` | CLN-1 | `omniroute/.build/**` — Next.js build cache (~19 k files, gitignored). **Removed 2026-11-20** (~2.2 GB). |
| `CLN-2 vendored cmake distro (owner)` | CLN-2 | `llama-rs/.cmake/**` — vendored CMake tree (~130 MB on disk). **Kept (2026-11-20, verified):** `.cargo/config.toml` pins `CMAKE` to `S:\rust\llama-rs\.cmake\cmake-3.31.9-windows-x86_64\bin\cmake.exe` and docs reference it — build dependency, not deleted; gitignored. |
| `CLN-3 test artifacts (untracked)` | CLN-3 | `poolAI/.tmp/**` — test scratch (~198). **Removed 2026-11-20** (untracked, gitignored). |
| `CNV-1 omniroute TS→Rust owner epic` | CNV-1 | omniroute `*.ts/tsx/mjs/jsx/cjs/mts/cts` (tracked) — **owner decision 2026-11-20: exempt-till-epic** (TS→Rust = separate epic, not this sweep). |
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
