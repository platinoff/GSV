//! MCP `gsv_mcp_openbot` contracts (band 135).
//!
//! Initialize + tools/list + tools/call over `POST /mcp`; GET discovery;
//! terminal stays on the HTTP allowlist (no extra shell); Omni defaults to dry-run.

use std::path::PathBuf;

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
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
    assert_eq!(json["stdio"], "gsv-mcp");
    assert_eq!(json["http"], "/mcp");
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
    assert_eq!(names.len(), 19);
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
    assert!(toml.contains("startup_timeout_sec"));
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
async fn notification_returns_no_content() {
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
    assert_eq!(res.status(), StatusCode::NO_CONTENT);
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
