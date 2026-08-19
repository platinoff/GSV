//! `gsv_mcp_openbot` — MCP JSON-RPC surface over the existing boxes.
//!
//! Stdio (`gsv-mcp`) and optional HTTP `POST /mcp` share [`handle_value`].
//! Tools wrap Tracker / SLI / Toolchain / Ratio / Vision / Omni / IDE / terminal /
//! preview; they do not add a second shell. Secrets in tool output are redacted.
//! Band 138: allowlisted `gsv://` `resources/*` and named `prompts/*`.
//! Band 139: `logging/setLevel` + `completion/complete` (resource URIs + prompt names).
//! Band 140: `resources/subscribe`+`unsubscribe` + `notifications/message` (log
//! filter) + `notifications/resources/updated` after `gsv_vision_sync`.
//! Band 141: HTTP Streamable HTTP SSE — `POST`/`GET /mcp` with
//! `Accept: text/event-stream` flush the same notification queue as stdio.
//! Band 142: HTTP `Mcp-Session-Id` (process-local) + `DELETE /mcp`.

use std::sync::atomic::Ordering;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::body::to_bytes;
use axum::http::{HeaderMap, HeaderValue};
use serde_json::{json, Value};

use crate::boxes::omni::proxy::chat_completions;
use crate::boxes::terminal;
use crate::boxes::{hooks, ide, ratio, sli, toolchain, update};
use crate::state::AppState;
use crate::GSV_SERVER_NAME;

/// MCP server id (OpenCode / Cursor / Grok CLI / Grok Bot).
pub const SERVER_ID: &str = "gsv_mcp_openbot";

/// MCP protocol version this server speaks.
pub const PROTOCOL_VERSION: &str = "2025-03-26";

/// Streamable HTTP session header (case-insensitive on the wire).
pub const MCP_SESSION_HEADER: &str = "mcp-session-id";

/// Cap on process-local HTTP MCP sessions (oldest dropped).
pub const MCP_SESSION_CAP: usize = 32;

/// Visible ASCII session id: 8–128 alphanumeric or hyphen.
pub fn valid_mcp_session_id(id: &str) -> bool {
    let n = id.len();
    (8..=128).contains(&n) && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}

/// New HTTP session id from a monotonic sequence (loopback, process-local).
pub fn new_mcp_session_id(seq: u64) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(seq);
    format!("{nanos:016x}-{seq:08x}")
}

/// Read `Mcp-Session-Id` from request headers.
pub fn mcp_session_id_from_headers(headers: &HeaderMap) -> Option<String> {
    headers
        .get(MCP_SESSION_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// True when this JSON-RPC value (or batch) contains `initialize`.
pub fn jsonrpc_mentions_initialize(value: &Value) -> bool {
    match value {
        Value::Array(items) => items.iter().any(jsonrpc_mentions_initialize),
        Value::Object(o) => o.get("method").and_then(Value::as_str) == Some("initialize"),
        _ => false,
    }
}

/// RFC 5424 syslog levels advertised by `logging/setLevel`.
pub const LOG_LEVELS: &[&str] = &[
    "debug",
    "info",
    "notice",
    "warning",
    "error",
    "critical",
    "alert",
    "emergency",
];

/// Default `logging/setLevel` index (`info`).
pub const DEFAULT_LOG_LEVEL: u8 = 1;

const COMPLETION_MAX: usize = 100;

const SECRET_KEYS: &[&str] = &["api_key", "secret", "authorization", "password", "token"];

/// Tool descriptors for `tools/list`.
pub fn tools_list() -> Vec<Value> {
    vec![
        tool("gsv_health", "GSV process health (name, version, uptime).", object_schema()),
        tool("gsv_tracker", "Tracker box: sprint snapshot and recent records.", object_schema()),
        tool("gsv_ratio", "Rust LOC ratio report (`gsv-loc-audit` store).", object_schema()),
        tool("gsv_sli", "SLI command catalog from src/bin/ and cargo xtask.", object_schema()),
        tool(
            "gsv_toolchain",
            "Toolchain inventory (rustc, cargo, clippy, MSYS2).",
            object_schema(),
        ),
        tool(
            "gsv_vision_manifest",
            "Vision manifest (nodes, layers, edges).",
            object_schema(),
        ),
        tool("gsv_vision_feed", "Vision feed items (sprint ticker).", object_schema()),
        tool(
            "gsv_vision_queue",
            "Vision sprint queue (entries ∪ active plan).",
            object_schema(),
        ),
        tool(
            "gsv_omni_chat",
            "OmniRouter chat completions. Default is dry-run (no upstream). Set live=true to forward. Empty model auto-picks the next live host (skips cooldown timers).",
            json!({
                "type": "object",
                "properties": {
                    "model": { "type": "string" },
                    "messages": { "type": "array" },
                    "provider": { "type": "string" },
                    "task": {
                        "type": "string",
                        "description": "rust | web | any — used when model is empty"
                    },
                    "prefer_free": { "type": "boolean" },
                    "live": {
                        "type": "boolean",
                        "description": "If true, call the upstream provider. Default false (dry-run)."
                    }
                }
            }),
        ),
        tool(
            "gsv_omni_route",
            "Timer-aware OmniRouter pick: next Rust/web model that is not in a free-tier cooldown. Shared catalog for Cursor / OpenCode / Grok.",
            json!({
                "type": "object",
                "properties": {
                    "task": { "type": "string", "description": "rust | web | any (default rust)" },
                    "prefer_free": { "type": "boolean", "description": "Prefer free hosts (default true)" }
                }
            }),
        ),
        tool(
            "gsv_ide_sessions",
            "OpenCode + Cursor sessions (read-only).",
            object_schema(),
        ),
        tool(
            "gsv_terminal",
            "Run a command through the same SLI allowlist as POST /api/terminal.",
            json!({
                "type": "object",
                "properties": {
                    "command": { "type": "string" }
                },
                "required": ["command"]
            }),
        ),
        tool("gsv_vision_map", "Vision map (layers L0–L5 + edge kinds).", object_schema()),
        tool("gsv_vision_board", "Sprint board (open/closed/planned + progress).", object_schema()),
        tool(
            "gsv_vision_progress",
            "Sprint progress (status counts + per-layer nodes).",
            object_schema(),
        ),
        tool("gsv_vision_speeds", "Speed index (latest test-CI + bench history).", object_schema()),
        tool(
            "gsv_vision_rust",
            "Rust diagnostics (warnings/errors + top clippy codes).",
            object_schema(),
        ),
        tool(
            "gsv_hooks_tests",
            "Tests hook: read-only artifacts under target/ (no rebuild).",
            object_schema(),
        ),
        tool(
            "gsv_hooks_bench",
            "Bench hook: Criterion dirs + speed index (no rebuild).",
            object_schema(),
        ),
        tool("gsv_update", "Update box: binary vs source mtime, git HEAD.", object_schema()),
        tool(
            "gsv_vision",
            "Vision summary (revision, next sprint, git HEAD).",
            object_schema(),
        ),
        tool(
            "gsv_vision_sprint_map",
            "Sprint map (scope/queue/session-tracks links + modules).",
            object_schema(),
        ),
        tool(
            "gsv_vision_doc_preview",
            "Doc preview for a vision node plus 1-hop neighbors.",
            json!({
                "type": "object",
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "Manifest node id (default galaxy_grid)."
                    }
                }
            }),
        ),
        tool(
            "gsv_vision_node_search",
            "Search vision nodes by id/label/path/sections.",
            json!({
                "type": "object",
                "properties": {
                    "q": { "type": "string" },
                    "layer": {
                        "type": "string",
                        "description": "Optional layer filter L0–L5."
                    }
                }
            }),
        ),
        tool(
            "gsv_vision_sync",
            "Re-mirror vision snapshots and report drift.",
            object_schema(),
        ),
        tool(
            "gsv_vision_extensions",
            "Vision extensions (active sprint, scopes, panels).",
            object_schema(),
        ),
        tool(
            "gsv_preview",
            "Repo-relative file preview (same confine as GET /api/preview).",
            json!({
                "type": "object",
                "properties": {
                    "file": { "type": "string", "description": "Repo-relative path." }
                },
                "required": ["file"]
            }),
        ),
        tool("gsv_products", "VDT environment products (workspace ∪ sibling git ∪ kit).", object_schema()),
        tool(
            "gsv_products_scan",
            "Scan one discovered product (git HEAD, HANDOFF/NEXT, kind). id optional after gsv_products_select.",
            json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "Product id from gsv_products (e.g. gsv). Omit when a product is already selected." }
                }
            }),
        ),
        tool(
            "gsv_products_select",
            "Select a discovered VDT product (same allowlist as POST /api/products/select).",
            json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "Product id from gsv_products (e.g. gsv)." }
                },
                "required": ["id"]
            }),
        ),
        tool("gsv_watchdog", "Live watchdog heartbeat (target/live/watchdog.json).", object_schema()),
        tool("gsv_sw", "Service Worker shell cache discovery (cache name + precache urls).", object_schema()),
        tool(
            "gsv_fingerprints",
            "Drain fingerprints JSONL (actor / IDE / model / time).",
            json!({
                "type": "object",
                "properties": {
                    "limit": { "type": "integer", "description": "Latest N rows (default 20, cap 100)." }
                }
            }),
        ),
        tool(
            "gsv_xtask",
            "Read-only cargo xtask catalog (task=catalog|products|disk|sync). sync is --check drift only; remirror is gsv_vision_sync. Mutating tasks stay on cargo xtask.",
            json!({
                "type": "object",
                "properties": {
                    "task": {
                        "type": "string",
                        "description": "catalog (default), products, disk, or sync (`--check` drift only)."
                    }
                }
            }),
        ),
        tool("gsv_disk", "S0 disk guard (free GiB/MiB on the repo volume + target/ size; --clean is CLI only).", json!({
                "type": "object",
                "properties": {
                    "enforce": {
                        "type": "boolean",
                        "description": "If true, ok=false when limits trip (same as cargo xtask disk --enforce)."
                    }
                }
            })),
        tool(
            "gsv_usage",
            "Session token usage (OmniRouter + MCP bot + OmniRoute pull).",
            object_schema(),
        ),
        tool(
            "gsv_settings",
            "GSV settings / Godfather store (redacted: token_set, never bot_token). Read-only; owner writes via POST /api/settings.",
            object_schema(),
        ),
        tool(
            "gsv_telegram",
            "Godfather Telegram bind status (redacted; never bot_token). Read-only getMe+getChat; dry-run stub under cargo test / X-Telegram-Dry-Run.",
            object_schema(),
        ),
        tool(
            "gsv_telegram_bus_send",
            "Send a bus envelope {from,to?,ticket_id?,body} to the Godfather channel. Requires co-workflow telegram-relay. Caps body at 2 KiB. Never returns bot_token. Tests/dry-run use an in-memory queue (no sockets).",
            json!({
                "type": "object",
                "properties": {
                    "from": { "type": "string", "description": "Sender id (must match godfather.allowed_user_ids when that list is non-empty)." },
                    "to": { "type": "string", "description": "Optional recipient id." },
                    "ticket_id": { "type": "string", "description": "Optional ticket id this message is about." },
                    "body": { "type": "string", "description": "Message body (max 2048 bytes)." }
                },
                "required": ["from", "body"]
            }),
        ),
        tool(
            "gsv_telegram_bus_poll",
            "Poll bus envelopes from the Godfather channel (or the dry-run queue). Requires telegram-relay. Never returns bot_token.",
            json!({
                "type": "object",
                "properties": {
                    "limit": { "type": "integer", "description": "Max envelopes to return (1–32, default 8)." }
                }
            }),
        ),
        tool(
            "gsv_tickets",
            "GSV ticket board (docs/gsv/tickets.jsonl). List open/in_progress/done. No secrets.",
            object_schema(),
        ),
        tool(
            "gsv_tickets_claim",
            "Claim a ticket (open → in_progress) and append docs/gsv/ticket_claims.jsonl. Requires co-workflow ticket-claim. Unknown id is a tool error.",
            json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "Ticket id to claim." }
                },
                "required": ["id"]
            }),
        ),
    ]
}

