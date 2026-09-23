# AGENTS.md — Telenetis (plugin, nested under GSV)

This tree is a **portable plugin** of the rust-folder environment (nested exception, not the pattern to copy).
**Global rules first:** `S:/rust/GSV/AGENTS.md` · `S:/rust/GSV/docs/gsv/GSV_AGI_PATH.md`.
Open `S:/rust/GSV/gsv.code-workspace` (GSV first) so hub MCP, tickets, bands, and sprints stay wired. Do **not** copy the VDT kit here. Do **not** install User-scope Cursor MCP. Do **not** grow Telenetis into the orchestrator.

When a multi-agent workflow pattern is more relevant in PoolAI than here, copy the **idea** from `S:/rust/poolAI` — not PoolAI product files.

Local canon: [`docs/HANDOFF_NEW_SESSION.md`](docs/HANDOFF_NEW_SESSION.md). Tests: `cargo fmt -- --check` → `cargo clippy --all-targets` → `cargo test`. Let `rust-toolchain.toml` win (`unset RUSTUP_TOOLCHAIN`). Grid calls go through the GSV hub (`/api/edge`), never `:8091` off-box.
