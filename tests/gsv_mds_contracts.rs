//! Light memory-disk-speed app contracts (band 175).
//!
//! `GET /api/mds` and MCP `gsv_mds` wrap `boxes::mds::report`. No secrets.

use std::path::PathBuf;

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use gsv::boxes::mds;
use gsv::mcp;
use gsv::server::router;
use gsv::AppState;
use serde_json::{json, Value};
use tokio::sync::broadcast;
use tower::ServiceExt;

fn app() -> axum::Router {
    let (tx, _rx) = broadcast::channel(8);
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let state = AppState::new(Some(root), None, tx);
    router(state)
}

async fn get_json(app: &axum::Router, path: &str) -> (StatusCode, Value) {
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(path)
                .method(Method::GET)
                .body(Body::empty())
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

#[test]
fn report_ok_on_this_crate() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let r = mds::report(&root);
    assert!(r.ok, "{r:?}");
    assert_eq!(r.name, "gsv-mds");
    assert_eq!(r.memory.sample_bytes, mds::SAMPLE_BYTES as u64);
    assert!(r.memory.alloc_ns > 0);
    assert_eq!(r.speed.iters, mds::SPEED_ITERS);
    assert!(r.speed.ns_per_iter >= 1);
    let v = mds::wire(&root);
    assert_eq!(v["ok"], true);
    assert_eq!(v["speed"]["label"], "xor-fold");
}

#[tokio::test]
async fn http_mds_ok() {
    let app = app();
    let (status, json) = get_json(&app, "/api/mds").await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert_eq!(json["ok"], true, "{json}");
    assert_eq!(json["name"], "gsv-mds");
    assert!(json["disk"]["repo"].as_str().unwrap_or("").len() > 1);
}

#[tokio::test]
async fn mcp_mds_ok() {
    let app = app();
    let res = app
        .oneshot(
            Request::builder()
                .uri("/mcp")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "jsonrpc": "2.0",
                        "id": 1,
                        "method": "tools/call",
                        "params": { "name": "gsv_mds", "arguments": {} }
                    })
                    .to_string(),
                ))
                .expect("req"),
        )
        .await
        .expect("res");
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .expect("body");
    let json: Value = serde_json::from_slice(&bytes).expect("json");
    assert_eq!(json["result"]["isError"], false, "{json}");
    let text = json["result"]["content"][0]["text"].as_str().unwrap_or("");
    assert!(text.contains("gsv-mds"), "{text}");
    assert!(text.contains("xor-fold"), "{text}");
    assert!(mcp::tool_names().contains(&"gsv_mds"));
}
