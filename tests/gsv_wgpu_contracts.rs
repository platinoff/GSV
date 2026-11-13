//! GSV WGPU mesh API contracts — Rust integration tests (HTTP/JSON).
//!
//! Uses `tower::ServiceExt::oneshot` against the axum router (no port binding).
//! Scope: the browser-mesh wire keeps its canonical JSON shape, rejects bad
//! envelopes, and publishes the machine-readable `unknown_peer` code that lets
//! a browser peer re-join itself (band E).

use std::path::PathBuf;

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use gsv::server::router;
use gsv::AppState;
use serde_json::Value;
use tokio::sync::broadcast;
use tower::ServiceExt;

/// Build a fresh app state + router with an isolated data dir.
fn app(tag: &str) -> (axum::Router, PathBuf) {
    let (tx, _rx) = broadcast::channel(64);
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let data_dir =
        std::env::temp_dir().join(format!("gsv_wgpu_contracts_{}_{}", tag, std::process::id()));
    let _ = std::fs::remove_dir_all(&data_dir);
    std::fs::create_dir_all(&data_dir).expect("data dir");
    let state = AppState::new(Some(repo_root), Some(data_dir.clone()), tx);
    (router(state), data_dir)
}

async fn get(app: &axum::Router, path: &str) -> (StatusCode, Value) {
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
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

async fn post(app: &axum::Router, path: &str, body: Value) -> (StatusCode, Value) {
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(path)
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
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[tokio::test]
async fn peer_join_publishes_lease_and_poll_of_unknown_peer_is_coded() {
    let (app, dir) = app("join");
    let (status, joined) = post(
        &app,
        "/api/wgpu/peer/join",
        serde_json::json!({"adapter":"arm/valhall","ua":"ua"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(joined["ok"], true);
    assert_eq!(joined["peer_id"], 1);
    assert_eq!(joined["poll_secs"], 2);
    assert_eq!(joined["lease_secs"], 60);

    // Band E: polling an unknown peer is a coded error, not a silent 404.
    let (status, polled) = post(
        &app,
        "/api/wgpu/task/poll",
        serde_json::json!({"peer_id": 9999}),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(polled["ok"], false);
    assert_eq!(polled["code"], "unknown_peer");
    let err = polled["error"].as_str().unwrap_or_default();
    assert!(
        err.contains("unknown peer"),
        "error must name the peer: {err}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn enqueue_rejects_unsupported_op_and_out_of_range_iters() {
    let (app, dir) = app("enqueue");
    let (status, bad_op) = post(
        &app,
        "/api/wgpu/task/enqueue",
        serde_json::json!({"op":"fft1024"}),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(bad_op["ok"], false);

    let (status, bad_iters) = post(
        &app,
        "/api/wgpu/task/enqueue",
        serde_json::json!({"op":"matvec","iters":0}),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(bad_iters["ok"], false);

    let (status, good) = post(
        &app,
        "/api/wgpu/task/enqueue",
        serde_json::json!({"op":"matvec","iters":64}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(good["iters"], 64);
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn mesh_wire_reports_leases_capability_and_best_peer() {
    let (app, dir) = app("mesh");
    let (_, joined) = post(
        &app,
        "/api/wgpu/peer/join",
        serde_json::json!({"adapter":"arm/bifrost","ua":"ua"}),
    )
    .await;
    let peer = joined["peer_id"].as_u64().expect("peer id");

    let (status, mesh) = get(&app, "/api/wgpu/mesh").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(mesh["ok"], true);
    assert_eq!(mesh["lease_secs"], 60);
    assert_eq!(mesh["poll_secs"], 2);
    assert!(mesh["capability"].is_array());
    assert!(mesh["peers"][0]["lease_until"].as_u64().is_some());

    // A measured task turns into a capability row + best peer.
    let (_, enq) = post(
        &app,
        "/api/wgpu/task/enqueue",
        serde_json::json!({"op":"matvec","iters":64}),
    )
    .await;
    let task_id = enq["task_id"].as_str().expect("task id").to_string();
    let (_, polled) = post(
        &app,
        "/api/wgpu/task/poll",
        serde_json::json!({"peer_id": peer}),
    )
    .await;
    assert_eq!(polled["task"]["task_id"], task_id.as_str());
    assert_eq!(polled["task"]["iters"], 64);
    let (status, result) = post(
        &app,
        "/api/wgpu/task/result",
        serde_json::json!({
            "peer_id": peer,
            "task_id": task_id,
            "ok": true,
            "elapsed_ms": 1.5,
            "max_err": 0.0,
            "iters": 64,
            "mflops": 349.6,
            "ops_per_sec": 671.2,
            "device_reused": true
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(result["status"], "done");

    let (_, mesh2) = get(&app, "/api/wgpu/mesh").await;
    assert_eq!(mesh2["capability"][0]["peer_id"], peer);
    assert_eq!(mesh2["capability"][0]["measured"], 1);
    assert_eq!(mesh2["best_peer"]["peer_id"], peer);
    assert_eq!(mesh2["best_peer"]["mflops_max"], 349.6);
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn task_result_rejects_negative_throughput() {
    let (app, dir) = app("negative");
    let (_, joined) = post(
        &app,
        "/api/wgpu/peer/join",
        serde_json::json!({"adapter":"a","ua":"u"}),
    )
    .await;
    let peer = joined["peer_id"].as_u64().expect("peer id");
    let (_, enq) = post(
        &app,
        "/api/wgpu/task/enqueue",
        serde_json::json!({"op":"matvec"}),
    )
    .await;
    let task_id = enq["task_id"].as_str().expect("task id").to_string();
    let _ = post(
        &app,
        "/api/wgpu/task/poll",
        serde_json::json!({"peer_id": peer}),
    )
    .await;
    let (status, bad) = post(
        &app,
        "/api/wgpu/task/result",
        serde_json::json!({
            "peer_id": peer,
            "task_id": task_id,
            "ok": true,
            "elapsed_ms": 1.0,
            "max_err": 0.0,
            "mflops": -1.0
        }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(bad["ok"], false);
    let _ = std::fs::remove_dir_all(&dir);
}
