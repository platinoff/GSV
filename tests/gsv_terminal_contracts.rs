//! GSV terminal endpoint contracts — Rust integration tests.
//!
//! Uses `tower::ServiceExt::oneshot` against the axum router (no port binding).
//! Tests the whitelist / sandbox logic of `POST /api/terminal`.

use std::path::PathBuf;

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
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
async fn terminal_echo_allowed() {
    let (app, _state) = app();
    let body = serde_json::json!({ "command": "echo hello" });
    let (status, json) = post(&app, "/api/terminal", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["allowed"], true);
    assert_eq!(json["exit_code"], 0);
    assert!(json["stdout"]
        .as_str()
        .unwrap_or_default()
        .contains("hello"));
}

#[tokio::test]
async fn terminal_response_has_expected_fields() {
    let (app, _state) = app();
    let body = serde_json::json!({ "command": "echo fields" });
    let (_status, json) = post(&app, "/api/terminal", body).await;
    assert!(json["command"].is_string(), "command field missing");
    assert!(json["allowed"].is_boolean(), "allowed field missing");
    assert!(json["stdout"].is_string(), "stdout field missing");
    assert!(json["stderr"].is_string(), "stderr field missing");
    assert!(
        json["exit_code"].is_number() || json["exit_code"].is_null(),
        "exit_code must be number or null"
    );
    assert!(json["duration_ms"].is_number(), "duration_ms field missing");
    assert_eq!(json["command"], "echo fields");
}

#[tokio::test]
async fn terminal_rm_not_allowed() {
    let (app, _state) = app();
    let body = serde_json::json!({ "command": "rm -rf /" });
    let (status, json) = post(&app, "/api/terminal", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["allowed"], false);
    assert!(json["stderr"]
        .as_str()
        .unwrap_or_default()
        .contains("not in whitelist"));
}

#[tokio::test]
async fn terminal_injection_blocked() {
    let (app, _state) = app();
    let body = serde_json::json!({ "command": "echo hello; rm -rf /" });
    let (status, json) = post(&app, "/api/terminal", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["allowed"], false);
    assert!(json["stderr"]
        .as_str()
        .unwrap_or_default()
        .contains("forbidden"));
}

#[tokio::test]
async fn terminal_pipe_blocked() {
    let (app, _state) = app();
    let body = serde_json::json!({ "command": "echo hello | cat" });
    let (status, json) = post(&app, "/api/terminal", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["allowed"], false);
    assert!(json["stderr"]
        .as_str()
        .unwrap_or_default()
        .contains("forbidden"));
}

#[tokio::test]
async fn terminal_dollar_blocked() {
    let (app, _state) = app();
    let body = serde_json::json!({ "command": "echo $HOME" });
    let (status, json) = post(&app, "/api/terminal", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["allowed"], false);
    assert!(json["stderr"]
        .as_str()
        .unwrap_or_default()
        .contains("forbidden"));
}

#[tokio::test]
async fn terminal_empty_command() {
    let (app, _state) = app();
    let body = serde_json::json!({ "command": "" });
    let (status, json) = post(&app, "/api/terminal", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["allowed"], false);
    assert!(json["stderr"]
        .as_str()
        .unwrap_or_default()
        .contains("empty command"));
}

#[tokio::test]
async fn terminal_cargo_version_allowed() {
    // A whitelisted `cargo` subcommand must execute. We deliberately use
    // `cargo --version` here: a nested `cargo test` from inside the harness
    // would wait on the same target-dir build lock and deadlock the suite.
    let (app, _state) = app();
    let body = serde_json::json!({ "command": "cargo --version" });
    let (status, json) = post(&app, "/api/terminal", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["allowed"], true);
    assert_eq!(json["exit_code"], 0);
    assert!(json["stdout"]
        .as_str()
        .unwrap_or_default()
        .contains("cargo"));
}

#[tokio::test]
async fn terminal_bash_not_allowed() {
    let (app, _state) = app();
    let body = serde_json::json!({ "command": "bash" });
    let (status, json) = post(&app, "/api/terminal", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["allowed"], false);
    assert!(json["stderr"]
        .as_str()
        .unwrap_or_default()
        .contains("not in whitelist"));
}
