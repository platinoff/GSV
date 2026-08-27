//! GSV preview endpoint — integration tests (HTTP contract).
//!
//! Uses `tower::ServiceExt::oneshot` against the axum router (no port binding).
//! Tests `GET /api/preview?file=<rel_path>` response shape and error paths.

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
async fn preview_valid_rs_file_returns_ok() {
    let (app, _state) = app();
    let (status, _json) = get(&app, "/api/preview?file=src/lib.rs").await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn preview_has_expected_fields() {
    let (app, _state) = app();
    let (status, json) = get(&app, "/api/preview?file=src/lib.rs").await;
    assert_eq!(status, StatusCode::OK);
    assert!(json["html"].is_string(), "missing html: {json}");
    assert!(json["size"].is_number(), "missing size: {json}");
    assert!(json["extension"].is_string(), "missing extension: {json}");
    assert!(json["path"].is_string(), "missing path: {json}");
}

#[tokio::test]
async fn preview_html_is_not_empty() {
    let (app, _state) = app();
    let (_, json) = get(&app, "/api/preview?file=src/lib.rs").await;
    let html = json["html"].as_str().expect("html string");
    assert!(!html.is_empty(), "html must be non-empty for a valid file");
}

#[tokio::test]
async fn preview_html_contains_pre_tag() {
    let (app, _state) = app();
    let (_, json) = get(&app, "/api/preview?file=src/lib.rs").await;
    let html = json["html"].as_str().expect("html string");
    assert!(html.contains("<pre"), "html must contain <pre tag: {html}");
}

#[tokio::test]
async fn preview_path_matches_query() {
    let (app, _state) = app();
    let (_, json) = get(&app, "/api/preview?file=src/lib.rs").await;
    assert_eq!(json["path"], "src/lib.rs");
}

#[tokio::test]
async fn preview_missing_file_returns_404() {
    let (app, _state) = app();
    let (status, _json) = get(&app, "/api/preview?file=nonexistent.rs").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn preview_missing_file_has_error() {
    let (app, _state) = app();
    let (status, json) = get(&app, "/api/preview?file=nonexistent.rs").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(json["ok"], false);
    assert!(json["error"].is_string(), "missing error field: {json}");
}

#[tokio::test]
async fn preview_traversal_returns_error() {
    let (app, _state) = app();
    let (status, json) = get(&app, "/api/preview?file=../../etc/passwd").await;
    assert!(status.is_client_error(), "expected 4xx, got {status}");
    assert_eq!(json["ok"], false);
    assert!(json["error"].is_string());
}

#[tokio::test]
async fn preview_no_file_param_returns_error() {
    let (app, _state) = app();
    let (status, _json) = get(&app, "/api/preview").await;
    assert!(
        status.is_client_error(),
        "expected 4xx for missing file param, got {status}"
    );
}
