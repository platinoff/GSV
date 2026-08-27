//! GSV hooks endpoints — Rust integration tests (HTTP/JSON shape).
//!
//! Uses `tower::ServiceExt::oneshot` against the axum router (no port binding).
//! Scope: `/api/hooks/tests` and `/api/hooks/bench` return expected status codes,
//! JSON shapes, and invariant fields.

use std::path::PathBuf;

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use gsv::server::router;
use gsv::AppState;
use serde_json::Value;
use tokio::sync::broadcast;
use tower::ServiceExt;

fn app() -> (axum::Router, AppState) {
    let (tx, _rx) = broadcast::channel(64);
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let state = AppState::new(Some(repo_root), None, tx);
    (router(state.clone()), state)
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
    let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, json)
}

#[tokio::test]
async fn hooks_tests_returns_ok() {
    let (app, _state) = app();
    let (status, _json) = get(&app, "/api/hooks/tests").await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn hooks_tests_has_expected_fields() {
    let (app, _state) = app();
    let (_status, json) = get(&app, "/api/hooks/tests").await;
    assert!(json["test_bins"].is_array(), "test_bins must be array");
    assert!(
        json["diagnostics"].is_object() || json["diagnostics"].is_null(),
        "diagnostics must be object or null"
    );
    assert!(json["status"].is_string(), "status must be string");
}

#[tokio::test]
async fn hooks_tests_status_is_string() {
    let (app, _state) = app();
    let (_status, json) = get(&app, "/api/hooks/tests").await;
    let s = json["status"].as_str().expect("status string");
    assert!(
        s == "ready" || s == "no-artifacts",
        "status must be ready or no-artifacts, got: {s}"
    );
}

#[tokio::test]
async fn hooks_bench_returns_ok() {
    let (app, _state) = app();
    let (status, _json) = get(&app, "/api/hooks/bench").await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn hooks_bench_has_expected_fields() {
    let (app, _state) = app();
    let (_status, json) = get(&app, "/api/hooks/bench").await;
    assert!(
        json["criterion_dirs"].is_array(),
        "criterion_dirs must be array"
    );
    assert!(
        json["speed_index"].is_object() || json["speed_index"].is_null(),
        "speed_index must be object or null"
    );
    assert!(json["status"].is_string(), "status must be string");
}

#[tokio::test]
async fn hooks_bench_status_is_string() {
    let (app, _state) = app();
    let (_status, json) = get(&app, "/api/hooks/bench").await;
    let s = json["status"].as_str().expect("status string");
    assert!(
        s == "ready" || s == "no-artifacts",
        "status must be ready or no-artifacts, got: {s}"
    );
}

#[tokio::test]
async fn hooks_diagnostics_shape_when_present() {
    let (app, _state) = app();
    let (_status, json) = get(&app, "/api/hooks/tests").await;
    let diag = match json["diagnostics"].as_object() {
        Some(d) => d,
        None => return, // diagnostics absent — nothing to check
    };
    assert!(
        diag.get("warnings").and_then(Value::as_u64).is_some(),
        "diagnostics.warnings must be u64"
    );
    assert!(
        diag.get("errors").and_then(Value::as_u64).is_some(),
        "diagnostics.errors must be u64"
    );
    assert!(
        diag.get("ok").and_then(Value::as_bool).is_some(),
        "diagnostics.ok must be bool"
    );
}

#[tokio::test]
async fn hooks_speed_index_shape_when_present() {
    let (app, _state) = app();
    let (_status, json) = get(&app, "/api/hooks/bench").await;
    let si = match json["speed_index"].as_object() {
        Some(o) => o,
        None => return, // speed_index absent — nothing to check
    };
    assert!(
        si.get("test_ci_wall_secs")
            .and_then(Value::as_f64)
            .is_some(),
        "speed_index.test_ci_wall_secs must be f64"
    );
    assert!(
        si.get("test_ci_ok").and_then(Value::as_bool).is_some(),
        "speed_index.test_ci_ok must be bool"
    );
}
