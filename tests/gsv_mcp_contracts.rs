//! MCP `gsv_mcp_openbot` contracts (band 135).
//!
//! Initialize + tools/list + tools/call over `POST /mcp`; GET discovery;
//! terminal stays on the HTTP allowlist (no extra shell); Omni defaults to dry-run.
//! Band 137: vision completeness (26 tools then) + preview confine.
//! Band 151: always-on catch-up (31 tools + 8 resources).
//! Band 152: `gsv_products_select` + scan-without-id.
//! Band 153: `gsv_xtask` + `gsv_disk` + `gsv://docs/rust-dev`.
//! Band 138: resources/list+read (gsv:// allowlist) + prompts/list+get.
//! Band 139: logging/setLevel + completion/complete (resource URIs + prompt names).
//! Band 140: resources/subscribe+unsubscribe + logging notifications + resource updated.
//! Band 141: HTTP SSE (`Accept: text/event-stream`) flushes the notification queue.
//! Band 142: HTTP `Mcp-Session-Id` + `DELETE /mcp`.
//! Band 185: `catalog_stale` / `catalog_hint` (restart Cursor when listed is 0).
//! Band 207: re-initialize reuses a live session; notifications-only POST →
//! 202 Accepted; initialize batch marks listed on the issued session id.

use std::path::PathBuf;
use std::time::Duration;

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use futures_util::StreamExt;
use gsv::mcp::{self, PROTOCOL_VERSION, SERVER_ID};
use gsv::server::router;
use gsv::AppState;
use serde_json::{json, Value};
use tokio::sync::broadcast;
use tower::ServiceExt;

fn app() -> axum::Router {
    let (tx, _rx) = broadcast::channel(32);
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let state = AppState::new(Some(repo_root), None, tx);
    router(state)
}

async fn mcp_post(app: &axum::Router, body: Value) -> (StatusCode, Value) {
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/mcp")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .expect("request"),
        )
        .await
        .expect("response");
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .expect("body");
    let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, json)
}

async fn mcp_get(app: &axum::Router) -> Value {
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/mcp")
                .method(Method::GET)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .expect("body");
    serde_json::from_slice(&bytes).expect("json")
}

