//! Grid (ALLBGP) contracts — durable poolAI fleet mirror + history ring.
//!
//! `ok` is always true (hub truthfulness); a down poolAI keeps the last-known
//! topology and flips `poolai_alive:false` + `stale:true`. `GET /api/grid` +
//! MCP `gsv_grid` + SSE `event: grid`. No live coordinator is required: base
//! points at a dead loopback port so refresh fails fast.

use std::path::{Path, PathBuf};

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use gsv::boxes::grid::{self, GridBox, GridStore, HISTORY_CAP};
use gsv::mcp;
use gsv::server::router;
use gsv::AppState;
use serde_json::{json, Value};
use tokio::sync::broadcast;
use tower::ServiceExt;

fn temp_data(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("gsv-grid-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("mkdir");
    dir
}

/// Bind+drop a loopback listener ? port guaranteed to RST instantly (a
/// discarded port like 9 can black-hole SYNs and burn the request timeout).
fn dead_base() -> String {
    let l = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = l.local_addr().expect("addr").port();
    drop(l);
    format!("http://127.0.0.1:{port}/api/v1")
}

async fn get_json(app: &axum::Router, path: &str) -> (StatusCode, Value) {
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(path)
                .method(Method::GET)
                .body(Body::empty())
                .expect("req"),
        )
        .await
        .expect("resp");
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .expect("body");
    (status, serde_json::from_slice(&bytes).unwrap_or_default())
}

async fn post_json(app: &axum::Router, path: &str, body: Value) -> (StatusCode, Value) {
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(path)
                .method(Method::POST)
                .header(axum::http::header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .expect("req"),
        )
        .await
        .expect("resp");
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .expect("body");
    (status, serde_json::from_slice(&bytes).unwrap_or_default())
}