const RESOURCE_URIS: &[&str] = &[
    "gsv://vision/manifest",
    "gsv://vision/feed",
    "gsv://vision/extensions",
    "gsv://docs/mcp-openbot",
    "gsv://docs/handoff",
    "gsv://docs/next",
    "gsv://docs/fingerprints",
    "gsv://docs/post-always-on",
    "gsv://docs/rust-dev",
    "gsv://docs/omni-catalog",
    "gsv://docs/settings-telegram",
];

const PROMPT_NAMES: &[&str] = &["gsv_status", "gsv_vision_brief", "gsv_drain"];

struct ResourceSpec {
    uri: &'static str,
    name: &'static str,
    description: &'static str,
    mime: &'static str,
    rel: &'static str,
}

const RESOURCES: &[ResourceSpec] = &[
    ResourceSpec {
        uri: "gsv://vision/manifest",
        name: "Vision manifest",
        description: "Galaxy graph (nodes, layers, edges).",
        mime: "application/json",
        rel: "docs/vision/manifest.json",
    },
    ResourceSpec {
        uri: "gsv://vision/feed",
        name: "Vision feed",
        description: "Sprint ticker items.",
        mime: "application/json",
        rel: "docs/vision/feed.json",
    },
    ResourceSpec {
        uri: "gsv://vision/extensions",
        name: "Vision extensions",
        description: "Active sprint, scopes, panels.",
        mime: "application/json",
        rel: "docs/vision/extensions.json",
    },
    ResourceSpec {
        uri: "gsv://docs/mcp-openbot",
        name: "MCP openbot canon",
        description: "gsv_mcp_openbot design and client wiring.",
        mime: "text/markdown",
        rel: "docs/gsv/GSV_MCP_OPENBOT.md",
    },
    ResourceSpec {
        uri: "gsv://docs/handoff",
        name: "Session handoff",
        description: "GSV HANDOFF for the next drain session.",
        mime: "text/markdown",
        rel: "docs/HANDOFF_NEW_SESSION.md",
    },
    ResourceSpec {
        uri: "gsv://docs/next",
        name: "Next session prompt",
        description: "Copy-paste prompt for the next GSV session.",
        mime: "text/markdown",
        rel: "docs/NEXT_SESSION_PROMPT.md",
    },
    ResourceSpec {
        uri: "gsv://docs/fingerprints",
        name: "Drain fingerprints",
        description: "Append-only drain fingerprint JSONL.",
        mime: "application/jsonl",
        rel: "docs/gsv/fingerprints.jsonl",
    },
    ResourceSpec {
        uri: "gsv://docs/post-always-on",
        name: "Post always-on spec",
        description: "MCP catch-up conception for band 151+.",
        mime: "text/markdown",
        rel: "docs/gsv/GSV_POST_ALWAYS_ON.md",
    },
    ResourceSpec {
        uri: "gsv://docs/rust-dev",
        name: "Rust-first tests/benches/scripts",
        description: "cargo xtask canon: product automation in .rs, not .sh/.ps1/JSON harnesses.",
        mime: "text/markdown",
        rel: "docs/gsv/GSV_RUST_DEV.md",
    },
    ResourceSpec {
        uri: "gsv://docs/omni-catalog",
        name: "OmniRouter shared catalog",
        description:
            "Rust+web models for OmniRouter, Cursor, OpenCode, Grok — free notes and quota timers.",
        mime: "text/markdown",
        rel: "docs/gsv/GSV_OMNI_CATALOG.md",
    },
    ResourceSpec {
        uri: "gsv://docs/settings-telegram",
        name: "Settings / Telegram spec",
        description: "Band 169 Telegram bus landed. Settings 166 · bind 167 · tickets 168.",
        mime: "text/markdown",
        rel: "docs/gsv/GSV_SETTINGS_TELEGRAM.md",
    },
];

struct PromptSpec {
    name: &'static str,
    description: &'static str,
    text: &'static str,
}

const PROMPTS: &[PromptSpec] = &[
    PromptSpec {
        name: "gsv_status",
        description: "Summarize GSV health, LOC ratio, and vision revision.",
        text: "Summarize GSV status. Call gsv_health, gsv_ratio, and gsv_vision. Report ok, rust ratio vs 96% stretch, vision revision, and next sprint. Keep secrets redacted.",
    },
    PromptSpec {
        name: "gsv_vision_brief",
        description: "Brief the current vision map and active sprint.",
        text: "Brief GSV vision. Call gsv_vision_sprint_map and gsv_vision_extensions. Name the active sprint, in-scope modules, and any drift from gsv_vision_sync.",
    },
    PromptSpec {
        name: "gsv_drain",
        description: "Start a VDT drain: next PH-S* band after the last closed sprint.",
        text: "Start a GSV VDT drain. Sandbox is this GSV repo (S:/rust/GSV): preview, terminal, vision, and xtask stay inside it. Registered VDT products (poolai, omniroute, …) are reached only via gsv_products / gsv_products_select / gsv_products_scan (unknown id is a tool error; no gsv_products_open). Do not install gsv_mcp_openbot as Cursor User MCP — that leaks into PoolAI windows. Keep it in GSV/.cursor/mcp.json (folder scope GSV). Cursor 3.16 still uses Streamable HTTP type=http on that folder file (never User; do not Origin-host this kit). Read gsv://docs/next, gsv://docs/rust-dev, gsv://docs/post-always-on, and gsv://docs/settings-telegram. Call gsv_xtask (task=products) or gsv_products, then gsv_products_select with the owner pick, then gsv_products_scan (id optional after select), gsv_disk, gsv_watchdog, gsv_usage, gsv_settings (redacted read; no MCP write of tokens — HTTP POST /api/settings is the owner path), gsv_telegram (read-only Godfather bind status), gsv_telegram_bus_send / gsv_telegram_bus_poll (Godfather channel envelopes; requires telegram-relay; no webhook / no Cloudflare), gsv_tickets (list the join board), and gsv_tickets_claim {id} (claim is allowed on MCP; requires ticket-claim workflow; unknown id is a tool error). Band 169 Telegram bus is landed. Do not invent 170. Next drain is an owner pick after a warnings-first scan. gsv_xtask task=sync (read-only vision drift). gsv_vision_sync remirrors snapshots and notifies subscribed gsv:// resources. For model routing call gsv_omni_route (task=rust|web, prefer_free) so cooldown timers skip exhausted free hosts. Cursor attaches over HTTP url http://127.0.0.1:9999/mcp (live gsv-server). Check GET /mcp crate_version vs version (version_lag); a stale live copy is why tools go missing. gsv_watchdog debug_newer means recopy after cargo test (do not kill target/live before tests). Stdio MCP is target/live/gsv-mcp.exe for OpenCode/Grok (cargo xtask live copies it; do not cargo run --bin gsv-mcp). Product tests/benches/scripts are cargo xtask / tests/*.rs / benches/*.rs — do not add .sh/.ps1/JSON harnesses. cargo xtask bump --band N locksteps the vision queue (last/next/active). Propose the next ≤10 PH-S* after the last closed band. Do not push mid-drain. Invoke cargo via MSYS2 bash.",
    },
];

/// Stable resource URI list (tests / GET /mcp).
pub fn resource_uris() -> &'static [&'static str] {
    RESOURCE_URIS
}

/// Stable prompt name list (tests / GET /mcp).
pub fn prompt_names() -> &'static [&'static str] {
    PROMPT_NAMES
}

const TOOL_NAMES: &[&str] = &[
    "gsv_health",
    "gsv_tracker",
    "gsv_ratio",
    "gsv_sli",
    "gsv_toolchain",
    "gsv_vision_manifest",
    "gsv_vision_feed",
    "gsv_vision_queue",
    "gsv_omni_chat",
    "gsv_omni_route",
    "gsv_ide_sessions",
    "gsv_terminal",
    "gsv_vision_map",
    "gsv_vision_board",
    "gsv_vision_progress",
    "gsv_vision_speeds",
    "gsv_vision_rust",
    "gsv_hooks_tests",
    "gsv_hooks_bench",
    "gsv_update",
    "gsv_vision",
    "gsv_vision_sprint_map",
    "gsv_vision_doc_preview",
    "gsv_vision_node_search",
    "gsv_vision_sync",
    "gsv_vision_extensions",
    "gsv_preview",
    "gsv_products",
    "gsv_products_scan",
    "gsv_products_select",
    "gsv_watchdog",
    "gsv_sw",
    "gsv_fingerprints",
    "gsv_xtask",
    "gsv_disk",
    "gsv_usage",
    "gsv_settings",
    "gsv_telegram",
    "gsv_telegram_bus_send",
    "gsv_telegram_bus_poll",
    "gsv_tickets",
    "gsv_tickets_claim",
];

