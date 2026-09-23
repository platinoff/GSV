# PLUGIN_AGENTS.md — local rules template (portable plugin)

Copy this into the plugin’s own `AGENTS.md` (or prepend it). **Do not** copy the
VDT kit (`.agents/skills/`, generic `.cursor/rules/`) into the plugin.

This tree is a **portable plugin** of the rust-folder environment (`S:/rust`).
GSV is the **mother hub**. PoolAI is the **genetic parent** for multi-agent
workflow DNA when that pattern dominates relevance.

## Global first

1. Read **`S:/rust/GSV/AGENTS.md`** and [`GSV_AGI_PATH.md`](./GSV_AGI_PATH.md)
   (`gsv://docs/agi-path`).
2. Then read **this product’s** HANDOFF / NEXT / test command
   ([`PRODUCTS.md`](./PRODUCTS.md) row).
3. Local rules below cover **this product only** (FM, ratio, bins, concept).
   Do not restate S0 / MSYS2 / `абракадабра` / MCP sandbox here.

## Auto-wire (any IDE / model / folder)

Open `S:/rust/GSV/gsv.code-workspace` (**GSV first**) so hub MCP, tickets,
bands, sprints, and keep-live stay wired. Do **not** install User-scope Cursor
MCP. OpenCode / Grok: stdio `S:/rust/GSV/target/live/gsv-mcp.exe`.

When a workflow pattern (roles, PH-S* bands, session iteration, marshal/squad)
is more relevant in PoolAI than here, copy the **idea** from `S:/rust/poolAI` —
not PoolAI product files.

## Must not

- Copy `.agents/skills/` or GSV globals into this tree.
- Run a second `gsv-server` / User-scope MCP.
- Call poolAI `:8091` off-box (clients use hub `/api/edge`).
- Export `RUSTUP_TOOLCHAIN` inside this crate (let `rust-toolchain.toml` win).
