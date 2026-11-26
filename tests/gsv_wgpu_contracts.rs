//! GSV WGPU mesh API contracts — Rust integration tests (HTTP/JSON).
//!
//! Uses `tower::ServiceExt::oneshot` against the axum router (no port binding).
//! Scope: the browser-mesh wire keeps its canonical JSON shape, rejects bad
//! envelopes, and publishes the machine-readable `unknown_peer` code that lets
//! a browser peer re-join itself (band E).

mod common;
use common::get;

use std::path::PathBuf;

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use base64::Engine;
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
async fn enqueue_minilm_carries_token_and_layer_and_rejects_bad_bounds() {
    let (app, dir) = app("minilm");
    let (status, good) = post(
        &app,
        "/api/wgpu/task/enqueue",
        serde_json::json!({"op":"minilm","token":7,"layer":1}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(good["ok"], true);
    assert!(good["task_id"].as_str().unwrap().starts_with("mt-"));

    // Out-of-range token (toy vocab is 0..1023) and layer (>= 2) are refused.
    let (status, bad_token) = post(
        &app,
        "/api/wgpu/task/enqueue",
        serde_json::json!({"op":"minilm","token":1024}),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(bad_token["ok"], false);

    let (status, bad_layer) = post(
        &app,
        "/api/wgpu/task/enqueue",
        serde_json::json!({"op":"minilm","layer":2}),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(bad_layer["ok"], false);
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn task_result_verifies_the_op_echo_against_the_task() {
    let (app, dir) = app("opecho");
    let (status, joined) = post(
        &app,
        "/api/wgpu/peer/join",
        serde_json::json!({"adapter": "a", "ua": "u"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let peer = joined["peer_id"].as_u64().unwrap();

    // Helper: enqueue one minilm layer task, claim it with the peer and settle
    // it with the given result envelope.
    async fn settle(app: &axum::Router, peer: u64, body: Value) -> (String, Value) {
        let (_s, enq) = post(
            app,
            "/api/wgpu/task/enqueue",
            serde_json::json!({"op": "minilm", "token": 7, "layer": 0}),
        )
        .await;
        let task_id = enq["task_id"].as_str().unwrap().to_string();
        let (_s, _poll) = post(
            app,
            "/api/wgpu/task/poll",
            serde_json::json!({"peer_id": peer}),
        )
        .await;
        let mut envelope = body;
        envelope["task_id"] = Value::String(task_id.clone());
        envelope["peer_id"] = Value::from(peer);
        let (_s, res) = post(app, "/api/wgpu/task/result", envelope).await;
        (task_id, res)
    }

    // 1) The peer echoes another op -> the result is not trustworthy -> failed.
    let (bad_id, bad) = settle(
        &app,
        peer,
        serde_json::json!({"op": "matvec", "ok": true, "elapsed_ms": 1.0, "max_err": 0.0}),
    )
    .await;
    assert_eq!(bad["status"], "failed");

    // 2) The peer echoes the task op -> done.
    let (good_id, good) = settle(
        &app,
        peer,
        serde_json::json!({"op": "minilm", "ok": true, "elapsed_ms": 1.0, "max_err": 0.0}),
    )
    .await;
    assert_eq!(good["status"], "done");

    // 3) An old peer (no echo) still settles, but the row is flagged.
    let (old_id, old) = settle(
        &app,
        peer,
        serde_json::json!({"ok": true, "elapsed_ms": 1.0, "max_err": 0.0}),
    )
    .await;
    assert_eq!(old["status"], "done");

    // The flags are visible in the mesh wire (task tail).
    let (_status, mesh) = get(&app, "/api/wgpu/mesh").await;
    let tasks = mesh["tasks"].as_array().expect("tasks");
    let row = |id: &str| {
        tasks
            .iter()
            .find(|t| t["task_id"].as_str() == Some(id))
            .cloned()
            .unwrap_or(Value::Null)
    };
    let bad_row = row(&bad_id);
    assert_eq!(bad_row["status"], "failed");
    assert!(bad_row["error"]
        .as_str()
        .unwrap_or("")
        .contains("op mismatch"));
    assert_eq!(row(&good_id)["status"], "done");
    assert_eq!(row(&good_id).get("op_unverified"), None);
    let old_row = row(&old_id);
    assert_eq!(old_row["status"], "done");
    assert_eq!(old_row["op_unverified"], true);
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn capability_and_mesh_carry_device_class() {
    let (app, dir) = app("device_class");
    let (status, joined) = post(
        &app,
        "/api/wgpu/peer/join",
        serde_json::json!({"adapter": "arm/valhall", "ua": "u", "device_class": "phone"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let peer = joined["peer_id"].as_u64().unwrap();

    let (_, mesh) = get(&app, "/api/wgpu/mesh").await;
    let peers = mesh["peers"].as_array().unwrap();
    let row = peers
        .iter()
        .find(|p| p["peer_id"] == serde_json::json!(peer))
        .expect("peer row");
    assert_eq!(row["device_class"], "phone");

    let (_, cap) = get(&app, "/api/wgpu/capability").await;
    let rows = cap["peers"].as_array().unwrap();
    let row = rows
        .iter()
        .find(|p| p["peer_id"] == serde_json::json!(peer))
        .expect("capability row");
    assert_eq!(row["device_class"], "phone");
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn weights_store_manifest_roundtrip_and_rejection() {
    let (app, dir) = app("weights");
    let (status, m0) = get(&app, "/api/wgpu/weights/manifest").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(m0["ok"], true);
    assert_eq!(m0["chunks"].as_array().unwrap().len(), 0);
    assert_eq!(m0["model"], "minicpm-v-4.7-1b");

    let payload = b"q8_0 scale block miniature for the minicpm mesh";
    let b64 = base64::engine::general_purpose::STANDARD.encode(payload);
    let (status, stored) = post(
        &app,
        "/api/wgpu/weights/chunk",
        serde_json::json!({"data_b64": b64}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(stored["ok"], true);
    let key = stored["key"].as_str().unwrap().to_string();
    assert_eq!(key.len(), 64);

    // manifest lists it; HTTP read roundtrip returns the same bytes.
    let (_s, m1) = get(&app, "/api/wgpu/weights/manifest").await;
    assert_eq!(m1["chunks"].as_array().unwrap().len(), 1);
    assert_eq!(m1["total_bytes"], payload.len() as u64);

    let (status, got) = get(&app, &format!("/api/wgpu/weights/chunk?key={key}")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(got["data_b64"], b64.clone());

    // Bad inputs are refused.
    let (status, bad_empty) = post(&app, "/api/wgpu/weights/chunk", serde_json::json!({})).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(bad_empty["ok"], false);
    let (status, bad_b64) = post(
        &app,
        "/api/wgpu/weights/chunk",
        serde_json::json!({"data_b64": "!!!notbase64!!!"}),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(bad_b64["ok"], false);
    let (status, bad_key) = get(&app, "/api/wgpu/weights/chunk?key=junk").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(bad_key["ok"], false);

    let audit = std::fs::read_to_string(dir.join("wgpu_weights_audit.jsonl")).unwrap();
    assert!(audit.contains("\"event\":\"store\""));
    assert!(audit.contains("\"event\":\"read\""));
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn capability_endpoint_lists_profiles_and_quotas() {
    let (app, dir) = app("capability");
    let (status, joined) = post(
        &app,
        "/api/wgpu/peer/join",
        serde_json::json!({"adapter": "arm/valhall", "ua": "u"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let peer = joined["peer_id"].as_u64().unwrap();

    let (status, cap) = get(&app, "/api/wgpu/capability").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(cap["ok"], true);
    assert!(cap["quotas"]["max_flops_per_task"].as_u64().unwrap() > 0);
    assert!(cap["quotas"]["max_concurrent_per_peer"].as_u64().unwrap() > 0);
    assert!(cap["quotas"]["route_sticky_secs"].as_u64().unwrap() > 0);
    let row = cap["peers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["peer_id"] == serde_json::json!(peer))
        .expect("profile");
    assert_eq!(row["adapter"], "arm/valhall");
    assert!(row["concurrent_limit"].as_u64().unwrap() > 0);
    assert_eq!(row["claimed_now"], 0);
    assert_eq!(row["within_quota"], true);
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn over_quota_enqueue_fails_fast_and_audits() {
    let (app, dir) = app("quota");
    // 200 iterations of matvec = 104 857 600 FLOP > the 64 MiB default quota.
    let (status, body) = post(
        &app,
        "/api/wgpu/task/enqueue",
        serde_json::json!({"op": "matvec", "iters": 200}),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["ok"], false);
    assert!(body["error"].as_str().unwrap_or("").contains("quota"));
    let audit = std::fs::read_to_string(dir.join("wgpu_quota_audit.jsonl")).unwrap_or_default();
    assert!(audit.contains("enqueue_over_quota"));
    // Within the quota the same op enqueues fine.
    let (status, ok) = post(
        &app,
        "/api/wgpu/task/enqueue",
        serde_json::json!({"op": "matvec", "iters": 64}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(ok["ok"], true);
    assert!(ok["quota"]["total_flops"].as_u64().unwrap() > 0);
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

#[test]
fn group_results_returns_ordered_verified_partials() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(format!("wgpu_group_results_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("data dir");
    let row = |shard: u64, peer: u64| {
        serde_json::json!({
            "group_id": "g-test", "status": "done", "shard_index": shard,
            "shards": 2, "peer_id": peer, "task_id": format!("mt-test-{shard}"),
            "row_start": shard * 2, "row_end": shard * 2 + 2,
            "max_err_model": 1e-18, "elapsed_ms": 12.5,
            "partial": [shard as f64, 1.0], "x": vec![7.0; 1024],
        })
    };
    let text = format!("{}\n{}\n", row(1, 39), row(0, 37));
    std::fs::write(dir.join("wgpu_tasks.jsonl"), text).expect("write tasks");
    let out = gsv::boxes::wgpu::group_results(&dir, "g-test").expect("group results");
    assert_eq!(out["shards_done"], 2);
    assert_eq!(out["results"][0]["peer_id"], 37);
    assert_eq!(out["results"][1]["peer_id"], 39);
    assert_eq!(out["results"][0]["partial"][0], 0.0);
    assert!(
        out["results"][0].get("x").is_none(),
        "no huge x on the wire"
    );
    assert!(gsv::boxes::wgpu::group_results(&dir, "bad../id").is_err());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn page_forces_wasm_for_phone_weights_matvec() {
    let page = gsv::boxes::wgpu::page();
    assert!(
        page.contains("WORKER&&WORKER.phone"),
        "phone WebGPU path must bypass to WASM SIMD"
    );
    assert!(
        page.contains("runWeightsMatvecJs"),
        "phone must use golden-tested JS dequant fallback"
    );
    assert!(
        page.contains("const cached=phone?null:await idbGet(c)"),
        "phone peers must bypass unverifiable IndexedDB chunk cache"
    );
    assert!(
        page.contains("clearWasmWeightCache"),
        "WASM cache guard must stay present"
    );
}
