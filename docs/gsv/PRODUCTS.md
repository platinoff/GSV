# Product registry (GSV VDT kit)

Registered Rust products the host skill [`abracadabra`](../../.agents/skills/abracadabra/SKILL.md) can drain.
**Open Cursor on `S:\rust\GSV`** (or `gsv.code-workspace`). The window being GSV does **not** pick the product — AskQuestion does.

Add a row here, then add an option in the abracadabra skill.

| id | Root | HANDOFF | NEXT | Test command | Ratio |
|----|------|---------|------|--------------|-------|
| **gsv** | `S:/rust/GSV` | [`docs/HANDOFF_NEW_SESSION.md`](../HANDOFF_NEW_SESSION.md) | [`docs/NEXT_SESSION_PROMPT.md`](../NEXT_SESSION_PROMPT.md) | `cargo fmt -- --check` → `cargo clippy --all-targets` → `cargo test` → `cargo run --bin gsv-loc-audit -- --stretch-96` (stop `gsv-server` first) | Rust **95–100%** (`--stretch-96` ≥96%) |
| **poolai** | `S:/rust/poolAI` | `S:/rust/poolAI/docs/development/HANDOFF_NEW_SESSION.md` | `S:/rust/poolAI/docs/development/NEXT_SESSION_PROMPT.md` | `K8S_OPENAPI_ENABLED_VERSION=1.28 cargo test-ci` (`CARGO_TARGET_DIR=/s/rust/poolAI/target`) | Rust **90–95%** |

## Shared vs product

| Lives in GSV (this kit) | Lives in the product repo |
|-------------------------|---------------------------|
| `.agents/skills/` (except product-only skills) | FM / concept / DIGEST |
| Generic `.cursor/rules/` (S0, MSYS2, git, rust style) | Product test aliases (`test-ci`, Playwright admin, OpenAPI gap) |
| `абракадабра` router | Product HANDOFF / NEXT / roadmap journal |

## New product checklist

1. Sibling git repo under `S:/rust/<name>`.
2. Row in this table (root, handoff, test, ratio).
3. Option in `.agents/skills/abracadabra/SKILL.md` Step 0.
4. Optional folder in `gsv.code-workspace`.
5. Do **not** copy this whole kit into the new repo.

Canon: [`GSV_VDT_KIT.md`](./GSV_VDT_KIT.md).
