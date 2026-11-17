//! GSV toolchain endpoint contracts — integration tests (HTTP/200 + JSON shapes).
//!
//! Uses `tower::ServiceExt::oneshot` against the axum router (no port binding).
//! Scope: toolchain endpoints return expected status codes + JSON shapes.

mod common;
use common::{app, get};

use axum::http::StatusCode;

/// Build a fresh app state + router for one test.
#[tokio::test]
async fn toolchain_returns_ok() {
    let (app, _state) = app();
    let (status, _json) = get(&app, "/api/toolchain").await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn toolchain_has_expected_fields() {
    let (app, _state) = app();
    let (status, json) = get(&app, "/api/toolchain").await;
    assert_eq!(status, StatusCode::OK);
    assert!(json["entries"].is_array(), "missing entries array");
    assert!(
        json["generated_at"].as_str().is_some(),
        "missing generated_at string"
    );
}

#[tokio::test]
async fn toolchain_entries_have_shape() {
    let (app, _state) = app();
    let (_status, json) = get(&app, "/api/toolchain").await;
    let entries = json["entries"].as_array().expect("entries");
    assert!(!entries.is_empty(), "toolchain entries must not be empty");
    for entry in entries {
        assert!(
            entry["tool"].as_str().is_some(),
            "entry missing tool: {entry}"
        );
        assert!(
            entry["version"].as_str().is_some(),
            "entry missing version: {entry}"
        );
        assert!(
            entry["source"].as_str().is_some(),
            "entry missing source: {entry}"
        );
    }
}

#[tokio::test]
async fn toolchain_rustc_returns_ok() {
    let (app, _state) = app();
    let (status, json) = get(&app, "/api/toolchain/rustc").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["ok"], true);
    assert_eq!(json["tool"], "rustc");
    assert!(json["entry"].is_object(), "missing entry object: {json}");
}

#[tokio::test]
async fn toolchain_cargo_returns_ok() {
    let (app, _state) = app();
    let (status, json) = get(&app, "/api/toolchain/cargo").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["ok"], true);
    assert_eq!(json["tool"], "cargo");
    assert!(json["entry"].is_object(), "missing entry object: {json}");
}

#[tokio::test]
async fn toolchain_clippy_returns_ok() {
    let (app, _state) = app();
    let (status, json) = get(&app, "/api/toolchain/clippy").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["ok"], true);
    assert_eq!(json["tool"], "clippy-driver");
    assert!(json["entry"].is_object(), "missing entry object: {json}");
}

#[tokio::test]
async fn toolchain_detailed_returns_ok() {
    let (app, _state) = app();
    let (status, json) = get(&app, "/api/toolchain/detailed").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["ok"], true);
}

#[tokio::test]
async fn toolchain_detailed_has_entries() {
    let (app, _state) = app();
    let (_status, json) = get(&app, "/api/toolchain/detailed").await;
    let tc = &json["toolchain"];
    assert!(
        tc.is_object(),
        "missing toolchain object in detailed response: {json}"
    );
    assert!(
        tc["entries"].is_array(),
        "missing toolchain.entries array: {json}"
    );
    assert!(
        tc["generated_at"].as_str().is_some(),
        "missing toolchain.generated_at: {json}"
    );
}

#[tokio::test]
async fn toolchain_generated_at_is_string() {
    let (app, _state) = app();
    let (_status, json) = get(&app, "/api/toolchain").await;
    let ga = json["generated_at"]
        .as_str()
        .expect("generated_at must be a string");
    assert!(!ga.is_empty(), "generated_at must not be empty");
}