/// Stable tool name list (tests / GET /mcp).
pub fn tool_names() -> &'static [&'static str] {
    TOOL_NAMES
}

/// Parse one NDJSON line (stdio). Empty lines are ignored.
///
/// Pending MCP notifications are flushed **before** the JSON-RPC response so a
/// Cursor/OpenCode stdio client sees `notifications/message` and
/// `notifications/resources/updated` on the same turn.
pub async fn handle_line(state: &AppState, line: &str) -> Option<String> {
    let line = line.trim().trim_end_matches('\r');
    if line.is_empty() {
        return None;
    }
    let response = match serde_json::from_str::<Value>(line) {
        Ok(v) => handle_value_in(state, v, Some("stdio")).await,
        Err(e) => Some(rpc_error(None, -32700, format!("parse: {e}"))),
    };
    let mut lines: Vec<String> = state
        .drain_mcp_notifications()
        .into_iter()
        .map(|v| v.to_string())
        .collect();
    if let Some(r) = response {
        lines.push(r.to_string());
    }
    if lines.is_empty() {
        None
    } else {
        Some(lines.join("\n"))
    }
}

/// Handle a JSON-RPC value (object or batch array).
pub async fn handle_value(state: &AppState, value: Value) -> Option<Value> {
    handle_value_in(state, value, None).await
}

/// Same as [`handle_value`], with an optional MCP/HTTP session id for token usage.
pub async fn handle_value_in(
    state: &AppState,
    value: Value,
    session: Option<&str>,
) -> Option<Value> {
    match value {
        Value::Array(items) => {
            let mut out = Vec::new();
            for item in items {
                if let Some(r) = handle_one(state, item, session).await {
                    out.push(r);
                }
            }
            if out.is_empty() {
                None
            } else {
                Some(Value::Array(out))
            }
        }
        other => handle_one(state, other, session).await,
    }
}

/// JSON-RPC error object (`id` may be null).
pub fn rpc_error(id: Option<Value>, code: i32, message: impl Into<String>) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id.unwrap_or(Value::Null),
        "error": { "code": code, "message": message.into() }
    })
}

/// Relative stdio path clients should spawn (live copy, no cargo lock).
pub fn stdio_live_rel() -> &'static str {
    if cfg!(windows) {
        "target/live/gsv-mcp.exe"
    } else {
        "target/live/gsv-mcp"
    }
}

/// Loopback Streamable HTTP URL (Cursor `url`; same process as Galaxy).
pub fn http_url() -> String {
    crate::mcp_http_url()
}

/// Discovery payload for `GET /mcp` (not a JSON-RPC session).
pub fn http_info(state: &AppState) -> Value {
    let tools = tool_names();
    let resources = resource_uris();
    let prompts = prompt_names();
    let subscriptions = state.mcp_subscription_list();
    let subscription_count = subscriptions.len();
    json!({
        "ok": true,
        "name": SERVER_ID,
        "protocol": PROTOCOL_VERSION,
        "transport": "streamable-http",
        "version": &*state.version,
        "crate_version": crate::boxes::update::crate_version(&state.repo_root),
        "version_lag": crate::boxes::update::version_lag(&state.repo_root, state.version.as_ref()),
        "sandbox": crate::boxes::products::display_path(&state.repo_root),
        "stdio": "gsv-mcp",
        "stdio_live": stdio_live_rel(),
        "http": "/mcp",
        "http_url": http_url(),
        "http_csrf": false,
        "sse": true,
        "streamable": true,
        "sessions": true,
        "session_count": state.mcp_session_count(),
        "tools": tools,
        "tool_count": tools.len(),
        "resources": resources,
        "resource_count": resources.len(),
        "prompts": prompts,
        "prompt_count": prompts.len(),
        "logging": true,
        "completions": true,
        "subscribe": true,
        "log_level": log_level_name(state.mcp_log_level.load(Ordering::Relaxed)),
        "subscriptions": subscriptions,
        "subscription_count": subscription_count,
    })
}

/// True when `Accept` lists `text/event-stream` (MCP Streamable HTTP).
pub fn wants_sse(accept: Option<&str>) -> bool {
    accept.unwrap_or("").split(',').any(|part| {
        part.trim()
            .split(';')
            .next()
            .unwrap_or("")
            .eq_ignore_ascii_case("text/event-stream")
    })
}

/// One SSE `message` event wrapping a JSON-RPC value.
pub fn format_sse_message(value: &Value) -> String {
    format!("event: message\ndata: {value}\n\n")
}

/// Finite SSE body: pending notifications, then the optional JSON-RPC response.
pub fn sse_body(notes: Vec<Value>, rpc: Option<Value>) -> String {
    let mut out = String::new();
    for note in notes {
        out.push_str(&format_sse_message(&note));
    }
    if let Some(value) = rpc {
        out.push_str(&format_sse_message(&value));
    }
    out
}

/// Map a stored index to a syslog level name.
pub fn log_level_name(idx: u8) -> &'static str {
    LOG_LEVELS.get(idx as usize).copied().unwrap_or("info")
}

/// Parse a syslog level name into the stored index.
pub fn parse_log_level(name: &str) -> Option<u8> {
    LOG_LEVELS
        .iter()
        .position(|level| *level == name)
        .map(|i| i as u8)
}

fn tool(name: &str, description: &str, input_schema: Value) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": input_schema,
    })
}

fn object_schema() -> Value {
    json!({ "type": "object", "properties": {} })
}

fn rpc_result(id: Option<Value>, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id.unwrap_or(Value::Null),
        "result": result
    })
}

async fn handle_one(state: &AppState, value: Value, session: Option<&str>) -> Option<Value> {
    let obj = match value.as_object() {
        Some(o) => o,
        None => return Some(rpc_error(None, -32600, "invalid request")),
    };
    let id = obj.get("id").cloned();
    let method = obj.get("method").and_then(Value::as_str).unwrap_or("");
    let params = obj.get("params").cloned().unwrap_or_else(|| json!({}));
    if method.is_empty() {
        return Some(rpc_error(id, -32600, "invalid request"));
    }
    match method {
        "initialize" => Some(rpc_result(id, initialize_result(state))),
        "ping" => Some(rpc_result(id, json!({}))),
        "tools/list" => Some(rpc_result(id, json!({ "tools": tools_list() }))),
        "tools/call" => Some(rpc_result(id, call_tool(state, &params, session).await)),
        "resources/list" => Some(rpc_result(id, json!({ "resources": resources_list() }))),
        "resources/read" => match resources_read(state, &params) {
            Ok(result) => Some(rpc_result(id, result)),
            Err(msg) => Some(rpc_error(id, -32602, msg)),
        },
        "resources/subscribe" => match resources_subscribe(state, &params) {
            Ok(result) => Some(rpc_result(id, result)),
            Err(msg) => Some(rpc_error(id, -32602, msg)),
        },
        "resources/unsubscribe" => match resources_unsubscribe(state, &params) {
            Ok(result) => Some(rpc_result(id, result)),
            Err(msg) => Some(rpc_error(id, -32602, msg)),
        },
        "prompts/list" => Some(rpc_result(id, json!({ "prompts": prompts_list() }))),
        "prompts/get" => match prompts_get(&params) {
            Ok(result) => Some(rpc_result(id, result)),
            Err(msg) => Some(rpc_error(id, -32602, msg)),
        },
        "logging/setLevel" => match logging_set_level(state, &params) {
            Ok(result) => Some(rpc_result(id, result)),
            Err(msg) => Some(rpc_error(id, -32602, msg)),
        },
        "completion/complete" => match completion_complete(&params) {
            Ok(result) => Some(rpc_result(id, result)),
            Err(msg) => Some(rpc_error(id, -32602, msg)),
        },
        "notifications/initialized" | "notifications/cancelled" => None,
        _ if id.is_none() => None,
        _ => Some(rpc_error(id, -32601, format!("method not found: {method}"))),
    }
}

fn initialize_result(state: &AppState) -> Value {
    json!({
        "protocolVersion": PROTOCOL_VERSION,
        "capabilities": {
            "tools": { "listChanged": false },
            "resources": { "subscribe": true, "listChanged": false },
            "prompts": { "listChanged": false },
            "logging": {},
            "completions": {}
        },
        "serverInfo": {
            "name": SERVER_ID,
            "version": *state.version,
        },
        "instructions": "GSV box tools plus allowlisted gsv:// resources and prompts. Sandbox is this GSV repo (preview/terminal/vision). VDT products are gsv_products_* allowlist only (no open/apply/tunnel tools). Cursor folder MCP is GSV/.cursor/mcp.json — not User scope (User leaks into PoolAI). Cursor 3.16 Streamable HTTP still uses type=http url http://127.0.0.1:9999/mcp on the live gsv-server (do not Origin-host this kit). Stdio is target/live/gsv-mcp.exe (copied by cargo xtask live) for OpenCode/Grok. GET /mcp reports version and sandbox — if version lags the crate, recopy the live server. resources/subscribe is process-local; gsv_vision_sync notifies every subscribed gsv:// URI. gsv_xtask task=sync is --check drift only. completion/complete covers resource URIs and prompt names. logging/setLevel filters notifications/message. HTTP GET with a session + Accept: text/event-stream holds SSE (no session stays a finite flush). HTTP initialize issues Mcp-Session-Id (DELETE /mcp ends it; unknown id → 404). POST /mcp skips browser CSRF (bots). Terminal uses the HTTP SLI allowlist. Omni chat defaults to dry-run."
    })
}

fn resources_list() -> Vec<Value> {
    RESOURCES
        .iter()
        .map(|r| {
            json!({
                "uri": r.uri,
                "name": r.name,
                "description": r.description,
                "mimeType": r.mime
            })
        })
        .collect()
}

fn uri_rejected(uri: &str) -> bool {
    uri.is_empty() || uri.contains("..") || uri.contains('\\') || !uri.starts_with("gsv://")
}

fn allowlisted_uri(uri: &str) -> Result<&'static str, String> {
    if uri.is_empty() {
        return Err("uri required".into());
    }
    if uri_rejected(uri) {
        return Err("unknown resource".into());
    }
    RESOURCE_URIS
        .iter()
        .copied()
        .find(|u| *u == uri)
        .ok_or_else(|| "unknown resource".to_string())
}

fn mcp_log(state: &AppState, level: &str, data: Value) {
    let Some(idx) = parse_log_level(level) else {
        return;
    };
    let min = state.mcp_log_level.load(Ordering::Relaxed);
    if idx < min {
        return;
    }
    state.push_mcp_notification(json!({
        "jsonrpc": "2.0",
        "method": "notifications/message",
        "params": {
            "level": level,
            "logger": SERVER_ID,
            "data": data
        }
    }));
}

