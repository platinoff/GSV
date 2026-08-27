//! GSV SLI endpoint contracts — Rust integration tests (HTTP/JSON shape).
//!
//! Uses `tower::ServiceExt::oneshot` against the axum router (no port binding).
//! Scope: `GET /api/sli` returns the expected JSON structure, catalog shape,
//! and derived invariants.

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
async fn sli_returns_ok() {
    let (app, _state) = app();
    let (status, _json) = get(&app, "/api/sli").await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn sli_has_expected_fields() {
    let (app, _state) = app();
    let (_status, json) = get(&app, "/api/sli").await;
    assert!(json.get("catalog").is_some(), "missing `catalog`");
    assert!(json.get("generated_at").is_some(), "missing `generated_at`");
}

#[tokio::test]
async fn sli_catalog_has_entries_and_roots() {
    let (app, _state) = app();
    let (_status, json) = get(&app, "/api/sli").await;
    let catalog = &json["catalog"];
    assert!(
        catalog["entries"].is_array(),
        "catalog.entries must be array"
    );
    assert!(catalog["roots"].is_array(), "catalog.roots must be array");
    assert!(
        catalog["used_count"].is_number(),
        "catalog.used_count must be number"
    );
    assert!(
        catalog["unused_count"].is_number(),
        "catalog.unused_count must be number"
    );
}

#[tokio::test]
async fn sli_entries_have_expected_shape() {
    let (app, _state) = app();
    let (_status, json) = get(&app, "/api/sli").await;
    let entries = json["catalog"]["entries"]
        .as_array()
        .expect("catalog.entries");
    assert!(!entries.is_empty(), "catalog must have at least one entry");
    for entry in entries {
        assert!(
            entry["name"].as_str().is_some(),
            "entry missing `name`: {entry}"
        );
        assert!(
            entry["path"].as_str().is_some(),
            "entry missing `path`: {entry}"
        );
        assert!(
            entry["kind"].as_str().is_some(),
            "entry missing `kind`: {entry}"
        );
        assert!(
            entry["description"].as_str().is_some(),
            "entry missing `description`: {entry}"
        );
        assert!(
            entry["used"].is_boolean(),
            "entry missing `used` bool: {entry}"
        );
        assert!(
            entry["example"].as_str().is_some(),
            "entry missing `example`: {entry}"
        );
    }
}

#[tokio::test]
async fn sli_generated_at_is_string() {
    let (app, _state) = app();
    let (_status, json) = get(&app, "/api/sli").await;
    let ts = json["generated_at"]
        .as_str()
        .expect("generated_at must be string");
    assert!(!ts.is_empty(), "generated_at must be non-empty");
}

#[tokio::test]
async fn sli_used_count_plus_unused_count_matches_entries() {
    let (app, _state) = app();
    let (_status, json) = get(&app, "/api/sli").await;
    let entries = json["catalog"]["entries"]
        .as_array()
        .expect("catalog.entries");
    let used = json["catalog"]["used_count"].as_u64().expect("used_count") as usize;
    let unused = json["catalog"]["unused_count"]
        .as_u64()
        .expect("unused_count") as usize;
    assert_eq!(
        used + unused,
        entries.len(),
        "used_count ({used}) + unused_count ({unused}) must equal entries.len ({})",
        entries.len()
    );
}

#[tokio::test]
async fn sli_roots_are_strings() {
    let (app, _state) = app();
    let (_status, json) = get(&app, "/api/sli").await;
    let roots = json["catalog"]["roots"].as_array().expect("catalog.roots");
    assert!(!roots.is_empty(), "roots must have at least one entry");
    for root in roots {
        assert!(
            root.as_str().is_some(),
            "each root must be a string: {root}"
        );
    }
}