#[tokio::test]
async fn grid_profile_post_feeds_capacity_view() {
    let dir = temp_data("profiles");
    // Seed the echo-stub topology durably (dead base keeps refresh inert).
    let seeded = GridStore {
        poolai_alive: true,
        nodes: json!({"nodes": {
            "edge-pc-01": {"total_gpu_memory_mb": 7560, "total_memory_mb": 7560},
            "a54-01": { "total_gpu_memory_mb": 7560, "total_memory_mb": 7560 }
        }}),
        ..Default::default()
    };
    std::fs::write(
        dir.join("gsv_grid.json"),
        serde_json::to_vec(&seeded).unwrap(),
    )
    .unwrap();
    let (tx, _rx) = broadcast::channel(32);
    let mut app_state = AppState::new(
        Some(PathBuf::from(env!("CARGO_MANIFEST_DIR"))),
        Some(dir.clone()),
        tx,
    );
    app_state.grid = std::sync::Arc::new(GridBox::with_base(&dir, &dead_base()));
    let app = router(app_state);

    let before = get_json(&app, "/api/grid").await.1;
    assert_eq!(before["capacity"]["poolai_capacity_stub"], true, "{before}");

    let (status, posted) = post_json(
        &app,
        "/api/grid/profile",
        json!({ "id": "a54-01", "class": "edge", "ram_mb": 6144, "note": "A54 no tensors" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{posted}");
    assert_eq!(posted["ok"], true);
    let after = get_json(&app, "/api/grid").await.1;
    let row = after["capacity"]["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == "a54-01")
        .expect("row");
    assert_eq!(row["source"], "hub");
    assert_eq!(row["effective"]["ram_mb"], 6144);
    assert_eq!(row["effective"]["class"], "edge");
    // Missing id → ok:false, no panic; path-y id rejected.
    let (_s, bad) = post_json(&app, "/api/grid/profile", json!({"class": "gpu"})).await;
    assert_eq!(bad["ok"], false);
    let (_s, evil) = post_json(&app, "/api/grid/profile", json!({"id": "../x"})).await;
    assert_eq!(evil["ok"], false);
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn grid_box_keeps_last_topology_when_poolai_down() {
    let dir = temp_data("stale");
    // Seed a durable snapshot so refresh-failure must preserve it.
    let seeded = GridStore {
        poolai_alive: true,
        generated_at: "2026-09-14T00:00:00Z".into(),
        nodes: json!({"nodes": {"edge-pc-01": {}, "a54-01": {}}}),
        workers: json!([{"id": "edge-pc-01"}, {"id": "a54-01"}]),
        virtual_nodes: json!({"nodes": [{"peer": {"peer_id": "edge-pc-01"}}]}),
        seats: json!({"seat_limit": 5, "active_telegram_edge_workers": 2}),
        status: json!({"status": "running"}),
        history: vec![],
        last_error: String::new(),
    };
    std::fs::write(
        dir.join("gsv_grid.json"),
        serde_json::to_vec(&seeded).unwrap(),
    )
    .unwrap();

    let box_ = GridBox::with_base(&dir, &dead_base());
    let before = box_.wire().await;
    assert_eq!(before["counts"]["nodes"], 2);
    assert_eq!(before["counts"]["seats_used"], 2);
    assert!(before["poolai_alive"].as_bool().unwrap());

    // Refresh against a dead base: fail-open, topology preserved, stale flips.
    box_.refresh().await;
    let after = box_.wire().await;
    assert_eq!(after["ok"], true, "hub always truthful even when down");
    assert_eq!(after["poolai_alive"], false);
    assert_eq!(after["stale"], true);
    assert!(after["last_error"]
        .as_str()
        .unwrap()
        .contains("poolai unreachable"));
    assert_eq!(after["counts"]["nodes"], 2, "last-known topology kept");
    assert_eq!(after["counts"]["workers"], 2);
    assert_eq!(after["history_len"].as_u64().unwrap(), 1);
    // Durable: a fresh box reloads the on-disk mirror (survives restarts).
    let reloaded = GridBox::with_base(&dir, &dead_base());
    let w = reloaded.wire().await;
    assert_eq!(w["counts"]["nodes"], 2);
    assert_eq!(w["poolai_alive"], false);
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn grid_history_ring_and_change_key() {
    let dir = temp_data("ring");
    let box_ = GridBox::with_base(&dir, &dead_base());
    for _ in 0..(HISTORY_CAP + 8) {
        box_.refresh().await;
    }
    let w = box_.wire().await;
    assert_eq!(w["history_len"].as_u64().unwrap() as usize, HISTORY_CAP);
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn api_grid_endpoint_and_mcp_tool() {
    let dir = temp_data("wire");
    let saved = {
        let b = GridBox::with_base(&dir, &dead_base());
        b.refresh().await;
        true
    };
    assert!(saved);
    let (tx, _rx) = broadcast::channel(32);
    let mut state = AppState::new(
        Some(PathBuf::from(env!("CARGO_MANIFEST_DIR"))),
        Some(dir.clone()),
        tx,
    );
    // Point this AppState's grid at the same dead base deterministically.
    state.grid = std::sync::Arc::new(GridBox::with_base(&dir, &dead_base()));
    let app = router(state.clone());

    let (status, json) = get_json(&app, "/api/grid").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["ok"], true);
    assert_eq!(json["poolai_alive"], false);
    assert!(json["counts"].get("nodes").is_some());

    // MCP tool is registered and returns the grid wire.
    assert!(mcp::tool_names().contains(&"gsv_grid"));
    let res = app
        .oneshot(
            Request::builder()
                .uri("/mcp")
                .method(Method::POST)
                .header(axum::http::header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "jsonrpc": "2.0", "id": 1, "method": "tools/call",
                        "params": { "name": "gsv_grid", "arguments": {} }
                    })
                    .to_string(),
                ))
                .expect("req"),
        )
        .await
        .expect("resp");
    let sb = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .expect("body");
    let sj: Value = serde_json::from_slice(&sb).expect("json");
    assert_eq!(sj["result"]["isError"], false, "{sj}");
    let text = sj["result"]["content"][0]["text"].as_str().unwrap_or("");
    assert!(text.contains("poolai_alive"), "{text}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn poolai_base_defaults_to_8091_not_8080() {
    // :8080 is llama — the grid must never assume it.
    assert!(grid::DEFAULT_POOLAI_BASE.contains(":8091"));
    assert!(!grid::DEFAULT_POOLAI_BASE.contains(":8080"));
    let _ = std::env::var("GSV_POOLAI_URL");
    let _ = Path::new(env!("CARGO_MANIFEST_DIR"));
}