fn notify_resource_updated(state: &AppState, uri: &str) {
    state.push_mcp_notification(json!({
        "jsonrpc": "2.0",
        "method": "notifications/resources/updated",
        "params": { "uri": uri }
    }));
    mcp_log(
        state,
        "debug",
        json!({ "event": "resource_updated", "uri": uri }),
    );
}

fn notify_subscribed_resources(state: &AppState) {
    let subs = match state.mcp_subscriptions.read() {
        Ok(g) => g.clone(),
        Err(_) => return,
    };
    for uri in RESOURCE_URIS.iter().copied() {
        if subs.contains(uri) {
            notify_resource_updated(state, uri);
        }
    }
}

fn resources_subscribe(state: &AppState, params: &Value) -> Result<Value, String> {
    let uri = params.get("uri").and_then(Value::as_str).unwrap_or("");
    let uri = allowlisted_uri(uri)?;
    if let Ok(mut subs) = state.mcp_subscriptions.write() {
        subs.insert(uri.to_string());
    }
    mcp_log(state, "info", json!({ "event": "subscribe", "uri": uri }));
    Ok(json!({}))
}

fn resources_unsubscribe(state: &AppState, params: &Value) -> Result<Value, String> {
    let uri = params.get("uri").and_then(Value::as_str).unwrap_or("");
    let uri = allowlisted_uri(uri)?;
    if let Ok(mut subs) = state.mcp_subscriptions.write() {
        subs.remove(uri);
    }
    mcp_log(state, "info", json!({ "event": "unsubscribe", "uri": uri }));
    Ok(json!({}))
}

fn resources_read(state: &AppState, params: &Value) -> Result<Value, String> {
    let uri = params.get("uri").and_then(Value::as_str).unwrap_or("");
    if uri.is_empty() {
        return Err("uri required".into());
    }
    if uri_rejected(uri) {
        return Err("unknown resource".into());
    }
    let spec = RESOURCES
        .iter()
        .find(|r| r.uri == uri)
        .ok_or_else(|| "unknown resource".to_string())?;
    let path = crate::boxes::preview::resolve(&state.repo_root, spec.rel)?;
    let text = std::fs::read_to_string(&path).map_err(|e| format!("read: {e}"))?;
    Ok(json!({
        "contents": [{
            "uri": spec.uri,
            "mimeType": spec.mime,
            "text": text
        }]
    }))
}

fn prompts_list() -> Vec<Value> {
    PROMPTS
        .iter()
        .map(|p| {
            json!({
                "name": p.name,
                "description": p.description,
                "arguments": []
            })
        })
        .collect()
}

fn prompts_get(params: &Value) -> Result<Value, String> {
    let name = params.get("name").and_then(Value::as_str).unwrap_or("");
    if name.is_empty() {
        return Err("name required".into());
    }
    let spec = PROMPTS
        .iter()
        .find(|p| p.name == name)
        .ok_or_else(|| "unknown prompt".to_string())?;
    Ok(json!({
        "description": spec.description,
        "messages": [{
            "role": "user",
            "content": { "type": "text", "text": spec.text }
        }]
    }))
}

fn logging_set_level(state: &AppState, params: &Value) -> Result<Value, String> {
    let level = params.get("level").and_then(Value::as_str).unwrap_or("");
    let idx = parse_log_level(level).ok_or_else(|| "invalid log level".to_string())?;
    state.mcp_log_level.store(idx, Ordering::Relaxed);
    Ok(json!({}))
}

fn completion_prefix_rejected(value: &str) -> bool {
    value.contains("..") || value.contains('\\') || value.contains("file:")
}

fn completion_complete(params: &Value) -> Result<Value, String> {
    let ref_obj = params
        .get("ref")
        .ok_or_else(|| "ref required".to_string())?;
    let ref_type = ref_obj.get("type").and_then(Value::as_str).unwrap_or("");
    let value = params
        .get("argument")
        .and_then(|a| a.get("value"))
        .and_then(Value::as_str)
        .unwrap_or("");
    if completion_prefix_rejected(value) {
        return Err("invalid completion prefix".into());
    }
    let values: Vec<&str> = match ref_type {
        "ref/resource" => {
            let uri_hint = ref_obj.get("uri").and_then(Value::as_str).unwrap_or("");
            if !uri_hint.is_empty() && completion_prefix_rejected(uri_hint) {
                return Err("invalid completion prefix".into());
            }
            RESOURCE_URIS
                .iter()
                .copied()
                .filter(|uri| uri.starts_with(value))
                .take(COMPLETION_MAX)
                .collect()
        }
        "ref/prompt" => PROMPT_NAMES
            .iter()
            .copied()
            .filter(|name| name.starts_with(value))
            .take(COMPLETION_MAX)
            .collect(),
        _ => return Err("unknown completion ref".into()),
    };
    let total = values.len();
    Ok(json!({
        "completion": {
            "values": values,
            "total": total,
            "hasMore": false
        }
    }))
}

async fn call_tool(state: &AppState, params: &Value, session: Option<&str>) -> Value {
    let name = params.get("name").and_then(Value::as_str).unwrap_or("");
    let args = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));
    match name {
        "gsv_health" => tool_ok(health_payload(state)),
        "gsv_tracker" => tool_ok(tracker_payload(state)),
        "gsv_ratio" => tool_ok(ratio::wire(&state.data_dir)),
        "gsv_sli" => tool_ok(to_json(sli::wire(&state.repo_root))),
        "gsv_toolchain" => tool_ok(to_json(toolchain::wire(&state.repo_root))),
        "gsv_vision_manifest" => tool_ok(crate::boxes::vision::wire_manifest(
            &state.repo_root,
            &state.data_dir,
        )),
        "gsv_vision_feed" => tool_ok(crate::boxes::vision::wire_feed(
            &state.repo_root,
            &state.data_dir,
        )),
        "gsv_vision_queue" => tool_ok(crate::boxes::vision::wire_sprint_queue(
            &state.repo_root,
            &state.data_dir,
        )),
        "gsv_ide_sessions" => {
            let sel = state.ide_selection.try_read().ok();
            tool_ok(to_json(ide::wire(sel.as_deref().and_then(|s| s.as_ref()))))
        }
        "gsv_terminal" => tool_terminal(state, &args),
        "gsv_omni_chat" => tool_omni(state, &args, session).await,
        "gsv_omni_route" => tool_omni_route(state, &args).await,
        "gsv_vision_map" => tool_ok(crate::boxes::vision::wire_map(
            &state.repo_root,
            &state.data_dir,
        )),
        "gsv_vision_board" => tool_ok(crate::boxes::vision::wire_sprint_board(
            &state.repo_root,
            &state.data_dir,
        )),
        "gsv_vision_progress" => tool_ok(crate::boxes::vision::wire_sprint_progress(
            &state.repo_root,
            &state.data_dir,
        )),
        "gsv_vision_speeds" => tool_ok(crate::boxes::vision::wire_speed_index(
            &state.repo_root,
            &state.data_dir,
        )),
        "gsv_vision_rust" => tool_ok(crate::boxes::vision::wire_rust_diagnostics(
            &state.repo_root,
            &state.data_dir,
        )),
        "gsv_hooks_tests" => tool_ok(to_json(hooks::tests_wire(&state.repo_root))),
        "gsv_hooks_bench" => tool_ok(to_json(hooks::bench_wire(&state.repo_root))),
        "gsv_update" => tool_ok(to_json(update::wire(state))),
        "gsv_vision" => tool_ok(crate::boxes::vision::wire_summary(
            &state.repo_root,
            &state.data_dir,
        )),
        "gsv_vision_sprint_map" => tool_ok(crate::boxes::vision::wire_sprint_map(
            &state.repo_root,
            &state.data_dir,
        )),
        "gsv_vision_doc_preview" => {
            let id = arg_str(&args, "id");
            let id = if id.is_empty() {
                "galaxy_grid"
            } else {
                id.as_str()
            };
            tool_ok(crate::boxes::vision::wire_doc_preview(
                &state.repo_root,
                &state.data_dir,
                id,
            ))
        }
        "gsv_vision_node_search" => {
            let q = arg_str(&args, "q");
            let layer = args
                .get("layer")
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty());
            tool_ok(crate::boxes::vision::wire_node_search(
                &state.repo_root,
                &state.data_dir,
                &q,
                layer,
            ))
        }
        "gsv_vision_sync" => {
            crate::boxes::usage::merge_omniroute_pull(state).await;
            let out = tool_ok(crate::boxes::vision::wire_sync(
                &state.repo_root,
                &state.data_dir,
            ));
            notify_subscribed_resources(state);
            out
        }
        "gsv_vision_extensions" => tool_ok(crate::boxes::vision::wire_extensions(
            &state.repo_root,
            &state.data_dir,
        )),
        "gsv_preview" => tool_preview(state, &args),
        "gsv_products" => {
            let sel = state.product_selected.lock().ok().and_then(|g| g.clone());
            tool_ok(crate::boxes::products::wire(
                &state.repo_root,
                sel.as_deref(),
            ))
        }
        "gsv_products_scan" => {
            let mut id = arg_str(&args, "id");
            if id.is_empty() {
                id = state
                    .product_selected
                    .lock()
                    .ok()
                    .and_then(|g| g.clone())
                    .unwrap_or_default();
            }
            if id.is_empty() {
                tool_err("no product selected")
            } else {
                match crate::boxes::products::scan(&state.repo_root, &id) {
                    Ok(s) => {
                        tool_ok(serde_json::to_value(&s).unwrap_or_else(|_| json!({"ok":false})))
                    }
                    Err(e) => tool_err(e),
                }
            }
        }
        "gsv_products_select" => tool_products_select(state, &args),
        "gsv_watchdog" => tool_ok(crate::boxes::watchdog::wire(&state.repo_root)),
        "gsv_sw" => tool_ok(crate::boxes::sw::wire()),
        "gsv_fingerprints" => {
            let limit = crate::boxes::fingerprint::clamp_limit(
                args.get("limit")
                    .and_then(|v| v.as_u64())
                    .map(|n| n as usize),
            );
            let sel = state.product_selected.lock().ok().and_then(|g| g.clone());
            tool_ok(crate::boxes::fingerprint::wire(
                &state.repo_root,
                sel.as_deref(),
                limit,
            ))
        }
        "gsv_xtask" => {
            let task = args
                .get("task")
                .and_then(Value::as_str)
                .unwrap_or("catalog");
            match crate::boxes::xtask::mcp_run(&state.repo_root, task) {
                Ok(v) => tool_ok(v),
                Err(e) => tool_err(e),
            }
        }
        "gsv_disk" => {
            let enforce = args
                .get("enforce")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            tool_ok(crate::boxes::xtask::disk_wire(&state.repo_root, enforce))
        }
        "gsv_usage" => tool_ok(crate::boxes::usage::wire_state(state).await),
        "gsv_settings" => tool_ok(crate::boxes::settings::wire(&state.data_dir)),
        "gsv_telegram" => {
            let dry = crate::boxes::telegram::env_dry_run();
            tool_ok(crate::boxes::telegram::status(&state.data_dir, dry).await)
        }
        "gsv_telegram_bus_send" => {
            let dry = crate::boxes::telegram::env_dry_run();
            let v = crate::boxes::telegram::bus_send(&state.data_dir, dry, &args).await;
            if v.get("ok").and_then(Value::as_bool) == Some(true) {
                tool_ok(v)
            } else {
                tool_err(
                    v.get("error")
                        .and_then(Value::as_str)
                        .unwrap_or("bus send failed"),
                )
            }
        }
        "gsv_telegram_bus_poll" => {
            let dry = crate::boxes::telegram::env_dry_run();
            let limit = args
                .get("limit")
                .and_then(Value::as_u64)
                .map(|n| n as usize);
            let v = crate::boxes::telegram::bus_poll(&state.data_dir, dry, limit).await;
            if v.get("ok").and_then(Value::as_bool) == Some(true) {
                tool_ok(v)
            } else {
                tool_err(
                    v.get("error")
                        .and_then(Value::as_str)
                        .unwrap_or("bus poll failed"),
                )
            }
        }
        "gsv_tickets" => tool_ok(crate::boxes::tickets::list(&state.repo_root)),
        "gsv_tickets_claim" => {
            let id = arg_str(&args, "id");
            if id.trim().is_empty() {
                tool_err("id required")
            } else {
                match crate::boxes::tickets::claim(
                    &state.repo_root,
                    &state.data_dir,
                    &id,
                    crate::boxes::tickets::resolve_claimed_by(),
                ) {
                    Ok(t) => tool_ok(json!({ "ok": true, "ticket": t })),
                    Err(e) => tool_err(e.to_string()),
                }
            }
        }
        "" => tool_err("missing tool name"),
        other => tool_err(format!("unknown tool: {other}")),
    }
}

