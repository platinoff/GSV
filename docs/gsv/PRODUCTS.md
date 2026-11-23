# Product registry (GSV VDT kit)

Registered **plugins** the host skill [`abracadabra`](../../.agents/skills/abracadabra/SKILL.md) can **drain** (HANDOFF, tests, ratio). Node rows are enrichment only — no PH-S* invent, no GSV loc-audit.

> **Екосистема як пара «тато і мама» (flip 2026-11-17):** [`GSV_ECOSYSTEM_SYMBIOSIS.md`](./GSV_ECOSYSTEM_SYMBIOSIS.md) — poolAI (FM/management план, **тато** — генетичний фонт коду) / GSV (операційний план, **мама** — хаб, народжує плагіни); живий стан 2026-11-17, дублі, протокол пошуку зламаних функцій, адаптований план розвантаження poolAI. **Функційний реєстр усіх проєктів:** [`GSV_FUNCTION_MANAGEMENT.md`](./GSV_FUNCTION_MANAGEMENT.md) — звірка функцій по репо, дублі (D1–D15), шар симбіозу, скелет per-project FM і owner-рішення.

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
| **telenetis** | `S:/rust/telenetis` | `S:/rust/telenetis/docs/HANDOFF_NEW_SESSION.md` | `S:/rust/telenetis/docs/NEXT_SESSION_PROMPT.md` | `cargo fmt -- --check` → `cargo clippy --all-targets` → `cargo test` | Rust **95–100%** |
| **llama-rs** | `S:/rust/llama-rs` | `S:/rust/llama-rs/docs/HANDOFF_NEW_SESSION.md` (journal: `docs/HANDOFF.md`) | `S:/rust/llama-rs/docs/ROADMAP.md` | `cargo fmt -- --check` → `cargo clippy --all-targets` → `cargo test` | Rust **95–100%** |
| **rebook** | `S:/rust/rebook` | `S:/rust/rebook/docs/HANDOFF_NEW_SESSION.md` | `S:/rust/rebook/docs/NEXT_SESSION_PROMPT.md` | `cargo fmt -- --check` → `cargo clippy --all-targets` → `cargo test` (EPUB path: `cargo run -- build-epub`) | Rust **95–100%** |
| **coreboard_labs_shop** | `S:/rust/CoreBoard_Labs_shop` | `S:/rust/CoreBoard_Labs_shop/docs/HANDOFF_NEW_SESSION.md` | `S:/rust/CoreBoard_Labs_shop/docs/NEXT_SESSION_PROMPT.md` | `cargo fmt --all` → `cargo clippy --all-targets` → `cargo test` (proprietary software, **private repo**) | Rust **95–100%** |
| **rusty-engine** | `S:/rust/rusty-engine` | `S:/rust/rusty-engine/docs/HANDOFF_NEW_SESSION.md` | `S:/rust/rusty-engine/docs/NEXT_SESSION_PROMPT.md` | `cargo fmt --all` → `cargo clippy --all-targets -- -D warnings` → `cargo test` (MIT, portable plugin; Rust-MoE core; **target model: MiniCPM-V 4.7 — official GGUF, no Python (B12)**; Docker only Win/Linux, Android via cargo-ndk) | Rust **95–100%** |
| **sport-forcast** | `S:/rust/sport-forcast` | `S:/rust/sport-forcast/docs/HANDOFF_NEW_SESSION.md` | `S:/rust/sport-forcast/docs/NEXT_SESSION_PROMPT.md` | `cargo fmt --all` → `cargo clippy --all-targets -- -D warnings` → `cargo test` (proprietary, **private repo**, binary releases only; Poland football first; `:9741`) | Rust **95–100%** |
| **gsv-wgpu** | `S:/rust/GSV-WGPU` | `S:/rust/GSV-WGPU/docs/HANDOFF_NEW_SESSION.md` | `S:/rust/GSV-WGPU/docs/NEXT_SESSION_PROMPT.md` | `cargo fmt --all` → `cargo clippy --all-targets -- -D warnings` → `cargo test` (MIT, WebGPU plugin core; band G0 scaffold: `GpuBroker`/scopes/tasks/grants) | Rust **95–100%** |
| **mesh-node** | `S:/rust/mesh-node` | `S:/rust/mesh-node/AGENTS.md` (немає HANDOFF/NEXT; Pi-only агент) | — (немає drain; зареєстровано 2026-11-17 як Pi mesh-agent) | `cargo fmt -- --check` → `cargo clippy --all-targets` → `cargo test` (на ПК; на Pi 4 — лише `mesh-node selftest`) | n/a (agent) |
| **adoreble_wifi** | `S:/rust/adoreble_wifi` | [`docs/HANDOFF_NEW_SESSION.md`](../../adoreble_wifi/docs/HANDOFF_NEW_SESSION.md) | [`docs/NEXT_SESSION_PROMPT.md`](../../adoreble_wifi/docs/NEXT_SESSION_PROMPT.md) | `cargo fmt -- --check` → `cargo clippy --all-targets` → `cargo test` (MIT, loopback Wi-Fi control plane, `:9770`, 2026-11-18 ADO-0 scaffold) | Rust **90–100%** |

