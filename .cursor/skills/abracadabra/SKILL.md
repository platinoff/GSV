---
name: abracadabra
description: >-
  Trigger words абракадабра, abrakadabra, or abracadabra start a VDT drain
  session. FIRST discover products from the live environment (workspace folders
  + sibling git repos), ask which one to work with, THEN run project-scan →
  drain → one commit + push. Host workspace is GSV (`S:\rust\GSV`). Use when
  the owner writes абракадабра or abrakadabra (or abracadabra) in a new
  session (Cursor or OpenCode).
metadata:
  audience: gsv-vdt-kit
  clients: cursor-opencode
---

# «абракадабра» / `abrakadabra` — VDT drain session (GSV host)

Works the same in **Cursor** and **OpenCode**. Git canon for this skill is
**`S:\rust\GSV/.agents/skills/abracadabra/`**. Client copies under `.cursor/skills/`
and `.opencode/skills/` must stay identical.

**Open this folder as the Cursor workspace:** `S:\rust\GSV` (kit entry).
Do not assume the product is GSV just because the window is GSV.

Kit split: [`docs/gsv/GSV_VDT_KIT.md`](../../../docs/gsv/GSV_VDT_KIT.md).

## Step 0 — Discover environment projects (ALWAYS first)

When the owner writes `абракадабра` **or** `abrakadabra` (same session;
`abracadabra` is the skill folder name), **before anything else** discover which
projects are actually on this machine, then ask which one to work with.

Do **not** hardcode a two-option `gsv | poolai` list. The question UI must show
the projects the agent can see in the environment.

### 0a. Scan (MSYS2 bash)

```bash
C:\msys64\usr\bin\bash.exe -lc 'cd /s/rust/GSV && cargo xtask products'
```

The script merges, in order:

1. Folders in `gsv.code-workspace` (Cursor / OpenCode multi-root).
2. Sibling **git** repos next to this kit (typically `S:/rust/*` with `.git`).
3. This kit itself (`S:/rust/GSV`).

Each row: `id`, `name`, `path`, `kind` (`rust` / `node` / `git` / `folder`),
`registered` (`yes` if a row exists in [`docs/gsv/PRODUCTS.md`](../../../docs/gsv/PRODUCTS.md)).

Example from this machine (changes when new repos appear):

| id | name | path | kind | registered |
|----|------|------|------|------------|
| gsv | GSV | `S:/rust/GSV` | rust | yes |
| poolai | poolAI | `S:/rust/poolAI` | rust | yes |
| omniroute | omniroute | `S:/rust/omniroute` | node | no |

If the script fails, fall back: read `gsv.code-workspace` + `ls` the parent of
GSV for directories that contain `.git`. Still do **not** invent a fixed pair.

### 0b. Ask (one click)

Use the host question UI. One option **per discovered row**. Label format:

`{name} — {path} ({kind} · registered|discovered)`

- **Cursor:** `AskQuestion`
- **OpenCode:** `question`

Prompt: **«Проєкти з цього середовища. З яким працюємо?»**

`PRODUCTS.md` is **enrichment** (HANDOFF, test command, ratio) — not the exclusive
list. A discovered repo that is not registered is still a valid choice.

### 0c. After the owner picks

| Pick | What to do |
|------|------------|
| **registered rust** (`gsv`, `poolai`, …) | Use that row in PRODUCTS.md: HANDOFF, NEXT, test command, ratio. Then the product flow below. |
| **registered node** (`omniroute`, …) | Use that row. S0 + git in **that** tree. **No** PH-S* invent. Tests from the row (`npm test`). |
| **discovered, not registered** | S0 disk + `git fetch` in **that** tree. Do **not** invent PH-S* / FM drain. Report kind (rust/node), ask whether to add a PRODUCTS.md row, or do a narrow owner-stated task in that repo. |

New **registered** products: add a row to PRODUCTS.md (root, handoff, test, ratio).
Discovery will pick them up automatically from disk / workspace; no need to
hardcode options in this skill. Implementation: `src/boxes/products.rs` /
`cargo xtask products` (not a `.sh` script).

## gsv flow