fn health_payload(state: &AppState) -> Value {
    json!({
        "ok": true,
        "name": GSV_SERVER_NAME,
        "server": SERVER_ID,
        "version": *state.version,
        "crate_version": crate::boxes::update::crate_version(&state.repo_root),
        "version_lag": crate::boxes::update::version_lag(&state.repo_root, state.version.as_ref()),
        "uptime_secs": state.started_at.elapsed().map(|d| d.as_secs()).unwrap_or(0),
        "update_available": crate::boxes::update::effective_available(state),
    })
}

fn tracker_payload(state: &AppState) -> Value {
    json!({
        "sprints": state.tracker.try_read().map(|t| t.sprints().clone()).ok(),
        "records": state.tracker.try_read().map(|t| t.records().to_vec()).unwrap_or_default(),
        "generated_at": crate::vision::rfc3339_now(),
    })
}

fn arg_str(args: &Value, key: &str) -> String {
    args.get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

fn tool_products_select(state: &AppState, args: &Value) -> Value {
    let id = arg_str(args, "id");
    if id.is_empty() {
        return tool_err("id required");
    }
    let rows = crate::boxes::products::discover(&state.repo_root);
    if crate::boxes::products::lookup(&rows, &id).is_none() {
        return tool_err("unknown product");
    }
    if let Ok(mut g) = state.product_selected.lock() {
        *g = Some(id.clone());
    }
    tool_ok(json!({ "ok": true, "selected": id }))
}

fn tool_preview(state: &AppState, args: &Value) -> Value {
    let file = arg_str(args, "file");
    if file.is_empty() {
        return tool_err("file required");
    }
    match crate::boxes::preview::resolve(&state.repo_root, &file) {
        Ok(path) => match crate::boxes::preview::render(&path, &file) {
            Ok(wire) => tool_ok(to_json(wire)),
            Err(e) => tool_err(e),
        },
        Err(e) => tool_err(e),
    }
}

fn tool_terminal(state: &AppState, args: &Value) -> Value {
    let command = args
        .get("command")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    if command.is_empty() {
        return tool_err("command required");
    }
    let resp = terminal::run(&command);
    terminal::audit(&resp, &state.data_dir);
    let is_error = !resp.allowed || resp.exit_code.unwrap_or(-1) != 0;
    tool_result(to_json(resp), is_error)
}

async fn tool_omni(state: &AppState, args: &Value, session: Option<&str>) -> Value {
    let live = args.get("live").and_then(Value::as_bool).unwrap_or(false);
    let mut body = args.clone();
    if let Value::Object(map) = &mut body {
        map.remove("live");
        map.remove("task");
        map.remove("prefer_free");
        map.insert("stream".to_string(), json!(false));
    }
    let raw = match serde_json::to_vec(&body) {
        Ok(b) => b,
        Err(e) => return tool_err(format!("encode: {e}")),
    };
    let mut headers = HeaderMap::new();
    if !live {
        headers.insert("x-omni-dry-run", HeaderValue::from_static("1"));
    }
    headers.insert("x-gsv-source", HeaderValue::from_static("mcp"));
    let sid = session.unwrap_or("stdio");
    if let Ok(v) = HeaderValue::from_str(sid) {
        headers.insert("x-gsv-session", v);
    }
    if let Some(p) = args.get("provider").and_then(Value::as_str) {
        if let Ok(v) = HeaderValue::from_str(p) {
            headers.insert("x-omni-provider", v);
        }
    }
    if let Some(t) = args.get("task").and_then(Value::as_str) {
        if let Ok(v) = HeaderValue::from_str(t) {
            headers.insert("x-omni-task", v);
        }
    }
    let prefer_free = args
        .get("prefer_free")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    headers.insert(
        "x-omni-prefer-free",
        HeaderValue::from_static(if prefer_free { "1" } else { "0" }),
    );
    match chat_completions(state, &headers, &raw).await {
        Ok(res) => {
            let bytes = to_bytes(res.into_body(), crate::security::MAX_BODY_BYTES)
                .await
                .unwrap_or_default();
            let parsed = serde_json::from_slice::<Value>(&bytes)
                .unwrap_or_else(|_| json!({ "raw": String::from_utf8_lossy(&bytes) }));
            tool_ok(redact_secrets(parsed))
        }
        Err(e) => tool_err(e.message()),
    }
}

async fn tool_omni_route(state: &AppState, args: &Value) -> Value {
    let task = args.get("task").and_then(Value::as_str).unwrap_or("rust");
    let prefer_free = args
        .get("prefer_free")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    tool_ok(crate::boxes::omni::route_wire(&state.omni, task, prefer_free).await)
}

fn to_json<T: serde::Serialize>(v: T) -> Value {
    serde_json::to_value(v).unwrap_or_else(|_| json!({ "ok": false, "error": "serialize" }))
}

fn tool_ok(v: Value) -> Value {
    tool_result(v, false)
}

fn tool_err(msg: impl Into<String>) -> Value {
    tool_result(json!({ "ok": false, "error": msg.into() }), true)
}

fn tool_result(v: Value, is_error: bool) -> Value {
    let text = redact_secrets(v).to_string();
    json!({
        "content": [{ "type": "text", "text": text }],
        "isError": is_error
    })
}

fn redact_secrets(value: Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut out = serde_json::Map::new();
            for (k, v) in map {
                let key_l = k.to_ascii_lowercase();
                if key_l == "token_set" || key_l.contains("tokens") {
                    out.insert(k, redact_secrets(v));
                } else if SECRET_KEYS.iter().any(|s| key_l.contains(s)) {
                    out.insert(k, json!("[redacted]"));
                } else {
                    out.insert(k, redact_secrets(v));
                }
            }
            Value::Object(out)
        }
        Value::Array(items) => Value::Array(items.into_iter().map(redact_secrets).collect()),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gsv_version;
    use tokio::sync::broadcast;

    fn state() -> AppState {
        let (tx, _rx) = broadcast::channel(8);
        AppState::new(
            Some(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))),
            None,
            tx,
        )
    }

    #[tokio::test]
    async fn initialize_names_openbot() {
        let s = state();
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": { "protocolVersion": PROTOCOL_VERSION, "capabilities": {}, "clientInfo": { "name": "t", "version": "0" } }
        });
        let out = handle_value(&s, req).await.expect("response");
        assert_eq!(out["result"]["serverInfo"]["name"], SERVER_ID);
        assert_eq!(out["result"]["protocolVersion"], PROTOCOL_VERSION);
        assert_eq!(out["result"]["serverInfo"]["version"], gsv_version());
    }

    #[tokio::test]
    async fn tools_list_covers_box_wraps() {
        let names = tool_names();
        for n in [
            "gsv_health",
            "gsv_tracker",
            "gsv_ratio",
            "gsv_sli",
            "gsv_toolchain",
            "gsv_vision_manifest",
            "gsv_vision_feed",
            "gsv_vision_queue",
            "gsv_omni_chat",
            "gsv_omni_route",
            "gsv_ide_sessions",
            "gsv_terminal",
            "gsv_vision_map",
            "gsv_vision_board",
            "gsv_vision_progress",
            "gsv_vision_speeds",
            "gsv_vision_rust",
            "gsv_hooks_tests",
            "gsv_hooks_bench",
            "gsv_update",
            "gsv_vision",
            "gsv_vision_sprint_map",
            "gsv_vision_doc_preview",
            "gsv_vision_node_search",
            "gsv_vision_sync",
            "gsv_vision_extensions",
            "gsv_preview",
            "gsv_products",
            "gsv_products_scan",
            "gsv_products_select",
            "gsv_watchdog",
            "gsv_sw",
            "gsv_fingerprints",
            "gsv_xtask",
            "gsv_disk",
            "gsv_usage",
            "gsv_settings",
            "gsv_telegram",
            "gsv_telegram_bus_send",
            "gsv_telegram_bus_poll",
            "gsv_tickets",
            "gsv_tickets_claim",
        ] {
            assert!(names.contains(&n), "missing {n}");
        }
        assert_eq!(names.len(), TOOL_NAMES.len());
    }

    #[tokio::test]
    async fn health_tool_ok() {
        let s = state();
        let req = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": { "name": "gsv_health", "arguments": {} }
        });
        let out = handle_value(&s, req).await.expect("response");
        let text = out["result"]["content"][0]["text"].as_str().unwrap_or("");
        assert!(text.contains(SERVER_ID));
        assert_eq!(out["result"]["isError"], false);
    }

    #[tokio::test]
    async fn terminal_rejects_extra_shell() {
        let s = state();
        let req = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": { "name": "gsv_terminal", "arguments": { "command": "bash" } }
        });
        let out = handle_value(&s, req).await.expect("response");
        assert_eq!(out["result"]["isError"], true);
        let text = out["result"]["content"][0]["text"].as_str().unwrap_or("");
        assert!(text.contains("whitelist") || text.contains("allowed"));
    }

    #[tokio::test]
    async fn unknown_method_is_rpc_error() {
        let s = state();
        let req = json!({ "jsonrpc": "2.0", "id": 9, "method": "nope" });
        let out = handle_value(&s, req).await.expect("response");
        assert_eq!(out["error"]["code"], -32601);
    }

    #[test]
    fn redact_strips_api_key() {
        let v = json!({ "api_key": "sk-secret", "ok": true, "nested": { "authorization": "Bearer x" } });
        let r = redact_secrets(v);
        assert_eq!(r["api_key"], "[redacted]");
        assert_eq!(r["nested"]["authorization"], "[redacted]");
        assert_eq!(r["ok"], true);
        let keep = redact_secrets(json!({ "token_set": true, "bot_token": "secret" }));
        assert_eq!(keep["token_set"], true);
        assert_eq!(keep["bot_token"], "[redacted]");
    }

    async fn rpc(s: &AppState, id: u32, method: &str, params: Value) -> Value {
        handle_value(
            s,
            json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }),
        )
        .await
        .expect("rpc response")
    }

    async fn tool_text(s: &AppState, id: u32, name: &str, arguments: Value) -> (bool, String) {
        let out = rpc(
            s,
            id,
            "tools/call",
            json!({ "name": name, "arguments": arguments }),
        )
        .await;
        let err = out["result"]["isError"].as_bool().unwrap_or(true);
        let text = out["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string();
        (err, text)
    }

    #[tokio::test]
    async fn ping_returns_empty_result() {
        let s = state();
        let out = rpc(&s, 10, "ping", json!({})).await;
        assert!(out["result"].is_object());
        assert!(out.get("error").is_none());
    }

    #[tokio::test]
    async fn initialized_notification_has_no_reply() {
        let s = state();
        let out = handle_value(
            &s,
            json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }),
        )
        .await;
        assert!(out.is_none());
        let cancelled = handle_value(
            &s,
            json!({ "jsonrpc": "2.0", "method": "notifications/cancelled" }),
        )
        .await;
        assert!(cancelled.is_none());
    }

    #[tokio::test]
    async fn handle_line_skips_blank_and_reports_parse_error() {
        let s = state();
        assert!(handle_line(&s, "  \n").await.is_none());
        let err = handle_line(&s, "{not-json").await.expect("parse error");
        let v: Value = serde_json::from_str(&err).expect("json");
        assert_eq!(v["error"]["code"], -32700);
    }

    #[tokio::test]
    async fn batch_initialize_and_ping() {
        let s = state();
        let out = handle_value(
            &s,
            json!([
                { "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {} },
                { "jsonrpc": "2.0", "id": 2, "method": "ping" }
            ]),
        )
        .await
        .expect("batch");
        let arr = out.as_array().expect("array");
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["result"]["serverInfo"]["name"], SERVER_ID);
        assert!(arr[1]["result"].is_object());
    }

    #[tokio::test]
    async fn unknown_tool_is_tool_error_not_rpc_error() {
        let s = state();
        let (is_err, text) = tool_text(&s, 11, "gsv_not_a_tool", json!({})).await;
        assert!(is_err);
        assert!(text.contains("unknown tool"));
    }

    #[tokio::test]
    async fn missing_tool_name_is_error() {
        let s = state();
        let out = rpc(&s, 12, "tools/call", json!({ "arguments": {} })).await;
        assert_eq!(out["result"]["isError"], true);
    }

    #[tokio::test]
    async fn terminal_requires_command() {
        let s = state();
        let (is_err, text) = tool_text(&s, 13, "gsv_terminal", json!({})).await;
        assert!(is_err);
        assert!(text.contains("command required"));
    }

    #[tokio::test]
    async fn terminal_blocks_git_push() {
        let s = state();
        let (is_err, text) =
            tool_text(&s, 14, "gsv_terminal", json!({ "command": "git push" })).await;
        assert!(is_err);
        assert!(text.contains("not allowed") || text.contains("whitelist"));
    }

    #[tokio::test]
    async fn read_tools_return_json_payloads() {
        let s = state();
        for (id, name) in [
            (20u32, "gsv_tracker"),
            (21, "gsv_ratio"),
            (22, "gsv_sli"),
            (23, "gsv_toolchain"),
            (24, "gsv_vision_feed"),
            (25, "gsv_vision_queue"),
            (26, "gsv_ide_sessions"),
            (27, "gsv_vision_map"),
            (28, "gsv_vision_board"),
            (29, "gsv_vision_progress"),
            (31, "gsv_vision_speeds"),
            (32, "gsv_vision_rust"),
            (33, "gsv_hooks_tests"),
            (34, "gsv_hooks_bench"),
            (35, "gsv_update"),
            (36, "gsv_vision"),
            (37, "gsv_vision_sprint_map"),
            (38, "gsv_vision_sync"),
            (39, "gsv_vision_extensions"),
        ] {
            let (is_err, text) = tool_text(&s, id, name, json!({})).await;
            assert!(!is_err, "{name} isError text={text}");
            assert!(
                text.starts_with('{') || text.starts_with('['),
                "{name} {text}"
            );
            assert!(!text.contains("sk-"), "{name} leaked secret");
        }
    }

    #[tokio::test]
    async fn tools_list_schema_matches_names() {
        let s = state();
        let out = rpc(&s, 30, "tools/list", json!({})).await;
        let tools = out["result"]["tools"].as_array().expect("tools");
        assert_eq!(tools.len(), TOOL_NAMES.len());
        for (tool, expected) in tools.iter().zip(TOOL_NAMES.iter()) {
            assert_eq!(tool["name"], *expected);
            assert!(tool["inputSchema"]["type"] == "object");
            assert!(tool["description"].as_str().unwrap_or("").len() > 8);
        }
    }

    #[tokio::test]
    async fn http_info_lists_same_tools() {
        let info = http_info(&state());
        assert_eq!(info["ok"], true);
        assert_eq!(info["name"], SERVER_ID);
        assert_eq!(info["protocol"], PROTOCOL_VERSION);
        assert_eq!(info["transport"], "streamable-http");
        let listed = info["tools"].as_array().expect("tools");
        assert_eq!(listed.len(), TOOL_NAMES.len());
        assert_eq!(info["tool_count"], TOOL_NAMES.len() as u64);
        assert_eq!(info["stdio"], "gsv-mcp");
        assert_eq!(info["stdio_live"], stdio_live_rel());
        assert_eq!(info["http"], "/mcp");
        assert_eq!(info["http_url"], http_url());
        assert_eq!(info["http_url"], "http://127.0.0.1:9999/mcp");
        assert_eq!(info["version"], *state().version);
        assert_eq!(info["crate_version"], *state().version);
        assert_eq!(info["version_lag"], false);
        assert_eq!(info["http_csrf"], false);
        let sandbox = info["sandbox"].as_str().unwrap_or("");
        assert!(
            sandbox.replace('\\', "/").ends_with("/GSV"),
            "sandbox={sandbox}"
        );
        assert!(!sandbox.to_ascii_lowercase().ends_with("/poolai"));
        for forbidden in ["gsv_products_open", "gsv_tunnel", "gsv_update_apply"] {
            assert!(!TOOL_NAMES.contains(&forbidden), "{forbidden}");
        }
        assert!(stdio_live_rel().contains("gsv-mcp"));
        assert_eq!(info["sse"], true);
        assert_eq!(info["streamable"], true);
        assert_eq!(info["sessions"], true);
        assert_eq!(info["session_count"], 0);
    }

    #[test]
    fn wants_sse_parses_accept_list() {
        assert!(!wants_sse(None));
        assert!(!wants_sse(Some("application/json")));
        assert!(!wants_sse(Some("*/*")));
        assert!(wants_sse(Some("text/event-stream")));
        assert!(wants_sse(Some(
            "application/json, text/event-stream; charset=utf-8"
        )));
    }

    #[test]
    fn sse_body_emits_message_events() {
        let note = json!({"jsonrpc": "2.0", "method": "notifications/message"});
        let rpc = json!({"jsonrpc": "2.0", "id": 1, "result": {}});
        let body = sse_body(vec![note.clone()], Some(rpc.clone()));
        assert!(body.starts_with("event: message\n"), "{body}");
        assert!(body.contains(&format!("data: {note}")), "{body}");
        assert!(body.contains(&format!("data: {rpc}")), "{body}");
        assert!(body.ends_with("\n\n"), "{body}");
    }

    #[test]
    fn session_id_helpers_reject_traversal_and_accept_issued() {
        assert!(!valid_mcp_session_id(""));
        assert!(!valid_mcp_session_id("short"));
        assert!(!valid_mcp_session_id("../secret"));
        assert!(!valid_mcp_session_id("file:gsv"));
        let id = new_mcp_session_id(7);
        assert!(valid_mcp_session_id(&id), "{id}");
        assert!(id.contains('-'));
        assert!(jsonrpc_mentions_initialize(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize"
        })));
        assert!(jsonrpc_mentions_initialize(&json!([
            { "jsonrpc": "2.0", "id": 1, "method": "initialize" },
            { "jsonrpc": "2.0", "id": 2, "method": "ping" }
        ])));
        assert!(!jsonrpc_mentions_initialize(
            &json!({ "jsonrpc": "2.0", "id": 2, "method": "ping" })
        ));
        let mut headers = HeaderMap::new();
        headers.insert(MCP_SESSION_HEADER, HeaderValue::from_static("abcdefgh"));
        assert_eq!(
            mcp_session_id_from_headers(&headers).as_deref(),
            Some("abcdefgh")
        );
    }

    #[test]
    fn app_state_issues_and_deletes_http_sessions() {
        let s = state();
        assert_eq!(s.mcp_session_count(), 0);
        let id = s.mcp_issue_session();
        assert!(s.mcp_session_ok(&id));
        assert_eq!(s.mcp_session_count(), 1);
        assert!(!s.mcp_session_ok("missing-session-id"));
        assert!(s.mcp_session_delete(&id));
        assert_eq!(s.mcp_session_count(), 0);
        assert!(!s.mcp_session_delete(&id));
    }

    #[tokio::test]
    async fn omni_chat_never_echoes_api_key() {
        let s = state();
        let (is_err, text) = tool_text(
            &s,
            40,
            "gsv_omni_chat",
            json!({
                "model": "probe",
                "messages": [{ "role": "user", "content": "ping" }],
                "live": false
            }),
        )
        .await;
        let _ = is_err;
        assert!(!text.to_ascii_lowercase().contains("api_key") || text.contains("[redacted]"));
        assert!(!text.contains("sk-super"));
        assert!(!text.contains("Bearer sk"));
    }

    #[tokio::test]
    async fn omni_route_tool_returns_pick() {
        let s = state();
        let (is_err, text) = tool_text(&s, 41, "gsv_omni_route", json!({ "task": "rust" })).await;
        assert!(!is_err, "{text}");
        assert!(
            text.contains("provider") || text.contains("model"),
            "{text}"
        );
        assert!(!text.to_ascii_lowercase().contains("api_key") || text.contains("[redacted]"));
    }

    #[test]
    fn rpc_error_uses_null_id_when_missing() {
        let v = rpc_error(None, -32600, "invalid request");
        assert_eq!(v["id"], Value::Null);
        assert_eq!(v["error"]["code"], -32600);
        assert_eq!(v["jsonrpc"], "2.0");
    }

    #[tokio::test]
    async fn preview_requires_file() {
        let s = state();
        let (is_err, text) = tool_text(&s, 50, "gsv_preview", json!({})).await;
        assert!(is_err);
        assert!(text.contains("file required"));
    }

    #[tokio::test]
    async fn preview_rejects_traversal() {
        let s = state();
        let (is_err, text) =
            tool_text(&s, 51, "gsv_preview", json!({ "file": "../../etc/hosts" })).await;
        assert!(is_err);
        assert!(
            text.contains("traversal") || text.contains("outside") || text.contains("rejected"),
            "{text}"
        );
    }

    #[tokio::test]
    async fn preview_renders_cargo_toml() {
        let s = state();
        let (is_err, text) =
            tool_text(&s, 52, "gsv_preview", json!({ "file": "Cargo.toml" })).await;
        assert!(!is_err, "{text}");
        assert!(
            text.contains("Cargo.toml") || text.contains("html"),
            "{text}"
        );
    }

    #[tokio::test]
    async fn doc_preview_and_node_search_ok() {
        let s = state();
        let (is_err, text) = tool_text(
            &s,
            53,
            "gsv_vision_doc_preview",
            json!({ "id": "galaxy_grid" }),
        )
        .await;
        assert!(!is_err, "{text}");
        assert!(text.starts_with('{'), "{text}");
        let (is_err, text) = tool_text(
            &s,
            54,
            "gsv_vision_node_search",
            json!({ "q": "galaxy", "layer": "L0" }),
        )
        .await;
        assert!(!is_err, "{text}");
        assert!(text.contains("ok") || text.contains("results"), "{text}");
    }

    #[tokio::test]
    async fn initialize_advertises_resources_and_prompts() {
        let s = state();
        let out = rpc(&s, 60, "initialize", json!({})).await;
        let caps = &out["result"]["capabilities"];
        assert!(caps["tools"].is_object());
        assert!(caps["resources"].is_object());
        assert!(caps["prompts"].is_object());
        assert!(caps["logging"].is_object());
        assert!(caps["completions"].is_object());
        assert_eq!(caps["resources"]["subscribe"], true);
        assert_eq!(caps["resources"]["listChanged"], false);
        assert_eq!(caps["prompts"]["listChanged"], false);
    }

    #[tokio::test]
    async fn resources_list_is_allowlisted_gsv_uris() {
        let s = state();
        let out = rpc(&s, 61, "resources/list", json!({})).await;
        let listed = out["result"]["resources"].as_array().expect("resources");
        assert_eq!(RESOURCES.len(), RESOURCE_URIS.len());
        assert_eq!(PROMPTS.len(), PROMPT_NAMES.len());
        assert!(RESOURCE_URIS.contains(&"gsv://docs/fingerprints"));
        assert!(RESOURCE_URIS.contains(&"gsv://docs/post-always-on"));
        assert!(RESOURCE_URIS.contains(&"gsv://docs/rust-dev"));
        assert!(RESOURCE_URIS.contains(&"gsv://docs/omni-catalog"));
        assert!(RESOURCE_URIS.contains(&"gsv://docs/settings-telegram"));
        assert_eq!(listed.len(), RESOURCE_URIS.len());
        for (item, expected) in listed.iter().zip(RESOURCE_URIS.iter()) {
            assert_eq!(item["uri"], *expected);
            assert!(item["uri"].as_str().unwrap_or("").starts_with("gsv://"));
            assert!(item["mimeType"].as_str().unwrap_or("").contains('/'));
            assert!(item["name"].as_str().unwrap_or("").len() > 3);
        }
    }

    #[tokio::test]
    async fn resources_read_manifest_and_reject_unknown() {
        let s = state();
        let out = rpc(
            &s,
            62,
            "resources/read",
            json!({ "uri": "gsv://vision/manifest" }),
        )
        .await;
        let text = out["result"]["contents"][0]["text"].as_str().unwrap_or("");
        assert!(out.get("error").is_none(), "{out}");
        assert!(text.contains("nodes") || text.contains("layers"), "{text}");
        assert_eq!(out["result"]["contents"][0]["mimeType"], "application/json");

        let bad = rpc(
            &s,
            63,
            "resources/read",
            json!({ "uri": "file:///etc/passwd" }),
        )
        .await;
        assert_eq!(bad["error"]["code"], -32602);

        let trav = rpc(
            &s,
            64,
            "resources/read",
            json!({ "uri": "gsv://vision/../../../.env" }),
        )
        .await;
        assert_eq!(trav["error"]["code"], -32602);
    }

    #[tokio::test]
    async fn prompts_list_and_get_status() {
        let s = state();
        let listed = rpc(&s, 65, "prompts/list", json!({})).await;
        let prompts = listed["result"]["prompts"].as_array().expect("prompts");
        assert_eq!(prompts.len(), PROMPT_NAMES.len());
        for (item, expected) in prompts.iter().zip(PROMPT_NAMES.iter()) {
            assert_eq!(item["name"], *expected);
            assert!(item["description"].as_str().unwrap_or("").len() > 8);
        }

        let got = rpc(&s, 66, "prompts/get", json!({ "name": "gsv_status" })).await;
        assert!(got.get("error").is_none(), "{got}");
        assert_eq!(got["result"]["messages"][0]["role"], "user");
        let text = got["result"]["messages"][0]["content"]["text"]
            .as_str()
            .unwrap_or("");
        assert!(
            text.contains("gsv_health") || text.contains("ratio"),
            "{text}"
        );

        let unknown = rpc(&s, 67, "prompts/get", json!({ "name": "nope" })).await;
        assert_eq!(unknown["error"]["code"], -32602);
    }

    #[test]
    fn http_info_lists_resources_and_prompts() {
        let info = http_info(&state());
        assert_eq!(info["resource_count"], RESOURCE_URIS.len() as u64);
        assert_eq!(info["prompt_count"], PROMPT_NAMES.len() as u64);
        assert_eq!(
            info["resources"].as_array().map(|a| a.len()),
            Some(RESOURCE_URIS.len())
        );
        assert_eq!(
            info["prompts"].as_array().map(|a| a.len()),
            Some(PROMPT_NAMES.len())
        );
        assert_eq!(info["logging"], true);
        assert_eq!(info["completions"], true);
        assert_eq!(info["subscribe"], true);
        assert_eq!(info["subscription_count"], 0);
        assert_eq!(info["log_level"], "info");
    }

    #[tokio::test]
    async fn logging_set_level_updates_http_info() {
        let s = state();
        let out = rpc(&s, 70, "logging/setLevel", json!({ "level": "warning" })).await;
        assert!(out.get("error").is_none(), "{out}");
        assert!(out["result"].is_object());
        assert_eq!(http_info(&s)["log_level"], "warning");

        let bad = rpc(&s, 71, "logging/setLevel", json!({ "level": "trace" })).await;
        assert_eq!(bad["error"]["code"], -32602);
        assert_eq!(http_info(&s)["log_level"], "warning");
    }

    #[tokio::test]
    async fn completion_complete_resources_and_prompts() {
        let s = state();
        let resources = rpc(
            &s,
            72,
            "completion/complete",
            json!({
                "ref": { "type": "ref/resource", "uri": "gsv://vision/manifest" },
                "argument": { "name": "uri", "value": "gsv://vision/" }
            }),
        )
        .await;
        assert!(resources.get("error").is_none(), "{resources}");
        let values = resources["result"]["completion"]["values"]
            .as_array()
            .expect("values");
        assert_eq!(resources["result"]["completion"]["hasMore"], false);
        assert_eq!(values.len(), 3);
        assert!(values
            .iter()
            .all(|v| v.as_str().unwrap_or("").starts_with("gsv://vision/")));

        let prompts = rpc(
            &s,
            73,
            "completion/complete",
            json!({
                "ref": { "type": "ref/prompt", "name": "gsv_status" },
                "argument": { "name": "name", "value": "gsv_" }
            }),
        )
        .await;
        let pvals = prompts["result"]["completion"]["values"]
            .as_array()
            .expect("prompts");
        assert_eq!(pvals.len(), PROMPT_NAMES.len());

        let trav = rpc(
            &s,
            74,
            "completion/complete",
            json!({
                "ref": { "type": "ref/resource", "uri": "gsv://vision/manifest" },
                "argument": { "name": "uri", "value": "gsv://vision/../../../.env" }
            }),
        )
        .await;
        assert_eq!(trav["error"]["code"], -32602);

        let file_uri = rpc(
            &s,
            75,
            "completion/complete",
            json!({
                "ref": { "type": "ref/resource", "uri": "file:///etc/passwd" },
                "argument": { "name": "uri", "value": "file://" }
            }),
        )
        .await;
        assert_eq!(file_uri["error"]["code"], -32602);

        let unknown = rpc(
            &s,
            76,
            "completion/complete",
            json!({
                "ref": { "type": "ref/tool", "name": "gsv_health" },
                "argument": { "name": "name", "value": "gsv" }
            }),
        )
        .await;
        assert_eq!(unknown["error"]["code"], -32602);
    }

    #[tokio::test]
    async fn resources_subscribe_unsubscribe_and_reject() {
        let s = state();
        let ok = rpc(
            &s,
            80,
            "resources/subscribe",
            json!({ "uri": "gsv://vision/manifest" }),
        )
        .await;
        assert!(ok.get("error").is_none(), "{ok}");
        assert!(ok["result"].is_object());
        let notes = s.drain_mcp_notifications();
        assert!(
            notes.iter().any(|n| {
                n["method"] == "notifications/message"
                    && n["params"]["data"]["event"] == "subscribe"
            }),
            "{notes:?}"
        );
        assert_eq!(http_info(&s)["subscription_count"], 1);
        assert_eq!(http_info(&s)["subscriptions"][0], "gsv://vision/manifest");

        let bad = rpc(
            &s,
            81,
            "resources/subscribe",
            json!({ "uri": "file:///etc/passwd" }),
        )
        .await;
        assert_eq!(bad["error"]["code"], -32602);

        let trav = rpc(
            &s,
            82,
            "resources/subscribe",
            json!({ "uri": "gsv://vision/../../../.env" }),
        )
        .await;
        assert_eq!(trav["error"]["code"], -32602);

        let off = rpc(
            &s,
            83,
            "resources/unsubscribe",
            json!({ "uri": "gsv://vision/manifest" }),
        )
        .await;
        assert!(off.get("error").is_none(), "{off}");
        let _ = s.drain_mcp_notifications();
        assert_eq!(http_info(&s)["subscription_count"], 0);
    }

    #[tokio::test]
    async fn vision_sync_notifies_subscribed_resources() {
        let s = state();
        let _ = rpc(
            &s,
            84,
            "resources/subscribe",
            json!({ "uri": "gsv://vision/manifest" }),
        )
        .await;
        let _ = s.drain_mcp_notifications();
        let (is_err, text) = tool_text(&s, 85, "gsv_vision_sync", json!({})).await;
        assert!(!is_err, "{text}");
        let notes = s.drain_mcp_notifications();
        assert!(
            notes.iter().any(|n| {
                n["method"] == "notifications/resources/updated"
                    && n["params"]["uri"] == "gsv://vision/manifest"
            }),
            "{notes:?}"
        );
        assert!(
            notes
                .iter()
                .all(|n| n["params"]["uri"] != "gsv://vision/feed"),
            "unsubscribed vision URIs stay quiet: {notes:?}"
        );
    }

    #[tokio::test]
    async fn vision_sync_notifies_subscribed_docs() {
        let s = state();
        let _ = rpc(
            &s,
            86,
            "resources/subscribe",
            json!({ "uri": "gsv://docs/handoff" }),
        )
        .await;
        let _ = s.drain_mcp_notifications();
        let (is_err, text) = tool_text(&s, 87, "gsv_vision_sync", json!({})).await;
        assert!(!is_err, "{text}");
        let notes = s.drain_mcp_notifications();
        assert!(
            notes.iter().any(|n| {
                n["method"] == "notifications/resources/updated"
                    && n["params"]["uri"] == "gsv://docs/handoff"
            }),
            "{notes:?}"
        );
    }

    #[tokio::test]
    async fn xtask_sync_is_check_only() {
        let s = state();
        let (is_err, text) = tool_text(&s, 88, "gsv_xtask", json!({ "task": "sync" })).await;
        if is_err {
            assert!(
                text.contains("drift") || text.contains("issue"),
                "sync --check tool error should name drift: {text}"
            );
        } else {
            assert!(text.contains("check") || text.contains("drift"), "{text}");
        }
        let (is_err, text) = tool_text(&s, 89, "gsv_xtask", json!({ "task": "bump" })).await;
        assert!(is_err, "{text}");
        assert!(
            text.contains("mutating") || text.contains("catalog/products/disk/sync"),
            "{text}"
        );
    }

    #[tokio::test]
    async fn log_level_filters_subscribe_message() {
        let s = state();
        let _ = rpc(&s, 86, "logging/setLevel", json!({ "level": "error" })).await;
        let _ = s.drain_mcp_notifications();
        let _ = rpc(
            &s,
            87,
            "resources/subscribe",
            json!({ "uri": "gsv://vision/feed" }),
        )
        .await;
        let notes = s.drain_mcp_notifications();
        assert!(
            notes.iter().all(|n| n["method"] != "notifications/message"),
            "info subscribe log filtered at error: {notes:?}"
        );
        assert_eq!(http_info(&s)["subscription_count"], 1);
    }

    #[tokio::test]
    async fn handle_line_flushes_notifications_before_response() {
        let s = state();
        let raw = handle_line(
            &s,
            r#"{"jsonrpc":"2.0","id":1,"method":"resources/subscribe","params":{"uri":"gsv://docs/next"}}"#,
        )
        .await
        .expect("lines");
        let lines: Vec<&str> = raw.lines().collect();
        assert!(lines.len() >= 2, "{raw}");
        let note: Value = serde_json::from_str(lines[0]).expect("note");
        assert_eq!(note["method"], "notifications/message");
        let resp: Value = serde_json::from_str(lines.last().unwrap()).expect("resp");
        assert!(resp["result"].is_object(), "{resp}");
        assert_eq!(resp["id"], 1);
    }

    #[tokio::test]
    async fn drain_prompt_names_always_on_tools() {
        let s = state();
        let got = rpc(&s, 90, "prompts/get", json!({ "name": "gsv_drain" })).await;
        let text = got["result"]["messages"][0]["content"]["text"]
            .as_str()
            .unwrap_or("");
        assert!(text.contains("gsv_products"), "{text}");
        assert!(text.contains("gsv_products_scan"), "{text}");
        assert!(text.contains("gsv_products_select"), "{text}");
        assert!(text.contains("gsv_watchdog"), "{text}");
        assert!(text.contains("gsv_xtask"), "{text}");
        assert!(text.contains("task=sync"), "{text}");
        assert!(text.contains("gsv_vision_sync"), "{text}");
        assert!(text.contains("target/live/gsv-mcp"), "{text}");
        assert!(text.contains("http://127.0.0.1:9999/mcp"), "{text}");
        assert!(text.contains("S:/rust/GSV"), "{text}");
        assert!(text.contains("User MCP"), "{text}");
        assert!(text.contains("gsv_disk"), "{text}");
        assert!(text.contains("gsv_usage"), "{text}");
        assert!(text.contains("gsv_settings"), "{text}");
        assert!(text.contains("gsv_telegram"), "{text}");
        assert!(text.contains("gsv_telegram_bus_send"), "{text}");
        assert!(text.contains("gsv_telegram_bus_poll"), "{text}");
        assert!(text.contains("gsv_tickets"), "{text}");
        assert!(text.contains("gsv_tickets_claim"), "{text}");
        assert!(text.contains("gsv://docs/settings-telegram"), "{text}");
        assert!(text.contains("Band 169"), "{text}");
        assert!(text.contains("gsv_omni_route"), "{text}");
        assert!(text.contains("gsv://docs/next"), "{text}");
        assert!(text.contains("gsv://docs/rust-dev"), "{text}");
        assert!(text.contains("crate_version"), "{text}");
        assert!(text.contains("debug_newer"), "{text}");
        assert!(text.contains("locksteps the vision queue"), "{text}");
        assert!(text.contains("mid-drain"), "{text}");
        assert!(text.contains("3.16"), "{text}");
        assert!(text.contains("type=http"), "{text}");
    }

    #[tokio::test]
    async fn products_select_unknown_is_tool_error() {
        let s = state();
        let (is_err, text) =
            tool_text(&s, 102, "gsv_products_select", json!({ "id": "nope" })).await;
        assert!(is_err);
        assert!(text.contains("unknown product"), "{text}");
        assert!(!text.contains("unknown tool"), "{text}");
    }

    #[tokio::test]
    async fn products_scan_without_id_requires_selection() {
        let s = state();
        let (is_err, text) = tool_text(&s, 103, "gsv_products_scan", json!({})).await;
        assert!(is_err);
        assert!(
            text.contains("id required") || text.contains("no product selected"),
            "{text}"
        );
    }

    #[tokio::test]
    async fn products_select_then_scan_omits_id() {
        let s = state();
        let sel = rpc(
            &s,
            100,
            "tools/call",
            json!({ "name": "gsv_products_select", "arguments": { "id": "gsv" } }),
        )
        .await;
        assert_eq!(sel["result"]["isError"], false, "{sel}");
        let text = sel["result"]["content"][0]["text"].as_str().unwrap_or("");
        assert!(text.contains("gsv"), "{text}");
        assert!(text.contains("selected"), "{text}");

        let scan = rpc(
            &s,
            101,
            "tools/call",
            json!({ "name": "gsv_products_scan", "arguments": {} }),
        )
        .await;
        assert_eq!(scan["result"]["isError"], false, "{scan}");
        let text = scan["result"]["content"][0]["text"].as_str().unwrap_or("");
        assert!(text.contains("git_head"), "{text}");
    }

    #[tokio::test]
    async fn resources_read_post_always_on() {
        let s = state();
        let out = rpc(
            &s,
            91,
            "resources/read",
            json!({ "uri": "gsv://docs/post-always-on" }),
        )
        .await;
        let text = out["result"]["contents"][0]["text"].as_str().unwrap_or("");
        assert!(out.get("error").is_none(), "{out}");
        assert!(text.contains("band 151"), "{text}");
        let trav = rpc(&s, 92, "resources/read", json!({ "uri": "gsv://docs/../" })).await;
        assert_eq!(trav["error"]["code"], -32602);
        let spec = rpc(
            &s,
            93,
            "resources/read",
            json!({ "uri": "gsv://docs/settings-telegram" }),
        )
        .await;
        let spec_text = spec["result"]["contents"][0]["text"].as_str().unwrap_or("");
        assert!(spec.get("error").is_none(), "{spec}");
        assert!(spec_text.contains("Godfather"), "{spec_text}");
    }
}
