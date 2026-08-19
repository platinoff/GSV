# gsv_mcp_openbot — GSV as an MCP server

**Status:** Implemented (band **162**, live crate/version lockstep · band **161**, vision lockstep + disk MiB / `--clean` keep-live · band **160**, GSV sandbox MCP · no User leak · band **159**, Cursor HTTP MCP + session SSE hold · band **158**, live stdio + sync check · band **157**, OmniRouter catalog + quota timers · band **156**, streaming usage · band **155**, session token usage · band **154**, watchdog ops card · band **153**, rust-first xtask · band **152**, `PH-S2159…S2168` ✅ · band **151**, `PH-S2149…S2158` ✅ · band 142 `PH-S2059…S2068` ✅ · band 141 `PH-S2049…S2058` ✅ · band 140 `PH-S2039…S2048` ✅ · band 139 `PH-S2029…S2038` ✅ · band 138 `PH-S2019…S2028` ✅ · band 137 `PH-S2009…S2018` ✅ · band 136 `PH-S1999…S2008` ✅ · band 135 `PH-S1989…S1998` ✅) · **Date:** 2026-08-18
**Deciders:** owner

GSV exposes one MCP server named **`gsv_mcp_openbot`**. OpenCode, Cursor, Grok CLI, and Grok Bot consume the **same** tools. Those products stay **clients** — they are not embedded inside `gsv-server`.

## Landed (band 135–141)

| Piece | Where |
|-------|--------|
| Stdio JSON-RPC (NDJSON) | `src/bin/gsv_mcp.rs` + `src/mcp.rs` · **`target/live/gsv-mcp.exe`** (`cargo xtask live` copies it; do **not** `cargo run --bin gsv-mcp`) |
| Discovery JSON | `GET /mcp` includes `version` + `crate_version` + `version_lag` + `http_url`. Sessionless `Accept: text/event-stream` still finite-flush. **GET with `Mcp-Session-Id` + SSE holds** the Streamable HTTP stream (Cursor). `POST /mcp` JSON-RPC; **skips browser CSRF**; `initialize` issues `Mcp-Session-Id`; `DELETE /mcp` ends the session; loopback unless `--allow-lan` |
| Auto-register | `.mcp.json` · `opencode.json` `mcp.gsv_mcp_openbot` · `.grok/config.toml` spawn **stdio** `target/live/gsv-mcp.exe`. **Cursor** `.cursor/mcp.json` uses **HTTP** `url: http://127.0.0.1:9999/mcp` (same live `gsv-server`, no second AppState) |
| Galaxy card | `GET /api/ui/card/mcp` (`render_mcp`, ops group, `CARD_NAMES` 32) |
| Tools (36) | health / tracker / ratio / sli / toolchain / vision (summary) / vision_{manifest,feed,queue,map,board,progress,speeds,rust,sprint_map,doc_preview,node_search,sync,extensions} / omni_chat (dry-run default) / **omni_route** / ide_sessions / terminal (HTTP allowlist) / hooks_{tests,bench} / update / preview (repo-relative, same confine as HTTP) / products / products_scan / products_select / watchdog / sw / fingerprints / xtask / disk / usage |
| Resources (10) | `gsv://vision/{manifest,feed,extensions}` · `gsv://docs/{mcp-openbot,handoff,next,fingerprints,post-always-on,rust-dev,omni-catalog}` — allowlist + `preview::resolve`; unknown / `file://` / `..` → JSON-RPC `-32602` |
| Subscribe | `resources/subscribe` + `resources/unsubscribe` (same allowlist); `initialize` `resources.subscribe: true` |
| Prompts (3) | `gsv_status` · `gsv_vision_brief` · `gsv_drain` |
| Logging | `logging/setLevel` (RFC 5424 levels; process-local on `AppState`; invalid → `-32602`) + `notifications/message` (stdio NDJSON + HTTP SSE, filtered by level) |
| Completions | `completion/complete` for `ref/resource` (`gsv://` URIs) and `ref/prompt` (prompt names); `..` / `file:` rejected |
| Resource updated | `gsv_vision_sync` → `notifications/resources/updated` for **every subscribed** `gsv://` URI (vision + docs) |
| Streamable HTTP | Sessionless GET/POST `Accept: text/event-stream` → finite SSE (`event: message`). **GET with session** holds SSE + keepalive. Discovery JSON adds `sse` / `streamable` / `stdio_live` / `http_url` / `version` / `http_csrf: false` |
| Live stdio | `cargo xtask live` copies `gsv-mcp` next to `gsv-server`; client JSON points at `target/live/gsv-mcp.exe` |
| Sync check | MCP `gsv_xtask` `{task:sync}` is `--check` only (drift); remirror stays `gsv_vision_sync` |
| HTTP sessions | `initialize` on POST `/mcp` issues `Mcp-Session-Id` (process-local, cap 32); unknown id → 404 `{ok:false}`; `DELETE /mcp` ends it; GET JSON discovery stays sessionless |
| Faster cold start | `target/debug/gsv-mcp.exe` after `cargo build --bin gsv-mcp` |

