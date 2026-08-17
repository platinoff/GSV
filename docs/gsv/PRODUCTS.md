# Product registry (GSV VDT kit)

Registered Rust products the host skill [`abracadabra`](../../.agents/skills/abracadabra/SKILL.md) can **drain** (HANDOFF, tests, ratio).

**Discovery is not this table.** When the owner writes `абракадабра` or `abrakadabra`, the agent runs
[`scripts/list-vdt-products.sh`](../../scripts/list-vdt-products.sh) and asks about
**projects visible in the environment** (workspace folders + sibling git repos).
This file only **enriches** a pick that is already registered.

**Open Cursor on `S:\rust\GSV`** (or `gsv.code-workspace`). The window being GSV does **not** pick the product — the environment list + AskQuestion does.

| id | Root | HANDOFF | NEXT | Test command | Ratio |
|----|------|---------|------|--------------|-------|
| **gsv** | `S:/rust/GSV` | [`docs/HANDOFF_NEW_SESSION.md`](../HANDOFF_NEW_SESSION.md) | [`docs/NEXT_SESSION_PROMPT.md`](../NEXT_SESSION_PROMPT.md) | `cargo fmt -- --check` → `cargo clippy --all-targets` → `cargo test` → `cargo run --bin gsv-loc-audit -- --stretch-96` (stop `gsv-server` first) | Rust **95–100%** (`--stretch-96` ≥96%) |
| **poolai** | `S:/rust/poolAI` | `S:/rust/poolAI/docs/development/HANDOFF_NEW_SESSION.md` | `S:/rust/poolAI/docs/development/NEXT_SESSION_PROMPT.md` | `K8S_OPENAPI_ENABLED_VERSION=1.28 cargo test-ci` (`CARGO_TARGET_DIR=/s/rust/poolAI/target`) | Rust **90–95%** |

Discovered but **not** in this table (example: `S:/rust/omniroute`) → S0 + git in that tree; no PH-S* drain until a row is added.

## Shared vs product

| Lives in GSV (this kit) | Lives in the product repo |
|-------------------------|---------------------------|
| `.agents/skills/` (except product-only skills) | FM / concept / DIGEST |
| Generic `.cursor/rules/` (S0, MSYS2, git, rust style) | Product test aliases (`test-ci`, Playwright admin, OpenAPI gap) |
| `абракадабра` / `abrakadabra` router (discover → ask) | Product HANDOFF / NEXT / roadmap journal |

## New product checklist

1. Sibling git repo under `S:/rust/<name>` (discovery will list it automatically).
2. Row in this table **only if** it should get a full VDT drain (root, handoff, test, ratio).
3. Optional folder in `gsv.code-workspace` so it also appears as a workspace root.
4. Do **not** copy this whole kit into the new repo.
5. Do **not** add a hardcoded option in the abracadabra skill — the scan is the list.

Canon: [`GSV_VDT_KIT.md`](./GSV_VDT_KIT.md).
