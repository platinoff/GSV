# Product registry (GSV VDT kit)

Registered products the host skill [`abracadabra`](../../.agents/skills/abracadabra/SKILL.md) can **drain** (HANDOFF, tests, ratio). Node rows are enrichment only — no PH-S* invent, no GSV loc-audit.

**Discovery is not this table.** When the owner writes `абракадабра` or `abrakadabra`, the agent runs
[`cargo xtask products`](../../src/boxes/xtask.rs) and asks about
**projects visible in the environment** (workspace folders + sibling git repos).
This file only **enriches** a pick that is already registered.

**Open Cursor on `S:\rust\GSV`** (or `gsv.code-workspace`). The window being GSV does **not** pick the product — the environment list + AskQuestion does.

| id | Root | HANDOFF | NEXT | Test command | Ratio |
|----|------|---------|------|--------------|-------|
| **gsv** | `S:/rust/GSV` | [`docs/HANDOFF_NEW_SESSION.md`](../HANDOFF_NEW_SESSION.md) | [`docs/NEXT_SESSION_PROMPT.md`](../NEXT_SESSION_PROMPT.md) | `cargo fmt -- --check` → `cargo clippy --all-targets` → `cargo test` → `cargo run --bin gsv-loc-audit -- --stretch-96` (do **not** kill `target/live/` copy) | Rust **95–100%** (`--stretch-96` ≥96%) |
| **poolai** | `S:/rust/poolAI` | `S:/rust/poolAI/docs/development/HANDOFF_NEW_SESSION.md` | `S:/rust/poolAI/docs/development/NEXT_SESSION_PROMPT.md` | `K8S_OPENAPI_ENABLED_VERSION=1.28 cargo test-ci` (`CARGO_TARGET_DIR=/s/rust/poolAI/target`) | Rust **90–95%** |
| **omniroute** | `S:/rust/omniroute` | `S:/rust/omniroute/AGENTS.md` | `S:/rust/omniroute/docs/ROADMAP.md` | `npm test` (focused: `node --import tsx/esm --test tests/unit/<file>.test.ts`) | n/a (node; GSV loc-audit does not apply) |
| **orr_desktop** | `S:/rust/ORR_DESKTOP` | `S:/rust/ORR_DESKTOP/docs/HANDOFF_NEW_SESSION.md` | `S:/rust/ORR_DESKTOP/docs/NEXT_SESSION_PROMPT.md` | `cargo fmt -- --check` → `cargo clippy --all-targets` → `cargo test` (windows-gnu; ffmpeg via `ORR_FFMPEG` or `%PATH%`) | Rust **95–100%** |
| **linfs** | `S:/rust/LinFS` | `S:/rust/LinFS/docs/HANDOFF_NEW_SESSION.md` | `S:/rust/LinFS/docs/NEXT_SESSION_PROMPT.md` | `cargo fmt -- --check` → `cargo clippy --all-targets` → `cargo test` → `cargo run --bin linfs-loc-audit -- --stretch-96` (windows-gnu) | Rust **95–100%** (`--stretch-96` ≥96%) |
| **telenetis** | `S:/rust/GSV/telenetis` | `S:/rust/GSV/docs/HANDOFF_NEW_SESSION.md` | `S:/rust/GSV/docs/NEXT_SESSION_PROMPT.md` | `cargo fmt -- --check` → `cargo clippy --all-targets` → `cargo test` | Rust **95–100%** |
| **llama-rs** | `S:/rust/llama-rs` | `S:/rust/llama-rs/docs/HANDOFF.md` | `S:/rust/llama-rs/docs/ROADMAP.md` | `cargo fmt -- --check` → `cargo clippy --all-targets` → `cargo test` | Rust **95–100%** |
| **rebook** | `S:/rust/rebook` | `S:/rust/rebook/PRODUCTION_GUIDE.md` | `S:/rust/rebook/PRODUCTION_GUIDE.md` | `cargo fmt -- --check` → `cargo clippy --all-targets` → `cargo test` | Rust **95–100%** |

Discovered but **not** in this table → S0 + git in that tree; no PH-S* drain until a row is added. OmniRoute is registered (band 149, owner-opt-in).

## Shared vs product

| Lives in GSV (this kit) | Lives in the product repo |
|-------------------------|---------------------------|
| `.agents/skills/` (except product-only skills) | FM / concept / DIGEST |
| Generic `.cursor/rules/` (S0, MSYS2, git, rust style) | Product test aliases (`test-ci`, Playwright admin, OpenAPI gap) |
| `абракадабра` / `abrakadabra` router (discover → ask) | Product HANDOFF / NEXT / roadmap journal |

## New product checklist

1. Sibling git repo under `S:/rust/<name>` (discovery will list it automatically).
2. Row in this table **only if** it should get a registered drain (Rust: HANDOFF + PH-S*; node: AGENTS + `npm test`, no PH-S*).
3. Optional folder in `gsv.code-workspace` so it also appears as a workspace root.
4. Do **not** copy this whole kit into the new repo.
5. Do **not** add a hardcoded option in the abracadabra skill — the scan is the list.

Canon: [`GSV_VDT_KIT.md`](./GSV_VDT_KIT.md).