1. S0 disk (GSV has its own `target/` at `S:/rust/GSV`): `df -h /s` → `cargo clean` if needed → `git fetch` + GSV HANDOFF.
2. Project scan: warnings first (`cargo clippy --all-targets` in `S:/rust/GSV`) → `docs/gsv/GSV_TECH_ROADMAP.md` unchecked rows → gaps → next band. **VDT kit** (`.cursor/rules`, `.agents/skills`) is an in-tree product gap until band close.
3. Drain next band (≤10 open PH-S*; no mid-drain push). Work in **`S:/rust/GSV`**.
4. Vision close: GSV HANDOFF + GSV NEXT → `cargo xtask sync` then `cargo xtask sync --check`. Speeds/Rust: `cargo xtask record-speed` + `cargo xtask record-rust`.
5. Test: `cargo test` in `S:/rust/GSV` — do **not** kill `target/live/` copy (only stop `target/debug/gsv-server.exe` if that file is the listener). Disk guard: `cargo xtask disk`.
6. Git (end of session): one commit in the GSV repo. Push `origin` (`https://github.com/platinoff/GSV`) if the remote exists.

## poolai flow

Work with **absolute paths** under `S:/rust/poolAI` (this window’s default cwd may be GSV).

1. S0 disk: `df -h /s` + `bash S:/rust/poolAI/scripts/check_target_disk.sh` → `cargo clean` if needed → `git fetch` in PoolAI → HANDOFF → FM §1–§5.1 → NEXT_SESSION → `poolai-vision-sync --check` ok.
2. Project scan (if §5.12 < 10): warnings/diagnostics first → concept → FM §5.1 → architect → roadmaps → gaps → code → 10 PH-S* into §5.12.
3. Drain all open PH-S* (no mid-drain push).
4. Vision close: FM §5.12 ✅ + HANDOFF + NEXT → one `poolai-vision-sync` → rev from manifest → `--check`.
5. Test: one `cargo fmt --all` → one `cargo test-ci` (`K8S_OPENAPI_ENABLED_VERSION=1.28`, `CARGO_TARGET_DIR=/s/rust/poolAI/target`) → `record-test-ci-speed.sh` + `record-rust-diagnostics.sh`.
6. Git (end of session): one commit in **PoolAI** → `git push origin main` + summary in chat.

## omniroute flow (node, registered)

Work with **absolute paths** under `S:/rust/omniroute`. Do **not** invent PH-S* / FM drain. Do **not** copy this kit into omniroute.

1. S0 disk: `df -h /s` → `git fetch` in omniroute → read `AGENTS.md` + `CONTRIBUTING.md` + `docs/ROADMAP.md`.
2. Owner-stated task only (no GSV band). Tests: `npm test` (focused file first per AGENTS.md).
3. One commit in **omniroute** if there is work → push if remote exists → summary in chat.

Ratio: n/a (node). `gsv-loc-audit` does not apply.

## Hard rules (all products)

- **No** `git add -A`; stage only sprint files.
- **No** push mid-drain / mid-scan; push + summary always last step.
- **No** parallel `cargo` (file lock). Separate `target/` per product.
- **Never** stage: `.env*`, `*.pem`/`*.key`, `certs/*.pem`, `data/audit/*` (except `.gitkeep`), `comitmsg/*` except `comitmsg/README.md`.
- Warnings >0 or errors >0 fixable → 1–3 PH-S* at the top of the band.
- Shell is **MSYS2 bash**, not PowerShell: `C:\msys64\usr\bin\bash.exe -lc '…'`.

## See also

- Kit: `docs/gsv/GSV_VDT_KIT.md`
- Registry: `docs/gsv/PRODUCTS.md` (enrichment; discovery is `cargo xtask products`)
- MCP horizon: `docs/gsv/GSV_MCP_OPENBOT.md`
- GSV: `docs/NEXT_SESSION_PROMPT.md`, `docs/GSV_ROLES.md`, `docs/gsv/GSV_TECH_ROADMAP.md`
- PoolAI: `S:/rust/poolAI/docs/development/NEXT_SESSION_PROMPT.md`, `S:/rust/poolAI/docs/catalog/FUNCTION_MANAGEMENT.md`
