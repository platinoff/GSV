//! Shared integration-test harness (P6.1 phase 4 — dedup).
//!
//! Contract tests used to carry identical local copies of `kit_root()`,
//! `fn app()` and `async fn get(...)`. They now live here; include this
//! module from each contract test file with `mod common;`.
//!
//! Semantics are unchanged: `app()` builds the same broadcast channel (64)
//! plus `AppState::new(Some(kit_root()), None, tx)` fixture the per-file
//! versions used. `router_app()` is the router-only variant. `get()` is a
//! oneshot GET returning `(StatusCode, Value)` with the body parsed as
//! JSON (`Value::Null` when the body is not JSON).

#![allow(dead_code)]

use std::path::PathBuf;

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use gsv::server::router;
use gsv::AppState;
use serde_json::Value;
use tokio::sync::broadcast;
use tower::ServiceExt;

/// Repo (kit) root for tests that operate on the real tree.
pub fn kit_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Fresh app + state for one test.
pub fn app() -> (axum::Router, AppState) {
    let (tx, _rx) = broadcast::channel(64);
    let state = AppState::new(Some(kit_root()), None, tx);
    (router(state.clone()), state)
}

/// Router-only variant of [`app`].
pub fn router_app() -> axum::Router {
    app().0
}

/// Oneshot GET against the router; returns status + parsed JSON body.
pub async fn get(app: &axum::Router, path: &str) -> (StatusCode, Value) {
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
