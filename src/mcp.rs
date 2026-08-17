//! `gsv_mcp_openbot` — MCP JSON-RPC surface over the existing boxes.
//!
//! Stdio (`gsv-mcp`) and optional HTTP `POST /mcp` share [`handle_value`].
//! Tools wrap Tracker / SLI / Toolchain / Ratio / Vision / Omni / IDE / terminal;
//! they do not add a second shell. Secrets in tool output are redacted.

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

const SECRET_KEYS: &[&str] = &["api_key", "secret", "authorization", "password", "token"];

/// Tool descriptors for `tools/list`.
pub fn tools_list() -> Vec<Value> {
    vec![
        tool("gsv_health", "GSV process health (name, version, uptime).", object_schema()),
        tool("gsv_tracker", "Tracker box: sprint snapshot and recent records.", object_schema()),
        tool("gsv_ratio", "Rust LOC ratio report (`gsv-loc-audit` store).", object_schema()),
        tool("gsv_sli", "SLI command catalog from bin/, scripts/, src/bin/.", object_schema()),
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
            "OmniRouter chat completions. Default is dry-run (no upstream). Set live=true to forward.",
            json!({
                "type": "object",
                "properties": {
                    "model": { "type": "string" },
                    "messages": { "type": "array" },
                    "provider": { "type": "string" },
                    "live": {
                        "type": "boolean",
                        "description": "If true, call the upstream provider. Default false (dry-run)."
                    }
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
    ]
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
];

/// Stable tool name list (tests / GET /mcp).
pub fn tool_names() -> &'static [&'static str] {
    TOOL_NAMES
}

/// Parse one NDJSON line (stdio). Empty lines are ignored.
pub async fn handle_line(state: &AppState, line: &str) -> Option<String> {
    let line = line.trim().trim_end_matches('\r');
    if line.is_empty() {
        return None;
    }
    match serde_json::from_str::<Value>(line) {
        Ok(v) => handle_value(state, v).await.map(|out| out.to_string()),
        Err(e) => Some(rpc_error(None, -32700, format!("parse: {e}")).to_string()),
    }
}

/// Handle a JSON-RPC value (object or batch array).
pub async fn handle_value(state: &AppState, value: Value) -> Option<Value> {
    match value {
        Value::Array(items) => {
            let mut out = Vec::new();
            for item in items {
                if let Some(r) = handle_one(state, item).await {
                    out.push(r);
                }
            }
            if out.is_empty() {
                None
            } else {
                Some(Value::Array(out))
            }
        }
        other => handle_one(state, other).await,
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

/// Discovery payload for `GET /mcp` (not a JSON-RPC session).
pub fn http_info() -> Value {
    let tools = tool_names();
    json!({
        "ok": true,
        "name": SERVER_ID,
        "protocol": PROTOCOL_VERSION,
        "transport": "streamable-http",
        "stdio": "gsv-mcp",
        "http": "/mcp",
        "tools": tools,
        "tool_count": tools.len(),
    })
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

async fn handle_one(state: &AppState, value: Value) -> Option<Value> {
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
        "tools/call" => Some(rpc_result(id, call_tool(state, &params).await)),
        "notifications/initialized" | "notifications/cancelled" => None,
        _ if id.is_none() => None,
        _ => Some(rpc_error(id, -32601, format!("method not found: {method}"))),
    }
}

fn initialize_result(state: &AppState) -> Value {
    json!({
        "protocolVersion": PROTOCOL_VERSION,
        "capabilities": { "tools": { "listChanged": false } },
        "serverInfo": {
            "name": SERVER_ID,
            "version": *state.version,
        },
        "instructions": "GSV box tools. Terminal uses the HTTP SLI allowlist. Omni chat defaults to dry-run."
    })
}

async fn call_tool(state: &AppState, params: &Value) -> Value {
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
        "gsv_omni_chat" => tool_omni(state, &args).await,
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
        "uptime_secs": state.started_at.elapsed().map(|d| d.as_secs()).unwrap_or(0),
        "update_available": state.update_available(),
    })
}

fn tracker_payload(state: &AppState) -> Value {
    json!({
        "sprints": state.tracker.try_read().map(|t| t.sprints().clone()).ok(),
        "records": state.tracker.try_read().map(|t| t.records().to_vec()).unwrap_or_default(),
        "generated_at": crate::vision::rfc3339_now(),
    })
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

async fn tool_omni(state: &AppState, args: &Value) -> Value {
    let live = args.get("live").and_then(Value::as_bool).unwrap_or(false);
    let mut body = args.clone();
    if let Value::Object(map) = &mut body {
        map.remove("live");
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
    if let Some(p) = args.get("provider").and_then(Value::as_str) {
        if let Ok(v) = HeaderValue::from_str(p) {
            headers.insert("x-omni-provider", v);
        }
    }
    match chat_completions(&state.omni, &headers, &raw).await {
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
                if SECRET_KEYS.iter().any(|s| key_l.contains(s)) {
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
        ] {
            assert!(names.contains(&n), "missing {n}");
        }
        assert_eq!(names.len(), 19);
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
        let info = http_info();
        assert_eq!(info["ok"], true);
        assert_eq!(info["name"], SERVER_ID);
        assert_eq!(info["protocol"], PROTOCOL_VERSION);
        assert_eq!(info["transport"], "streamable-http");
        let listed = info["tools"].as_array().expect("tools");
        assert_eq!(listed.len(), TOOL_NAMES.len());
        assert_eq!(info["tool_count"], TOOL_NAMES.len() as u64);
        assert_eq!(info["stdio"], "gsv-mcp");
        assert_eq!(info["http"], "/mcp");
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

    #[test]
    fn rpc_error_uses_null_id_when_missing() {
        let v = rpc_error(None, -32600, "invalid request");
        assert_eq!(v["id"], Value::Null);
        assert_eq!(v["error"]["code"], -32600);
        assert_eq!(v["jsonrpc"], "2.0");
    }
}