#[tokio::test]
async fn get_mcp_discovers_openbot() {
    let app = app();
    let res = app
        .oneshot(
            Request::builder()
                .uri("/mcp")
                .method(Method::GET)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .expect("body");
    let json: Value = serde_json::from_slice(&bytes).expect("json");
    assert_eq!(json["ok"], true);
    assert_eq!(json["name"], SERVER_ID);
    assert_eq!(json["protocol"], PROTOCOL_VERSION);
    let tools = json["tools"].as_array().expect("tools");
    assert_eq!(tools.len(), mcp::tool_names().len());
    assert_eq!(json["tool_count"], tools.len() as u64);
    assert_eq!(json["tools_list_changed"], true);
    assert_eq!(json["stdio"], "gsv-mcp");
    assert!(
        json["stdio_live"]
            .as_str()
            .unwrap_or("")
            .contains("gsv-mcp"),
        "stdio_live={}",
        json["stdio_live"]
    );
    assert_eq!(json["http_csrf"], false);
    assert_eq!(json["http"], "/mcp");
    assert_eq!(json["http_url"], gsv::mcp_http_url());
    assert_eq!(json["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(json["crate_version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(json["version_lag"], false);
    let sandbox = json["sandbox"].as_str().unwrap_or("").replace('\\', "/");
    assert!(
        sandbox.ends_with("/GSV"),
        "sandbox must be the GSV crate, got {sandbox}"
    );
    let resources = json["resources"].as_array().expect("resources");
    assert_eq!(resources.len(), mcp::resource_uris().len());
    assert_eq!(json["resource_count"], resources.len() as u64);
    let prompts = json["prompts"].as_array().expect("prompts");
    assert_eq!(prompts.len(), mcp::prompt_names().len());
    assert_eq!(json["prompt_count"], prompts.len() as u64);
    assert_eq!(json["logging"], true);
    assert_eq!(json["completions"], true);
    assert_eq!(json["subscribe"], true);
    assert_eq!(json["subscription_count"], 0);
    assert_eq!(json["log_level"], "info");
    assert_eq!(json["sse"], true);
    assert_eq!(json["streamable"], true);
    assert_eq!(json["sessions"], true);
    assert_eq!(json["session_count"], 0);
    assert_eq!(json["catalog_notify"], true);
    assert_eq!(json["listed_tool_count"], 0);
    assert_eq!(json["catalog_stale"], false);
    assert_eq!(json["catalog_hint"], "");
    assert_eq!(json["session_listed"], 0);
}

#[tokio::test]
async fn post_initialize_and_tools_list() {
    let app = app();
    let (status, init) = mcp_post(
        &app,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": { "name": "gsv-test", "version": "0" }
            }
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(init["result"]["serverInfo"]["name"], SERVER_ID);
    assert!(init["result"]["capabilities"]["tools"].is_object());
    assert!(init["result"]["capabilities"]["resources"].is_object());
    assert!(init["result"]["capabilities"]["prompts"].is_object());
    assert!(init["result"]["capabilities"]["logging"].is_object());
    assert!(init["result"]["capabilities"]["completions"].is_object());
    assert_eq!(
        init["result"]["capabilities"]["resources"]["subscribe"],
        true
    );

    let (status, listed) = mcp_post(
        &app,
        json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let tools = listed["result"]["tools"].as_array().expect("tools");
    let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
    assert!(names.contains(&"gsv_health"));
    assert!(names.contains(&"gsv_terminal"));
    assert!(names.contains(&"gsv_omni_chat"));
    assert!(names.contains(&"gsv_omni_route"));
    assert!(names.contains(&"gsv_vision_sprint_map"));
    assert!(names.contains(&"gsv_preview"));
    assert!(names.contains(&"gsv_products"));
    assert!(names.contains(&"gsv_products_scan"));
    assert!(names.contains(&"gsv_products_select"));
    assert!(names.contains(&"gsv_watchdog"));
    assert!(names.contains(&"gsv_sw"));
    assert!(names.contains(&"gsv_fingerprints"));
    assert!(names.contains(&"gsv_xtask"));
    assert!(names.contains(&"gsv_disk"));
    assert!(names.contains(&"gsv_usage"));
    assert!(names.contains(&"gsv_settings"));
    assert!(names.contains(&"gsv_telegram"));
    assert!(names.contains(&"gsv_telegram_decode"));
    assert!(names.contains(&"gsv_tickets"));
    assert!(names.contains(&"gsv_tickets_claim"));
    assert!(names.contains(&"gsv_tickets_next"));
    assert_eq!(names.len(), mcp::tool_names().len());
}

#[tokio::test]
async fn catalog_stale_after_initialize_until_tools_list() {
    let app = app();
    let idle = mcp_get(&app).await;
    assert_eq!(idle["catalog_stale"], false);
    assert_eq!(idle["catalog_hint"], "");
    let (status, _) = mcp_post(
        &app,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": { "name": "gsv-test", "version": "0" }
            }
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let stale = mcp_get(&app).await;
    assert_eq!(stale["session_count"], 1);
    assert_eq!(stale["listed_tool_count"], 0);
    assert_eq!(stale["catalog_stale"], true);
    assert!(
        stale["catalog_hint"]
            .as_str()
            .unwrap_or("")
            .contains("restart Cursor"),
        "{}",
        stale["catalog_hint"]
    );
    let (status, _) = mcp_post(
        &app,
        json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let listed = mcp_get(&app).await;
    assert_eq!(listed["catalog_stale"], false);
    assert_eq!(listed["listed_tool_count"], mcp::tool_names().len() as u64);
}

#[tokio::test]
async fn post_health_and_vision_tools() {
    let app = app();
    let (status, health) = mcp_post(
        &app,
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": { "name": "gsv_health", "arguments": {} }
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(health["result"]["isError"], false);
    let text = health["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or("");
    assert!(text.contains("gsv_mcp_openbot"));
    assert!(text.contains("catalog_stale"), "{text}");
    assert!(text.contains("listed_tool_count"), "{text}");

    let (status, vis) = mcp_post(
        &app,
        json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": { "name": "gsv_vision_manifest", "arguments": {} }
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(vis["result"]["isError"], false);
}

#[tokio::test]
async fn terminal_has_no_extra_shell() {
    let app = app();
    let (status, json) = mcp_post(
        &app,
        json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "tools/call",
            "params": { "name": "gsv_terminal", "arguments": { "command": "cat README.md" } }
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["result"]["isError"], true);
    let text = json["result"]["content"][0]["text"].as_str().unwrap_or("");
    assert!(text.contains("whitelist") || text.contains("not in whitelist"));
}

#[tokio::test]
async fn omni_defaults_to_dry_run() {
    let app = app();
    let (status, json) = mcp_post(
        &app,
        json!({
            "jsonrpc": "2.0",
            "id": 6,
            "method": "tools/call",
            "params": {
                "name": "gsv_omni_chat",
                "arguments": {
                    "model": "unused",
                    "messages": [{ "role": "user", "content": "hi" }]
                }
            }
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let text = json["result"]["content"][0]["text"].as_str().unwrap_or("");
    assert!(
        text.contains("dry_run") || text.contains("no enabled provider") || text.contains("route"),
        "omni tool text={text}"
    );
    assert!(!text.contains("sk-"), "secrets must stay redacted: {text}");
}

#[test]
fn grok_project_overlay_registers_openbot() {
    let toml = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/.grok/config.toml"))
        .expect(".grok/config.toml");
    assert!(toml.contains("[mcp_servers.gsv_mcp_openbot]"));
    assert!(toml.contains("gsv-mcp"));
    assert!(toml.contains("target/live/gsv-mcp"));
    assert!(!toml.contains("command = \"cargo\""));
    assert!(toml.contains("startup_timeout_sec"));
}

#[test]
fn stdio_clients_spawn_live_gsv_mcp() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for rel in [".mcp.json", "opencode.json"] {
        let text = std::fs::read_to_string(root.join(rel)).unwrap_or_default();
        assert!(
            text.contains("target/live/gsv-mcp"),
            "{rel} must spawn the live copy: {text}"
        );
        assert!(
            !text.contains("\"cargo\""),
            "{rel} must not cargo-run MCP: {text}"
        );
    }
}

#[test]
fn cursor_mcp_uses_live_http_url() {
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/.cursor/mcp.json"))
        .expect(".cursor/mcp.json");
    assert!(
        text.contains("http://127.0.0.1:9999/mcp"),
        "Cursor must attach to live Galaxy HTTP MCP: {text}"
    );
    assert!(
        !text.contains("\"cargo\""),
        "Cursor must not cargo-run MCP: {text}"
    );
    assert!(
        !text.contains("target/live/gsv-mcp"),
        "Cursor uses HTTP url, not stdio spawn: {text}"
    );
    assert!(
        text.contains("\"type\": \"http\"") || text.contains("\"type\":\"http\""),
        "Cursor HTTP transport must be type http: {text}"
    );
}

#[test]
fn cursor_environment_baseline_pins_316() {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/.cursor/rules/cursor-environment-baseline.mdc"
    ))
    .expect("cursor-environment-baseline.mdc");
    assert!(
        text.contains("**3.16.29**"),
        "baseline must pin installed Cursor: {text}"
    );
    assert!(
        !text.contains("3.13.21"),
        "stale Cursor 3.13.21 pin: {text}"
    );
    assert!(text.contains("type: http"), "{text}");
    assert!(
        text.contains("never User") || text.contains("Never"),
        "{text}"
    );
}

#[test]
fn cursor_mcp_json_is_folder_loopback_only() {
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/.cursor/mcp.json"))
        .expect(".cursor/mcp.json");
    assert!(text.contains("127.0.0.1:9999/mcp"), "{text}");
    assert!(
        !text.contains("cursor.com"),
        "must not Origin-host MCP: {text}"
    );
}

#[test]
fn mcp_tools_omit_mutating_and_tunnel() {
    let names = mcp::tool_names();
    for forbidden in ["gsv_products_open", "gsv_tunnel", "gsv_update_apply"] {
        assert!(
            !names.contains(&forbidden),
            "{forbidden} must not be an MCP tool"
        );
    }
    assert!(names.contains(&"gsv_products"));
    assert!(names.contains(&"gsv_products_scan"));
    assert!(names.contains(&"gsv_products_select"));
}

#[tokio::test]
async fn preview_rejects_sibling_product_path() {
    let app = app();
    let (status, json) = mcp_post(
        &app,
        json!({
            "jsonrpc": "2.0",
            "id": 91,
            "method": "tools/call",
            "params": {
                "name": "gsv_preview",
                "arguments": { "file": "../poolAI/Cargo.toml" }
            }
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["result"]["isError"], true);
    let text = json["result"]["content"][0]["text"].as_str().unwrap_or("");
    assert!(
        text.contains("traversal") || text.contains("outside"),
        "{text}"
    );
}

#[tokio::test]
async fn ping_and_parse_error() {
    let app = app();
    let (status, json) =
        mcp_post(&app, json!({ "jsonrpc": "2.0", "id": 7, "method": "ping" })).await;
    assert_eq!(status, StatusCode::OK);
    assert!(json["result"].is_object());
    assert!(json.get("error").is_none());

    let (status, bad) = mcp_post(&app, json!("not-an-object")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(bad["error"]["code"], -32600);
}

#[tokio::test]
async fn notification_only_post_returns_accepted() {
    let app = app();
    let res = app
        .oneshot(
            Request::builder()
                .uri("/mcp")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({"jsonrpc":"2.0","method":"notifications/initialized"}).to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    // MCP Streamable HTTP: notifications-only input → 202 Accepted (band 207).
    assert_eq!(res.status(), StatusCode::ACCEPTED);
}

#[tokio::test]
async fn notification_only_batch_returns_accepted() {
    let app = app();
    let res = app
        .oneshot(
            Request::builder()
                .uri("/mcp")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!([
                        {"jsonrpc":"2.0","method":"notifications/initialized"},
                        {"jsonrpc":"2.0","method":"notifications/cancelled",
                         "params":{"requestId":1}}
                    ])
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(res.status(), StatusCode::ACCEPTED);
}

#[tokio::test]
async fn ide_and_tracker_tools_ok() {
    let app = app();
    for name in [
        "gsv_ide_sessions",
        "gsv_tracker",
        "gsv_sli",
        "gsv_toolchain",
    ] {
        let (status, json) = mcp_post(
            &app,
            json!({
                "jsonrpc": "2.0",
                "id": 8,
                "method": "tools/call",
                "params": { "name": name, "arguments": {} }
            }),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{name}");
        assert_eq!(json["result"]["isError"], false, "{name}");
    }
}

#[tokio::test]
async fn vision_complete_and_preview_tools() {
    let app = app();
    for (name, arguments) in [
        ("gsv_vision", json!({})),
        ("gsv_vision_sprint_map", json!({})),
        ("gsv_vision_sync", json!({})),
        ("gsv_vision_extensions", json!({})),
        ("gsv_vision_doc_preview", json!({ "id": "galaxy_grid" })),
        ("gsv_vision_node_search", json!({ "q": "sprint" })),
        ("gsv_preview", json!({ "file": "Cargo.toml" })),
    ] {
        let (status, json) = mcp_post(
            &app,
            json!({
                "jsonrpc": "2.0",
                "id": 9,
                "method": "tools/call",
                "params": { "name": name, "arguments": arguments }
            }),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{name}");
        assert_eq!(json["result"]["isError"], false, "{name} {json}");
    }

    let (status, json) = mcp_post(
        &app,
        json!({
            "jsonrpc": "2.0",
            "id": 10,
            "method": "tools/call",
            "params": { "name": "gsv_preview", "arguments": { "file": "../secret" } }
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["result"]["isError"], true);
}

#[tokio::test]
async fn resources_and_prompts_over_http() {
    let app = app();
    let (status, listed) = mcp_post(
        &app,
        json!({ "jsonrpc": "2.0", "id": 11, "method": "resources/list" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let resources = listed["result"]["resources"].as_array().expect("resources");
    assert_eq!(resources.len(), mcp::resource_uris().len());
    assert!(resources
        .iter()
        .any(|r| r["uri"] == "gsv://vision/manifest"));

    let (status, read) = mcp_post(
        &app,
        json!({
            "jsonrpc": "2.0",
            "id": 12,
            "method": "resources/read",
            "params": { "uri": "gsv://docs/mcp-openbot" }
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let text = read["result"]["contents"][0]["text"].as_str().unwrap_or("");
    assert!(text.contains("gsv_mcp_openbot"), "{text}");

    let (status, rejected) = mcp_post(
        &app,
        json!({
            "jsonrpc": "2.0",
            "id": 13,
            "method": "resources/read",
            "params": { "uri": "gsv://docs/../../../.env" }
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(rejected["error"]["code"], -32602);

    let (status, prompts) = mcp_post(
        &app,
        json!({ "jsonrpc": "2.0", "id": 14, "method": "prompts/list" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let names = prompts["result"]["prompts"].as_array().expect("prompts");
    assert_eq!(names.len(), mcp::prompt_names().len());

    let (status, got) = mcp_post(
        &app,
        json!({
            "jsonrpc": "2.0",
            "id": 15,
            "method": "prompts/get",
            "params": { "name": "gsv_drain" }
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let text = got["result"]["messages"][0]["content"]["text"]
        .as_str()
        .unwrap_or("");
    assert!(text.contains("PH-S") || text.contains("drain"), "{text}");
}

#[tokio::test]
async fn logging_and_completion_over_http() {
    let app = app();
    let (status, set) = mcp_post(
        &app,
        json!({
            "jsonrpc": "2.0",
            "id": 16,
            "method": "logging/setLevel",
            "params": { "level": "debug" }
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(set.get("error").is_none(), "{set}");

    let (status, complete) = mcp_post(
        &app,
        json!({
            "jsonrpc": "2.0",
            "id": 17,
            "method": "completion/complete",
            "params": {
                "ref": { "type": "ref/resource", "uri": "gsv://docs/next" },
                "argument": { "name": "uri", "value": "gsv://docs/" }
            }
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let values = complete["result"]["completion"]["values"]
        .as_array()
        .expect("values");
    assert_eq!(values.len(), 10);
    assert!(values
        .iter()
        .all(|v| v.as_str().unwrap_or("").starts_with("gsv://docs/")));
    assert!(values
        .iter()
        .any(|v| v.as_str() == Some("gsv://docs/rust-dev")));
    assert!(values
        .iter()
        .any(|v| v.as_str() == Some("gsv://docs/omni-catalog")));
    assert!(values
        .iter()
        .any(|v| v.as_str() == Some("gsv://docs/settings-telegram")));
    assert!(values
        .iter()
        .any(|v| v.as_str() == Some("gsv://docs/solo-squad-jail")));
    assert!(values
        .iter()
        .any(|v| v.as_str() == Some("gsv://docs/ranks")));

    let (status, rejected) = mcp_post(
        &app,
        json!({
            "jsonrpc": "2.0",
            "id": 18,
            "method": "completion/complete",
            "params": {
                "ref": { "type": "ref/resource", "uri": "gsv://vision/manifest" },
                "argument": { "name": "uri", "value": "file:///etc/passwd" }
            }
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(rejected["error"]["code"], -32602);
}

#[tokio::test]
async fn post_subscribe_updates_discovery_count() {
    let app = app();
    let (status, sub) = mcp_post(
        &app,
        json!({
            "jsonrpc": "2.0",
            "id": 19,
            "method": "resources/subscribe",
            "params": { "uri": "gsv://vision/extensions" }
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(sub.get("error").is_none(), "{sub}");

    let (status, rejected) = mcp_post(
        &app,
        json!({
            "jsonrpc": "2.0",
            "id": 20,
            "method": "resources/subscribe",
            "params": { "uri": "file:///etc/passwd" }
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(rejected["error"]["code"], -32602);

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/mcp")
                .method(Method::GET)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .expect("body");
    let json: Value = serde_json::from_slice(&bytes).expect("json");
    assert_eq!(json["subscribe"], true);
    assert_eq!(json["subscription_count"], 1);
    assert_eq!(json["subscriptions"][0], "gsv://vision/extensions");
}

#[tokio::test]
async fn mcp_post_gets_security_headers() {
    let app = app();
    let res = app
        .oneshot(
            Request::builder()
                .uri("/mcp")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({"jsonrpc":"2.0","id":1,"method":"ping"}).to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    let headers = res.headers();
    assert_eq!(
        headers
            .get("x-content-type-options")
            .and_then(|v| v.to_str().ok()),
        Some("nosniff")
    );
    assert_eq!(
        headers.get("cache-control").and_then(|v| v.to_str().ok()),
        Some("no-store")
    );
}

fn app_with_state() -> (axum::Router, AppState) {
    let (tx, _rx) = broadcast::channel(32);
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let state = AppState::new(Some(repo_root), None, tx);
    (router(state.clone()), state)
}

async fn mcp_post_sse(app: &axum::Router, body: Value) -> (StatusCode, String, String) {
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/mcp")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::ACCEPT, "application/json, text/event-stream")
                .body(Body::from(body.to_string()))
                .expect("request"),
        )
        .await
        .expect("response");
    let status = res.status();
    let ctype = res
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .expect("body");
    (status, ctype, String::from_utf8_lossy(&bytes).into_owned())
}

#[tokio::test]
async fn post_subscribe_sse_includes_notification_then_result() {
    let app = app();
    let (status, ctype, body) = mcp_post_sse(
        &app,
        json!({
            "jsonrpc": "2.0",
            "id": 21,
            "method": "resources/subscribe",
            "params": { "uri": "gsv://docs/next" }
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        ctype.starts_with("text/event-stream"),
        "content-type: {ctype}"
    );
    assert!(body.contains("event: message"), "{body}");
    assert!(body.contains("notifications/message"), "{body}");
    assert!(body.contains("\"id\":21"), "{body}");
    assert!(body.contains("\"result\""), "{body}");
}

#[tokio::test]
async fn get_mcp_sse_flushes_pending_notifications() {
    let (app, state) = app_with_state();
    state.push_mcp_notification(json!({
        "jsonrpc": "2.0",
        "method": "notifications/message",
        "params": { "level": "info", "logger": SERVER_ID, "data": { "event": "probe" } }
    }));
    let res = app
        .oneshot(
            Request::builder()
                .uri("/mcp")
                .method(Method::GET)
                .header(header::ACCEPT, "text/event-stream")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(res.status(), StatusCode::OK);
    let ctype = res
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(
        ctype.starts_with("text/event-stream"),
        "content-type: {ctype}"
    );
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .expect("body");
    let body = String::from_utf8_lossy(&bytes);
    assert!(body.contains("event: message"), "{body}");
    assert!(body.contains("\"event\":\"probe\""), "{body}");
}

#[tokio::test]
async fn get_mcp_sse_session_holds_and_emits() {
    let (app, state) = app_with_state();
    let sid = state.mcp_issue_session();
    state.push_mcp_notification(json!({
        "jsonrpc": "2.0",
        "method": "notifications/message",
        "params": { "level": "info", "logger": SERVER_ID, "data": { "event": "hold" } }
    }));
    let res = app
        .oneshot(
            Request::builder()
                .uri("/mcp")
                .method(Method::GET)
                .header(header::ACCEPT, "text/event-stream")
                .header("mcp-session-id", &sid)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(res.status(), StatusCode::OK);
    let ctype = res
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(
        ctype.starts_with("text/event-stream"),
        "content-type: {ctype}"
    );
    let mut stream = res.into_body().into_data_stream();
    let first = tokio::time::timeout(Duration::from_secs(2), stream.next())
        .await
        .expect("sse chunk timed out")
        .expect("stream ended")
        .expect("frame");
    let text = String::from_utf8_lossy(&first);
    assert!(
        text.contains("event: message") || text.contains("notifications/message"),
        "{text}"
    );
    assert!(text.contains("hold"), "{text}");
}

#[tokio::test]
async fn json_post_does_not_drop_notification_queue() {
    let (app, state) = app_with_state();
    state.push_mcp_notification(json!({
        "jsonrpc": "2.0",
        "method": "notifications/message",
        "params": { "level": "info", "logger": SERVER_ID, "data": { "event": "keep-me" } }
    }));
    let (status, _) = mcp_post(&app, json!({ "jsonrpc": "2.0", "id": 9, "method": "ping" })).await;
    assert_eq!(status, StatusCode::OK);
    let notes = state.drain_mcp_notifications();
    assert!(
        notes
            .iter()
            .any(|n| n["params"]["data"]["event"] == "keep-me"),
        "{notes:?}"
    );
}

#[tokio::test]
async fn json_initialize_keeps_list_changed_for_session_sse() {
    let app = app();
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/mcp")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "jsonrpc": "2.0",
                        "id": 1,
                        "method": "initialize",
                        "params": {
                            "protocolVersion": PROTOCOL_VERSION,
                            "capabilities": {},
                            "clientInfo": { "name": "gsv-test", "version": "0" }
                        }
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(res.status(), StatusCode::OK);
    let sid = session_header(&res).expect("Mcp-Session-Id");
    let hold = app
        .oneshot(
            Request::builder()
                .uri("/mcp")
                .method(Method::GET)
                .header(header::ACCEPT, "text/event-stream")
                .header("mcp-session-id", &sid)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(hold.status(), StatusCode::OK);
    let mut stream = hold.into_body().into_data_stream();
    let first = tokio::time::timeout(Duration::from_secs(2), stream.next())
        .await
        .expect("sse chunk timed out")
        .expect("stream ended")
        .expect("frame");
    let text = String::from_utf8_lossy(&first);
    assert!(text.contains("tools/list_changed"), "{text}");
}

fn session_header(res: &axum::http::Response<Body>) -> Option<String> {
    res.headers()
        .get("mcp-session-id")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string)
}

#[tokio::test]
async fn initialize_issues_mcp_session_id() {
    let app = app();
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/mcp")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "jsonrpc": "2.0",
                        "id": 1,
                        "method": "initialize",
                        "params": {
                            "protocolVersion": PROTOCOL_VERSION,
                            "capabilities": {},
                            "clientInfo": { "name": "gsv-test", "version": "0" }
                        }
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(res.status(), StatusCode::OK);
    let sid = session_header(&res).expect("Mcp-Session-Id");
    assert!(mcp::valid_mcp_session_id(&sid), "{sid}");
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .expect("body");
    let json: Value = serde_json::from_slice(&bytes).expect("json");
    assert_eq!(json["result"]["serverInfo"]["name"], SERVER_ID);

    let info_res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/mcp")
                .method(Method::GET)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(info_res.status(), StatusCode::OK);
    let info_bytes = axum::body::to_bytes(info_res.into_body(), usize::MAX)
        .await
        .expect("body");
    let info: Value = serde_json::from_slice(&info_bytes).expect("json");
    assert_eq!(info["sessions"], true);
    assert_eq!(info["session_count"], 1);

    let ping = app
        .oneshot(
            Request::builder()
                .uri("/mcp")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .header("mcp-session-id", &sid)
                .body(Body::from(
                    json!({"jsonrpc":"2.0","id":2,"method":"ping"}).to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(ping.status(), StatusCode::OK);
}

#[tokio::test]
async fn reinitialize_reuses_live_session() {
    let (app, state) = app_with_state();
    let init_body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": {},
            "clientInfo": { "name": "gsv-test", "version": "0" }
        }
    })
    .to_string();
    let first = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/mcp")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(init_body.clone()))
                .expect("request"),
        )
        .await
        .expect("response");
    let sid = session_header(&first).expect("Mcp-Session-Id");

    // Re-initialize carrying the live id must reuse it, not mint a second one.
    let again = app
        .oneshot(
            Request::builder()
                .uri("/mcp")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .header("mcp-session-id", &sid)
                .body(Body::from(init_body))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(again.status(), StatusCode::OK);
    let reused = session_header(&again).expect("Mcp-Session-Id on re-init");
    assert_eq!(reused, sid, "re-initialize must keep the live session id");
    assert_eq!(state.mcp_session_count(), 1, "no duplicate sessions");
}

#[tokio::test]
async fn initialize_batch_marks_listed_on_issued_session() {
    let (app, state) = app_with_state();
    let res = app
        .oneshot(
            Request::builder()
                .uri("/mcp")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!([
                        {
                            "jsonrpc": "2.0",
                            "id": 1,
                            "method": "initialize",
                            "params": {
                                "protocolVersion": PROTOCOL_VERSION,
                                "capabilities": {},
                                "clientInfo": { "name": "gsv-test", "version": "0" }
                            }
                        },
                        { "jsonrpc": "2.0", "id": 2, "method": "tools/list" }
                    ])
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(res.status(), StatusCode::OK);
    let sid = session_header(&res).expect("Mcp-Session-Id on batch initialize");
    assert!(mcp::valid_mcp_session_id(&sid), "issued id shape {sid}");
    assert_eq!(state.mcp_session_count(), 1);
    assert_eq!(
        state.mcp_session_listed_count(),
        1,
        "tools/list in the same batch must mark the issued session"
    );
    assert_eq!(
        state.mcp_listed_tool_count(),
        mcp::tool_names().len() as u32
    );
    assert!(!mcp::catalog_stale(&state));
}

#[tokio::test]
async fn unknown_mcp_session_is_not_found() {
    let app = app();
    let res = app
        .oneshot(
            Request::builder()
                .uri("/mcp")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .header("mcp-session-id", "deadbeef-00000001")
                .body(Body::from(
                    json!({"jsonrpc":"2.0","id":3,"method":"ping"}).to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .expect("body");
    let json: Value = serde_json::from_slice(&bytes).expect("json");
    assert_eq!(json["ok"], false);
    assert_eq!(json["error"], "mcp session not found");
}

#[tokio::test]
async fn delete_mcp_ends_session() {
    let app = app();
    let init = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/mcp")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "jsonrpc": "2.0",
                        "id": 1,
                        "method": "initialize",
                        "params": { "protocolVersion": PROTOCOL_VERSION, "capabilities": {}, "clientInfo": { "name": "gsv-test", "version": "0" } }
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    let sid = session_header(&init).expect("sid");

    let missing = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/mcp")
                .method(Method::DELETE)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(missing.status(), StatusCode::BAD_REQUEST);

    let gone = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/mcp")
                .method(Method::DELETE)
                .header("mcp-session-id", &sid)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(gone.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(gone.into_body(), usize::MAX)
        .await
        .expect("body");
    let json: Value = serde_json::from_slice(&bytes).expect("json");
    assert_eq!(json["ok"], true);

    let reuse = app
        .oneshot(
            Request::builder()
                .uri("/mcp")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .header("mcp-session-id", &sid)
                .body(Body::from(
                    json!({"jsonrpc":"2.0","id":4,"method":"ping"}).to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(reuse.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn get_mcp_sse_unknown_session_is_not_found() {
    let app = app();
    let res = app
        .oneshot(
            Request::builder()
                .uri("/mcp")
                .method(Method::GET)
                .header(header::ACCEPT, "text/event-stream")
                .header("mcp-session-id", "deadbeef-00000002")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn products_scan_unknown_id_is_tool_error() {
    let app = app();
    let (status, body) = mcp_post(
        &app,
        json!({
            "jsonrpc": "2.0",
            "id": 80,
            "method": "tools/call",
            "params": { "name": "gsv_products_scan", "arguments": { "id": "nope" } }
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["result"]["isError"], true);
}

#[tokio::test]
async fn products_scan_gsv_ok() {
    let app = app();
    let (status, body) = mcp_post(
        &app,
        json!({
            "jsonrpc": "2.0",
            "id": 81,
            "method": "tools/call",
            "params": { "name": "gsv_products_scan", "arguments": { "id": "gsv" } }
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["result"]["isError"], false);
    let text = body["result"]["content"][0]["text"].as_str().unwrap_or("");
    assert!(
        text.contains("\"ok\":true") || text.contains("\"ok\": true"),
        "{text}"
    );
    assert!(text.contains("git_head"), "{text}");
}

#[tokio::test]
async fn watchdog_and_sw_tools_ok() {
    let app = app();
    for (id, name, needle) in [
        (82u64, "gsv_watchdog", "alive"),
        (83, "gsv_sw", "gsv-shell-v2"),
    ] {
        let (status, body) = mcp_post(
            &app,
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "tools/call",
                "params": { "name": name, "arguments": {} }
            }),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{name}");
        assert_eq!(body["result"]["isError"], false, "{name}");
        let text = body["result"]["content"][0]["text"].as_str().unwrap_or("");
        assert!(text.contains(needle), "{name} {text}");
    }
}

#[tokio::test]
async fn fingerprints_tool_ok() {
    let app = app();
    let (status, body) = mcp_post(
        &app,
        json!({
            "jsonrpc": "2.0",
            "id": 84,
            "method": "tools/call",
            "params": { "name": "gsv_fingerprints", "arguments": { "limit": 3 } }
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["result"]["isError"], false);
    let text = body["result"]["content"][0]["text"].as_str().unwrap_or("");
    assert!(text.contains("fingerprints.jsonl"), "{text}");
}

#[tokio::test]
async fn drain_prompt_names_always_on_tools() {
    let app = app();
    let (status, got) = mcp_post(
        &app,
        json!({
            "jsonrpc": "2.0",
            "id": 90,
            "method": "prompts/get",
            "params": { "name": "gsv_drain" }
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let text = got["result"]["messages"][0]["content"]["text"]
        .as_str()
        .unwrap_or("");
    assert!(text.contains("gsv_products"), "{text}");
    assert!(text.contains("gsv_products_scan"), "{text}");
    assert!(text.contains("gsv_products_select"), "{text}");
    assert!(text.contains("gsv_watchdog"), "{text}");
    assert!(text.contains("gsv_usage"), "{text}");
    assert!(text.contains("gsv_settings"), "{text}");
    assert!(text.contains("gsv_telegram"), "{text}");
    assert!(text.contains("gsv_telegram_bus_send"), "{text}");
    assert!(text.contains("gsv_telegram_bus_poll"), "{text}");
    assert!(text.contains("gsv_telegram_ticket"), "{text}");
    assert!(text.contains("gsv_tickets"), "{text}");
    assert!(text.contains("gsv_tickets_claim"), "{text}");
    assert!(text.contains("gsv_tickets_create"), "{text}");
    assert!(text.contains("gsv_tickets_presence"), "{text}");
    assert!(text.contains("gsv_tickets_reclaim"), "{text}");
    assert!(text.contains("gsv_tickets_walk"), "{text}");
    assert!(text.contains("gsv_tickets_hook"), "{text}");
    assert!(text.contains("gsv_mds"), "{text}");
    assert!(text.contains("Band 175"), "{text}");
    assert!(text.contains("gsv://docs/settings-telegram"), "{text}");
    assert!(text.contains("Band 171"), "{text}");
    assert!(text.contains("Band 172"), "{text}");
    assert!(text.contains("Band 173"), "{text}");
    assert!(text.contains("Band 174"), "{text}");
    assert!(text.contains("Band 185"), "{text}");
    assert!(text.contains("Band 186"), "{text}");
    assert!(text.contains("gsv://docs/solo-squad-jail"), "{text}");
    assert!(text.contains("catalog_stale"), "{text}");
    assert!(text.contains("restart Cursor"), "{text}");
    assert!(text.contains("lockstep-wait"), "{text}");
    assert!(text.contains("gsv://docs/next"), "{text}");
    assert!(text.contains("http://127.0.0.1:9999/mcp"), "{text}");
    assert!(text.contains("S:/rust/GSV"), "{text}");
    assert!(text.contains("locksteps the vision queue"), "{text}");
    assert!(text.contains("close of N"), "{text}");
    assert!(text.contains("mid-drain"), "{text}");
    assert!(text.contains("3.16"), "{text}");
    assert!(text.contains("type=http"), "{text}");
}

#[tokio::test]
async fn products_select_unknown_id_is_tool_error() {
    let app = app();
    let (status, body) = mcp_post(
        &app,
        json!({
            "jsonrpc": "2.0",
            "id": 85,
            "method": "tools/call",
            "params": { "name": "gsv_products_select", "arguments": { "id": "nope" } }
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["result"]["isError"], true);
    let text = body["result"]["content"][0]["text"].as_str().unwrap_or("");
    assert!(text.contains("unknown product"), "{text}");
    assert!(!text.contains("unknown tool"), "{text}");
}

#[tokio::test]
async fn products_scan_without_id_needs_select() {
    let app = app();
    let (status, body) = mcp_post(
        &app,
        json!({
            "jsonrpc": "2.0",
            "id": 86,
            "method": "tools/call",
            "params": { "name": "gsv_products_scan", "arguments": {} }
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["result"]["isError"], true);
}

#[tokio::test]
async fn products_select_then_scan_omits_id() {
    let app = app();
    let (status, sel) = mcp_post(
        &app,
        json!({
            "jsonrpc": "2.0",
            "id": 87,
            "method": "tools/call",
            "params": { "name": "gsv_products_select", "arguments": { "id": "gsv" } }
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(sel["result"]["isError"], false);
    let text = sel["result"]["content"][0]["text"].as_str().unwrap_or("");
    assert!(text.contains("gsv"), "{text}");

    let (status, scan) = mcp_post(
        &app,
        json!({
            "jsonrpc": "2.0",
            "id": 88,
            "method": "tools/call",
            "params": { "name": "gsv_products_scan", "arguments": {} }
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(scan["result"]["isError"], false, "{scan}");
    let text = scan["result"]["content"][0]["text"].as_str().unwrap_or("");
    assert!(
        text.contains("\"ok\":true") || text.contains("\"ok\": true"),
        "{text}"
    );
    assert!(text.contains("git_head"), "{text}");
}
