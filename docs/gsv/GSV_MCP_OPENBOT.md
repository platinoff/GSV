# gsv_mcp_openbot — GSV as an MCP server

**Status:** Implemented (band **141**, `PH-S2049…S2058` ✅ · band 140 `PH-S2039…S2048` ✅ · band 139 `PH-S2029…S2038` ✅ · band 138 `PH-S2019…S2028` ✅ · band 137 `PH-S2009…S2018` ✅ · band 136 `PH-S1999…S2008` ✅ · band 135 `PH-S1989…S1998` ✅) · **Date:** 2026-08-17
**Deciders:** owner

GSV exposes one MCP server named **`gsv_mcp_openbot`**. OpenCode, Cursor, Grok CLI, and Grok Bot consume the **same** tools. Those products stay **clients** — they are not embedded inside `gsv-server`.

## Landed (band 135–141)

| Piece | Where |
|-------|--------|
| Stdio JSON-RPC (NDJSON) | `src/bin/gsv_mcp.rs` + `src/mcp.rs` · `cargo run --quiet --bin gsv-mcp` |
| HTTP | `GET /mcp` (discovery JSON unless `Accept: text/event-stream`) · `POST /mcp` JSON-RPC; SSE flushes notifications when Accept lists `text/event-stream`; loopback unless `--allow-lan` |
| Auto-register | `.mcp.json` · `.cursor/mcp.json` · `opencode.json` `mcp.gsv_mcp_openbot` · `.grok/config.toml` |
| Galaxy card | `GET /api/ui/card/mcp` (`render_mcp`, ops group, `CARD_NAMES` 32) |
| Tools (26) | health / tracker / ratio / sli / toolchain / vision (summary) / vision_{manifest,feed,queue,map,board,progress,speeds,rust,sprint_map,doc_preview,node_search,sync,extensions} / omni_chat (dry-run default) / ide_sessions / terminal (HTTP allowlist) / hooks_{tests,bench} / update / preview (repo-relative, same confine as HTTP) |
| Resources (6) | `gsv://vision/{manifest,feed,extensions}` · `gsv://docs/{mcp-openbot,handoff,next}` — allowlist + `preview::resolve`; unknown / `file://` / `..` → JSON-RPC `-32602` |
| Subscribe | `resources/subscribe` + `resources/unsubscribe` (same allowlist); `initialize` `resources.subscribe: true` |
| Prompts (3) | `gsv_status` · `gsv_vision_brief` · `gsv_drain` |
| Logging | `logging/setLevel` (RFC 5424 levels; process-local on `AppState`; invalid → `-32602`) + `notifications/message` (stdio NDJSON + HTTP SSE, filtered by level) |
| Completions | `completion/complete` for `ref/resource` (`gsv://` URIs) and `ref/prompt` (prompt names); `..` / `file:` rejected |
| Resource updated | `gsv_vision_sync` → `notifications/resources/updated` for subscribed `gsv://vision/*` URIs |
| Streamable HTTP | `Accept: text/event-stream` on GET/POST `/mcp` → finite SSE (`event: message`); discovery JSON adds `sse` / `streamable` |
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
| `.grok/config.toml` | Grok CLI project overlay (`[mcp_servers.gsv_mcp_openbot]`) |

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
| `gsv_vision_*` | summary / manifest / feed / sprint-queue / map / board / progress / speeds / rust-diagnostics / sprint-map / doc-preview / node-search / sync / extensions |
| `gsv_preview` | Box preview (`file` repo-relative; same confine as `GET /api/preview`) |
| `gsv_hooks_*` | tests + bench hooks (read `target/`, no rebuild) |
| `gsv_update` | Update box (binary vs source mtime) |
| `gsv_omni_chat` | OmniRouter `POST /api/omni/v1/chat/completions` |
| `gsv_ide_sessions` | IDE box (OpenCode + Cursor sessions, read) |
| `gsv_terminal` | SLI terminal **same allowlist** as HTTP (no extra shell) |

### Resources (`resources/list` · `resources/read`)

Allowlisted `gsv://` URIs only. Read uses `preview::resolve` (no traversal, no absolute, no `file://`).

| URI | File |
|-----|------|
| `gsv://vision/manifest` | `docs/vision/manifest.json` |
| `gsv://vision/feed` | `docs/vision/feed.json` |
| `gsv://vision/extensions` | `docs/vision/extensions.json` |
| `gsv://docs/mcp-openbot` | `docs/gsv/GSV_MCP_OPENBOT.md` |
| `gsv://docs/handoff` | `docs/HANDOFF_NEW_SESSION.md` |
| `gsv://docs/next` | `docs/NEXT_SESSION_PROMPT.md` |

### Prompts (`prompts/list` · `prompts/get`)

| Name | Use |
|------|-----|
| `gsv_status` | Health + ratio + vision revision |
| `gsv_vision_brief` | Sprint map + extensions + drift |
| `gsv_drain` | Next PH-S* band (no mid-drain push) |

### Logging (`logging/setLevel`)

RFC 5424 levels: `debug` · `info` · `notice` · `warning` · `error` · `critical` · `alert` · `emergency`.
Default is `info` (process-local on `AppState`). Unknown level → JSON-RPC `-32602`.
`GET /mcp` reports the current `log_level`.

### Completions (`completion/complete`)

| `ref.type` | Completes |
|------------|-----------|
| `ref/resource` | Allowlisted `gsv://` URIs whose prefix matches `argument.value` |
| `ref/prompt` | Prompt names whose prefix matches `argument.value` |

Traversal (`..`), `file:`, and `\\` prefixes → `-32602` (same confine as `resources/read`). Unknown `ref.type` → `-32602`. At most 100 values; `hasMore` is always false for this allowlist.

### Subscribe (`resources/subscribe` · `resources/unsubscribe`)

Same allowlist as `resources/read`. `initialize` advertises `resources.subscribe: true`.
Unknown / `file://` / `..` → `-32602`. Subscriptions are process-local on `AppState`.
`GET /mcp` reports `subscribe`, `subscription_count`, and `subscriptions`.

After `gsv_vision_sync`, the server queues `notifications/resources/updated` for each
subscribed `gsv://vision/*` URI. Subscribe/unsubscribe also queue
`notifications/message` at `info` (skipped when `logging/setLevel` is above that).

Stdio (`gsv-mcp`) writes pending notifications as extra NDJSON lines **before** the
JSON-RPC response. HTTP `POST /mcp` with `Accept: text/event-stream` returns the
same queue as SSE `event: message` frames (notifications, then the RPC result).
`GET /mcp` with that Accept flushes pending notifications as a finite SSE body.
Without `text/event-stream`, POST stays JSON-RPC (queue drained) and GET stays
discovery JSON (`sse` / `streamable` true) for the Galaxy card and stand-smoke.

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

- Roadmap sprints: [`GSV_TECH_ROADMAP.md`](./GSV_TECH_ROADMAP.md) band 135–141
- Server: [`GSV_SERVER.md`](./GSV_SERVER.md)
- Boxes: [`GSV_BOXES.md`](./GSV_BOXES.md)