Grok Bot tunnel of `/mcp` to the public internet remains an **owner opt-in**. Do not port-forward in v1.

## Research (what actually exists)

| Client | What it is | How it talks MCP | Fit for GSV |
|--------|------------|------------------|-------------|
| **OpenCode** | Local coding agent already in this repo (`opencode.json`, `.opencode/`) | `opencode.json` → `mcp.<name>` `type: local` (stdio command) or `type: remote` (URL) | **Client.** Already used for VDT. Wire `gsv_mcp_openbot` as a local stdio server. |
| **Cursor** | IDE agent (this window) | `.cursor/mcp.json` `url` → live `http://127.0.0.1:9999/mcp` | **Client.** HTTP to always-on Galaxy. Stdio is the OpenCode/Grok path. |
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
| `.mcp.json` (repo root) | Grok CLI + anything that reads project MCP (stdio live copy) |
| `.cursor/mcp.json` | Cursor folder **GSV** — HTTP `http://127.0.0.1:9999/mcp`. **Do not** copy into `%USERPROFILE%/.cursor/mcp.json` (User scope leaks into PoolAI windows). |
| `opencode.json` → `mcp.gsv_mcp_openbot` | OpenCode local stdio |
| `.grok/config.toml` | Grok CLI project overlay (`[mcp_servers.gsv_mcp_openbot]`) stdio |

Cursor (band **160**: folder GSV only, never User MCP):

```json
{
  "mcpServers": {
    "gsv_mcp_openbot": {
      "type": "http",
      "url": "http://127.0.0.1:9999/mcp"
    }
  }
}
```

Stdio clients (OpenCode / Grok — live copy, `cargo xtask live` must have run once):

```json
{
  "mcpServers": {
    "gsv_mcp_openbot": {
      "command": "S:/rust/GSV/target/live/gsv-mcp.exe",
      "args": ["--repo-root", "S:/rust/GSV"],
      "cwd": "S:/rust/GSV"
    }
  }
}
```

OpenCode equivalent: `"type": "local", "command": ["S:/rust/GSV/target/live/gsv-mcp.exe", "--repo-root", "S:/rust/GSV"]`.

Do **not** `cargo run --bin gsv-mcp` from the client: it is slow, takes the cargo file lock, and is a second `AppState` that dies on every drain `cargo test`.

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
| `gsv_omni_chat` | OmniRouter `POST /api/omni/v1/chat/completions` (empty model auto-picks; live completions increment `/api/usage`) |
| `gsv_omni_route` | OmniRouter `GET /api/omni/route` — skip cooling free hosts until `reset_secs` |
| `gsv_usage` | Session token totals (`GET /api/usage`) — OmniRouter + MCP session + OmniRoute pull |
| `gsv_xtask` | Read-only `catalog` / `products` / `disk` / `sync` (`--check` drift). Remirror is `gsv_vision_sync`. |
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
| `gsv://docs/fingerprints` | `docs/gsv/fingerprints.jsonl` |
| `gsv://docs/post-always-on` | `docs/gsv/GSV_POST_ALWAYS_ON.md` |
| `gsv://docs/rust-dev` | `docs/gsv/GSV_RUST_DEV.md` |
| `gsv://docs/omni-catalog` | `docs/gsv/GSV_OMNI_CATALOG.md` |

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
subscribed `gsv://` URI (vision snapshots **and** docs such as handoff/next). Subscribe/unsubscribe also queue
`notifications/message` at `info` (skipped when `logging/setLevel` is above that).

