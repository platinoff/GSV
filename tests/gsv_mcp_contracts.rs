//! MCP `gsv_mcp_openbot` contracts (band 135).
//!
//! Initialize + tools/list + tools/call over `POST /mcp`; GET discovery;
//! terminal stays on the HTTP allowlist (no extra shell); Omni defaults to dry-run.
//! Band 137: vision completeness (26 tools) + preview confine.
//! Band 138: resources/list+read (gsv:// allowlist) + prompts/list+get.
//! Band 139: logging/setLevel + completion/complete (resource URIs + prompt names).
//! Band 140: resources/subscribe+unsubscribe + logging notifications + resource updated.
//! Band 141: HTTP SSE (`Accept: text/event-stream`) flushes the notification queue.

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
    assert!(names.contains(&"gsv_vision_sprint_map"));
    assert!(names.contains(&"gsv_preview"));
    assert_eq!(names.len(), 26);
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
    assert_eq!(values.len(), 3);
    assert!(values
        .iter()
        .all(|v| v.as_str().unwrap_or("").starts_with("gsv://docs/")));

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