**gsv-wgpu mesh plan (2026-11-12, owner):** bands A/B/C closed — browser mesh worker loop
(join/poll/result), 2+ peers fan-out, `matvec` LLM shard as a mesh op; live mesh = 3 browser
peers (phone ×2, PC Firefox) on hub `/api/wgpu/*` (`src/boxes/wgpu.rs`). Далі — bands D…J
(sustained bench · auto-rejoin · weight sharding · transformer block · norm/softmax ·
mini-LM loop (stretch) · capacity planner): canon у продукті
[`S:/rust/GSV-WGPU/docs/GSV_WGPU_MESH_TEST_PLAN.md`](../../GSV-WGPU/docs/GSV_WGPU_MESH_TEST_PLAN.md).

**Roles (owner 2026-10-16):** `telenetis` = **commutation (switching) plugin** (no APK / tensor / KVM inside it); the worker + APK role lives in the **engines** — `llama-rs` (MTP models, local provider `bunke-rock`) + `rusty-engine` (MoE models, local provider `rusty-serve`), adapted per each engine's own HANDOFF/NEXT. Canon: [`GSV_AGI_PATH.md`](./GSV_AGI_PATH.md).

Keep-live (band 225): llama-rs writes `target/live/llama_heartbeat.json` when run with `GSV_LIVE=1` or `LLAMA_RS_HEARTBEAT=1` (PID, model, epoch, bin_version, 15s tick); GSV reads it via `keep_live.llama_rs` (fresh = age ≤ 60s). `gsv_products_scan` / `cargo xtask products` enrich this heartbeat path + freshness for llama-rs.

Discovered but **not** in this table → S0 + git in that tree; no PH-S* drain until a row is added. OmniRoute is registered (band 149, owner-opt-in).

### Plugin automation / toolchain matrix (2026-11-23, read-only facts)

Джерело: `cargo xtask products --state` + `--wiring` + `rust-toolchain.toml` по репо. Політика SLI — [`GSV_SLI_ADOPTION_POLICY_2026-11-23.md`](./research/GSV_SLI_ADOPTION_POLICY_2026-11-23.md).

| id | Toolchain pin class | Wiring | Git remote |
|---|---|---|---|
| gsv (host) | GNU stable `stable-x86_64-pc-windows-gnu` | na (host) | origin |
| poolai | GNU stable | ok | origin (CI-authority for tests) |
| linfs, telenetis, llama-rs, rebook, orr_desktop, omniroute, rusty-engine, gsv-wgpu, sport-forcast, coreboard_labs_shop | GNU stable | ok | origin |
| adoreble_wifi, mesh-node | host-agnostic `stable` (cross-platform CI / Pi) | ok | origin — **крім mesh-node: no remote, owner gate** |
| gsv-wgpu | GNU stable | ok | **no remote, owner gate** |
| freebeat.ai, platinoff-profile | n/a (git-only) | na | unregistered |

**Звірено 2026-11-23:** 14 registered (gsv host + 13 plugins), усі `wiring=ok`; 2 unregistered `na`. Старий P6.5 «mesh-node missing» знято —_remote_ досі owner-gated, wiring справний. `omniroute` = policy-down, не піднімати без owner.

**Безпечний read-only drain-гейт:** `cargo xtask eco-check` → `rules-check` → `fm-xlink` → `mirrors --check` → `products --state/--wiring` → `gsv-loc-audit --stretch-96`. Mutating (`bump`, `fingerprint`, `push`, `live`, `tunnel`, `watchdog-install`) — окремі owner/drain кроки.

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
