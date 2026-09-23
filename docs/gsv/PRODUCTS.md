# Product registry (GSV VDT kit)

Registered **plugins** the host skill [`abracadabra`](../../.agents/skills/abracadabra/SKILL.md) can **drain** (HANDOFF, tests, ratio). Node rows are enrichment only — no PH-S* invent, no GSV loc-audit.

**Host vs plugin:** the rust folder that contains GSV is the host (`S:/rust/GSV`). Every other row is a **portable plugin** (own git tree, own `target/`). Canon: [`GSV_AGI_PATH.md`](./GSV_AGI_PATH.md).

**Discovery is not this table.** When the owner writes `абракадабра` or `abrakadabra`, the agent runs
[`cargo xtask products`](../../src/boxes/xtask.rs) and asks about
**projects visible in the environment** (workspace folders + sibling git repos under the rust folder).
This file only **enriches** a pick that is already registered.

**Open Cursor on `S:\rust\GSV`** (or `gsv.code-workspace`). The window being GSV does **not** pick the product — the environment list + AskQuestion does.

| id | Root | HANDOFF | NEXT | Test command | Ratio |
|----|------|---------|------|--------------|-------|
| **gsv** | `S:/rust/GSV` | [`docs/HANDOFF_NEW_SESSION.md`](../HANDOFF_NEW_SESSION.md) | [`docs/NEXT_SESSION_PROMPT.md`](../NEXT_SESSION_PROMPT.md) | `cargo fmt -- --check` → `cargo clippy --all-targets` → `cargo test` → `cargo run --bin gsv-loc-audit -- --stretch-96` (do **not** kill `target/live/` copy) | Rust **95–100%** (`--stretch-96` ≥96%) |
| **poolai** | `S:/rust/poolAI` | `S:/rust/poolAI/docs/development/HANDOFF_NEW_SESSION.md` | `S:/rust/poolAI/docs/development/NEXT_SESSION_PROMPT.md` | `K8S_OPENAPI_ENABLED_VERSION=1.28 cargo test-ci` (`CARGO_TARGET_DIR=/s/rust/poolAI/target`) | Rust **90–95%** |
| **omniroute** | `S:/rust/omniroute` | `S:/rust/omniroute/AGENTS.md` | `S:/rust/omniroute/docs/ROADMAP.md` | `npm test` (focused: `node --import tsx/esm --test tests/unit/<file>.test.ts`) | n/a (node; GSV loc-audit does not apply) |
| **orr_desktop** | `S:/rust/ORR_DESKTOP` | `S:/rust/ORR_DESKTOP/docs/HANDOFF_NEW_SESSION.md` | `S:/rust/ORR_DESKTOP/docs/NEXT_SESSION_PROMPT.md` | `cargo fmt -- --check` → `cargo clippy --all-targets` → `cargo test` (windows-gnu; ffmpeg via `ORR_FFMPEG` or `%PATH%`) | Rust **95–100%** |
| **linfs** | `S:/rust/LinFS` | `S:/rust/LinFS/docs/HANDOFF_NEW_SESSION.md` | `S:/rust/LinFS/docs/NEXT_SESSION_PROMPT.md` | `cargo fmt -- --check` → `cargo clippy --all-targets` → `cargo test` → `cargo run --bin linfs-loc-audit -- --stretch-96` (windows-gnu) | Rust **95–100%** (`--stretch-96` ≥96%) |
| **telenetis** | `S:/rust/GSV/telenetis` | `S:/rust/GSV/telenetis/docs/HANDOFF_NEW_SESSION.md` | `S:/rust/GSV/telenetis/docs/NEXT_SESSION_PROMPT.md` | `cargo fmt -- --check` → `cargo clippy --all-targets` → `cargo test` | Rust **95–100%** |
| **llama-rs** | `S:/rust/llama-rs` | `S:/rust/llama-rs/docs/HANDOFF_NEW_SESSION.md` (journal: `docs/HANDOFF.md`) | `S:/rust/llama-rs/docs/ROADMAP.md` | `cargo fmt -- --check` → `cargo clippy --all-targets` → `cargo test` | Rust **95–100%** |
| **rebook** | `S:/rust/rebook` | `S:/rust/rebook/docs/HANDOFF_NEW_SESSION.md` | `S:/rust/rebook/docs/NEXT_SESSION_PROMPT.md` | `cargo fmt -- --check` → `cargo clippy --all-targets` → `cargo test` (EPUB path: `cargo run -- build-epub`) | Rust **95–100%** |

Keep-live (band 225): llama-rs writes `target/live/llama_heartbeat.json` when run with `GSV_LIVE=1` or `LLAMA_RS_HEARTBEAT=1` (PID, model, epoch, bin_version, 15s tick); GSV reads it via `keep_live.llama_rs` (fresh = age ≤ 60s). `gsv_products_scan` / `cargo xtask products` enrich this heartbeat path + freshness for llama-rs.

Discovered but **not** in this table → S0 + git in that tree; no PH-S* drain until a row is added. OmniRoute is registered (band 149, owner-opt-in).

## Shared vs plugin

| Lives in GSV (host kit) | Lives in the plugin repo |
|-------------------------|---------------------------|
| `.agents/skills/` (except product-only skills) | FM / concept / DIGEST |
| Generic `.cursor/rules/` (S0, MSYS2, git, rust style) | Product test aliases (`test-ci`, Playwright admin, OpenAPI gap) |
| `абракадабра` / `abrakadabra` / `agi` router (discover → ask) | Plugin HANDOFF / NEXT / roadmap journal |
| MCP sandbox, edge proxy, tickets, keep-live | Plugin runtime (`target/`, ports, secrets) |
| Global-first / IDE-agnostic auto-wire / PoolAI genetic pointer | Thin `AGENTS.md` from [`PLUGIN_AGENTS.md`](./PLUGIN_AGENTS.md) |

**Global first.** Any IDE / model / focused folder: read GSV `AGENTS.md` +
`GSV_AGI_PATH.md` before product HANDOFF. Local rules are product specifics
only. Open `gsv.code-workspace` (GSV first) so hub MCP stays present. PoolAI
is the genetic parent for multi-agent workflow DNA when that pattern dominates
— copy the idea, not the files.

## New plugin checklist

1. Sibling git repo under the rust folder (`S:/rust/<name>` on this box). Discovery lists it. Nested under GSV is an exception (Telenetis), not the pattern.
2. Row in this table **only if** it should get a registered drain (Rust: HANDOFF + PH-S*; node: AGENTS + `npm test`, no PH-S*).
3. Optional folder in `gsv.code-workspace` so it also appears as a workspace root.
4. Do **not** copy this whole kit into the plugin. Do **not** absorb plugin rules into GSV `AGENTS.md`.
5. Do **not** add a hardcoded option in the abracadabra skill — the scan is the list.
6. Plugin talks to the hub (`/mcp`, `/api/edge`, keep-live). Unplug = fail-open. Environment security first (sandbox stays GSV; no User MCP; no `:8091` off-box).

Canon: [`GSV_VDT_KIT.md`](./GSV_VDT_KIT.md) · [`GSV_AGI_PATH.md`](./GSV_AGI_PATH.md).
