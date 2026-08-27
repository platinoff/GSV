//! GSV IDE API contracts — Rust integration tests (HTTP/JSON).
//!
//! Uses `tower::ServiceExt::oneshot` against the axum router (no port binding).
//! Scope: IDE session listing and selection endpoints return expected JSON shapes.

use std::path::PathBuf;

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use gsv::server::router;
use gsv::AppState;
use serde_json::Value;
use tokio::sync::broadcast;
use tower::ServiceExt;

/// Build a fresh app state + router for one test.
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
    let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, json)
}

#[tokio::test]
async fn ide_sessions_returns_ok() {
    let (app, _state) = app();
    let (status, _json) = get(&app, "/api/ide/sessions").await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn ide_sessions_has_expected_fields() {
    let (app, _state) = app();
    let (status, json) = get(&app, "/api/ide/sessions").await;
    assert_eq!(status, StatusCode::OK);
    assert!(json.get("sessions").is_some(), "missing sessions field");
    assert!(json.get("selection").is_some(), "missing selection field");
    assert!(json.get("preview").is_some(), "missing preview field");
    assert!(
        json.get("generated_at").is_some(),
        "missing generated_at field"
    );
}

#[tokio::test]
async fn ide_sessions_sessions_is_array() {
    let (app, _state) = app();
    let (_status, json) = get(&app, "/api/ide/sessions").await;
    assert!(
        json["sessions"].is_array(),
        "sessions must be an array: {json}"
    );
}

#[tokio::test]
async fn ide_sessions_preview_is_array() {
    let (app, _state) = app();
    let (_status, json) = get(&app, "/api/ide/sessions").await;
    assert!(
        json["preview"].is_array(),
        "preview must be an array: {json}"
    );
}

#[tokio::test]
async fn ide_select_returns_ok() {
    let (app, _state) = app();
    let body = serde_json::json!({ "tool": "opencode", "session": "some/id" });
    let (status, json) = post(&app, "/api/ide/select", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["ok"], true);
}

#[tokio::test]
async fn ide_select_sets_selection() {
    let (app, _state) = app();
    let body = serde_json::json!({ "tool": "opencode", "session": "some/id" });
    let (_status, json) = post(&app, "/api/ide/select", body).await;
    let sel = json["selection"].as_object().expect("selection object");
    assert_eq!(sel.get("tool").and_then(|v| v.as_str()), Some("opencode"));
    assert_eq!(sel.get("session").and_then(|v| v.as_str()), Some("some/id"));
}

#[tokio::test]
async fn ide_sessions_generated_at_is_string() {
    let (app, _state) = app();
    let (_status, json) = get(&app, "/api/ide/sessions").await;
    let ts = json["generated_at"]
        .as_str()
        .expect("generated_at must be a string");
    assert!(!ts.is_empty(), "generated_at must not be empty");
}

#[tokio::test]
async fn ide_select_empty_body_returns_422() {
    let (app, _state) = app();
    let (status, _json) = post(&app, "/api/ide/select", serde_json::json!({})).await;
    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "missing tool/session fields must produce 422, not 500"
    );
}
