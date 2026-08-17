# gsv_mcp_openbot — GSV as an MCP server

**Status:** Implemented (band **135**, `PH-S1989…S1998` ✅) · **Date:** 2026-08-17
**Deciders:** owner

GSV exposes one MCP server named **`gsv_mcp_openbot`**. OpenCode, Cursor, Grok CLI, and Grok Bot consume the **same** tools. Those products stay **clients** — they are not embedded inside `gsv-server`.

## Landed (band 135)

| Piece | Where |
|-------|--------|
| Stdio JSON-RPC (NDJSON) | `src/bin/gsv_mcp.rs` + `src/mcp.rs` · `cargo run --quiet --bin gsv-mcp` |
| HTTP | `GET /mcp` (discovery) · `POST /mcp` (JSON-RPC); loopback unless `--allow-lan` |
| Auto-register | `.mcp.json` · `.cursor/mcp.json` · `opencode.json` `mcp.gsv_mcp_openbot` |
| Tools | `gsv_health` / `gsv_tracker` / `gsv_ratio` / `gsv_sli` / `gsv_toolchain` / `gsv_vision_{manifest,feed,queue}` / `gsv_omni_chat` (dry-run default) / `gsv_ide_sessions` / `gsv_terminal` (HTTP allowlist) |
| Faster cold start | `target/debug/gsv-mcp.exe` after `cargo build --bin gsv-mcp` |

Grok Bot tunnel of `/mcp` to the public internet remains an **owner opt-in**. Do not port-forward in v1.

## Research (what actually exists)

| Client | What it is | How it talks MCP | Fit for GSV |
|--------|------------|------------------|-------------|
| **OpenCode** | Local coding agent already in this repo (`opencode.json`, `.opencode/`) | `opencode.json` → `mcp.<name>` `type: local` (stdio command) or `type: remote` (URL) | **Client.** Already used for VDT. Wire `gsv_mcp_openbot` as a local stdio server. |
| **Cursor** | IDE agent (this window) | `.cursor/mcp.json` + Settings → MCP | **Client.** Same stdio or loopback HTTP. |
| **Grok CLI** | xAI Grok in the terminal | `grok mcp add`; `~/.grok/config.toml`; project `.grok/config.toml`; also **loads** `.cursor/mcp.json` and project `.mcp.json` | **Client.** Ship `.mcp.json` in this repo so `grok inspect` sees `gsv_mcp_openbot`. |
| **Grok Bot** (SpaceXAI, 2026-08-11, beta) | Cloud teammate with **its own computer**; Cursor Ultra / SuperGrok Heavy | Follows **Cursor MCP policy**. Plugins + MCP. Cloud bot cannot reach `127.0.0.1` unless you tunnel. Can also run commands on the **member’s local computer** after consent. | **Client, not a library.** Do not try to “install Grok Bot into GSV”. Give it the same MCP server. Local stdio via the member’s machine is the safe default; public HTTP is a later opt-in. |

Sources: [OpenCode MCP](https://dev.opencode.ai/docs/mcp-servers/), [Grok MCP servers](https://docs.x.ai/build/features/mcp-servers), [Grok Bot intro](https://x.ai/news/introducing-grok-bot), [Grok Bot teams MCP policy](https://docs.x.ai/grok-bot/teams-and-enterprises).

## Decision — best fit

**A. GSV owns the MCP server (accepted).**

New Rust bin `gsv-mcp` (stdio) + optional Streamable HTTP route on `gsv-server` (`/mcp`), both wrapping **existing** boxes. Server id: `gsv_mcp_openbot`.

Why this wins:

1. GSV is already the local ops/vision process (`127.0.0.1:9999`, OmniRouter, Tracker, SLI, terminal allowlists).
2. One tool surface → four clients. No second daemon.
3. Matches localhost security (band 133–134): default **stdio on loopback**, not a public bot runtime.
4. OpenCode stays a client (IDE box already reads its sessions). Grok Bot stays a cloud client.

**B. Embed Grok Bot inside GSV (rejected).** Cloud product, own computer, not an SDK we vendor.

**C. Embed OpenCode as a child of `gsv-server` (rejected).** Duplicates the IDE box; OpenCode is the agent host, GSV is the tool host.

**D. Only HTTP `/mcp` on port 9999 (rejected as the only transport).** Fine as an extra for tunneled Grok Bot; stdio is the automatic local path.

## Automatic wiring (what “openbot” means)

When band 135 lands, `gsv_mcp_openbot` should appear **without** the owner pasting JSON by hand:

| File | Client |
|------|--------|
| `.mcp.json` (repo root) | Grok CLI + anything that reads project MCP |
| `.cursor/mcp.json` | Cursor + Grok Bot (Cursor MCP policy) |
| `opencode.json` → `mcp.gsv_mcp_openbot` | OpenCode local stdio |
| `.grok/config.toml` (optional `--scope project`) | Grok CLI project overlay |

Local command (sketch):

```json
{
  "mcpServers": {
    "gsv_mcp_openbot": {
      "command": "cargo",
      "args": ["run", "--quiet", "--bin", "gsv-mcp"],
      "cwd": "S:/rust/GSV"
    }
  }
}
```

OpenCode equivalent: `"type": "local", "command": ["cargo", "run", "--quiet", "--bin", "gsv-mcp"]`.

Prefer a built `target/debug/gsv-mcp.exe` in docs once the bin exists (faster cold start than `cargo run`).

## Tools (wrap boxes — do not invent a second product)

| Tool | Existing GSV surface |
|------|----------------------|
| `gsv_health` | `GET /api/health` |
| `gsv_tracker` | Tracker box |
| `gsv_sli` | SLI catalog |
| `gsv_toolchain` | Toolchain inventory |
| `gsv_ratio` | Ratio / `gsv-loc-audit` |
| `gsv_vision_*` | manifest / feed / sprint-queue |
| `gsv_omni_chat` | OmniRouter `POST /api/omni/v1/chat/completions` |
| `gsv_ide_sessions` | IDE box (OpenCode + Cursor sessions, read) |
| `gsv_terminal` | SLI terminal **same allowlist** as HTTP (no extra shell) |

No secrets in tool output (`omni.toml` keys stay redacted). POST body cap and CSRF do not apply to stdio; HTTP `/mcp` stays loopback unless `--allow-lan`.

## Security

- Default bind remains `127.0.0.1`. `/mcp` must not widen LAN without `--allow-lan`.
- Terminal tool = existing cargo/git allowlists (band 133).
- Grok Bot cloud: do **not** port-forward `/mcp` to the public internet in v1. Document a tunnel only as an explicit owner step.
- MCP auth tokens never land in `data/` git.

## Non-goals (band 135)

- Running Grok Bot’s cloud computer inside this repo.
- Auto-generating a second Galaxy UI for OpenCode.
- Python MCP adapters.

## See also

- Roadmap sprints: [`GSV_TECH_ROADMAP.md`](./GSV_TECH_ROADMAP.md) band 135
- Server: [`GSV_SERVER.md`](./GSV_SERVER.md)
- Boxes: [`GSV_BOXES.md`](./GSV_BOXES.md)