Stdio (`gsv-mcp`) writes pending notifications as extra NDJSON lines **before** the
JSON-RPC response. HTTP `POST /mcp` with `Accept: text/event-stream` returns the
same queue as SSE `event: message` frames (notifications, then the RPC result).
`GET /mcp` with that Accept flushes pending notifications as a finite SSE body.
Without `text/event-stream`, POST stays JSON-RPC (queue drained) and GET stays
discovery JSON (`sse` / `streamable` / `sessions` / `session_count` true) for the Galaxy
card and stand-smoke. HTTP `initialize` sets `Mcp-Session-Id`; later POST/GET/DELETE
with an unknown id return 404 `{ok:false,error}`. Missing header stays allowed so
Galaxy discovery and stand-smoke keep working. Stdio does not issue HTTP sessions.

No secrets in tool output (`omni.toml` keys stay redacted). POST body cap and CSRF do not apply to stdio; HTTP `/mcp` stays loopback unless `--allow-lan`.

## Security

- Default bind remains `127.0.0.1`. `/mcp` must not widen LAN without `--allow-lan`.
- Sandbox is the GSV crate (`S:/rust/GSV`): preview + `gsv://` confine; terminal = HTTP SLI allowlist (no `git push` / extra shell).
- VDT products (`poolai`, `omniroute`, …) only via `gsv_products_*` discovered ids. No MCP `products/open`, `update/apply`, or tunnel.
- Cursor MCP is **folder GSV** (`.cursor/mcp.json`). User-scope `%USERPROFILE%/.cursor/mcp.json` leaks the bot into PoolAI windows — do not install it there.
- Terminal tool = existing cargo/git allowlists (band 133).
- POST `/mcp` skips the browser Origin / `Sec-Fetch-Site` CSRF gate (bots are not the Galaxy UI). Body cap still applies. Other POSTs stay gated.
- Grok Bot cloud: `cargo xtask tunnel` is the **owner-opt-in** public hop (`cloudflared tunnel --url http://127.0.0.1:9999`). Not on by default. `/mcp` on that URL is world-reachable until you Ctrl+C. Do not add an MCP tool that starts the tunnel.
- MCP auth tokens never land in `data/` git.

## Non-goals (band 135)

- Running Grok Bot’s cloud computer inside this repo.
- Auto-generating a second Galaxy UI for OpenCode.
- Python MCP adapters.

## Horizon (band 160+)

Band **160** scoped Cursor MCP to folder **GSV** (`S:/rust/GSV` sandbox; VDT products via allowlist). Do not install User MCP. Still **not** on MCP: `products/open`, `update/apply`, starting the tunnel.

Band **159** attached Cursor to the live Galaxy HTTP MCP (`url` in `.cursor/mcp.json`), advertised `version`/`http_url` on `GET /mcp`, and held session GET SSE so Streamable HTTP stays up. Recopy `target/live/gsv-server.exe` after a drain or HTTP tools lag the crate. Still **not** on MCP: `products/open`, `update/apply`, starting the tunnel.

Spec: [`GSV_POST_ALWAYS_ON.md`](./GSV_POST_ALWAYS_ON.md). Plan: [`docs/superpowers/plans/2026-08-18-mcp-always-on-catchup.md`](../superpowers/plans/2026-08-18-mcp-always-on-catchup.md).

## See also

- Roadmap sprints: [`GSV_TECH_ROADMAP.md`](./GSV_TECH_ROADMAP.md) band 135–142 ✅ · **151 ✅** · **152 ✅** · **153 ✅** · **154 ✅** · **155 ✅** · **156 ✅** · **157 ✅** · **158 ✅** · **159 ✅** · **160 ✅** · **161 ✅** · **162 ✅**
- Server: [`GSV_SERVER.md`](./GSV_SERVER.md)
- Boxes: [`GSV_BOXES.md`](./GSV_BOXES.md)
