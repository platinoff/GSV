//! Local-bind / CSRF / data-file / HTTP header contracts (bands 133–134).
//!
//! Defensive checks only: loopback Origin is accepted, cross-site POST is
//! forbidden, `/data/{file}` stays on the allowlist, Omni GET has no `api_key`,
//! responses carry CSP / nosniff / no-store, oversized POST is 413.

use std::path::PathBuf;

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use gsv::server::router;
use gsv::AppState;
use serde_json::Value;
use tokio::sync::broadcast;
use tower::ServiceExt;

fn app() -> axum::Router {
    let (tx, _rx) = broadcast::channel(64);
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let state = AppState::new(Some(repo_root), None, tx);
    router(state)
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

async fn post_with(
    app: &axum::Router,
    path: &str,
    body: Value,
    origin: Option<&str>,
    sec_fetch_site: Option<&str>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .uri(path)
        .method(Method::POST)
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(o) = origin {
        builder = builder.header(header::ORIGIN, o);
    }
    if let Some(s) = sec_fetch_site {
        builder = builder.header("sec-fetch-site", s);
    }
    let res = app
        .clone()
        .oneshot(builder.body(Body::from(body.to_string())).expect("request"))
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
async fn post_from_loopback_origin_is_allowed() {
    let app = app();
    let body = serde_json::json!({ "command": "echo gsv-local" });
    let (status, json) = post_with(
        &app,
        "/api/terminal",
        body,
        Some("http://127.0.0.1:9999"),
        Some("same-origin"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["allowed"], true);
}

#[tokio::test]
async fn post_from_non_local_origin_is_forbidden() {
    let app = app();
    let body = serde_json::json!({ "command": "echo gsv-remote" });
    let (status, json) = post_with(
        &app,
        "/api/terminal",
        body,
        Some("https://example.com"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(json["ok"], false);
    assert!(json["error"]
        .as_str()
        .unwrap_or_default()
        .contains("non-local origin"));
}

#[tokio::test]
async fn post_cross_site_fetch_is_forbidden() {
    let app = app();
    let body = serde_json::json!({ "command": "echo gsv-csrf" });
    let (status, json) = post_with(&app, "/api/terminal", body, None, Some("cross-site")).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(json["ok"], false);
    assert!(json["error"]
        .as_str()
        .unwrap_or_default()
        .contains("cross-site"));
}

#[tokio::test]
async fn data_file_unknown_and_dotdot_are_rejected() {
    let app = app();
    let (status, json) = get(&app, "/data/does-not-exist.json").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(json["ok"], false);
    let (status2, json2) = get(&app, "/data/..").await;
    assert_eq!(status2, StatusCode::BAD_REQUEST);
    assert_eq!(json2["ok"], false);
    let (status3, json3) = get(&app, "/data/omni.toml").await;
    assert_eq!(status3, StatusCode::BAD_REQUEST);
    assert_eq!(json3["ok"], false);
}

#[tokio::test]
async fn omni_config_get_has_no_api_key_field() {
    let app = app();
    let (status, json) = get(&app, "/api/omni/config").await;
    assert_eq!(status, StatusCode::OK);
    let providers = json["provider"].as_object().expect("provider");
    for (id, row) in providers {
        assert!(
            row.get("api_key").is_none(),
            "provider {id} leaked api_key field"
        );
        assert!(row["key_set"].is_boolean(), "provider {id} missing key_set");
    }
}

fn header_str(res: &axum::http::Response<Body>, name: &str) -> String {
    res.headers()
        .get(name)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string()
}

#[tokio::test]
async fn health_response_carries_security_headers() {
    let app = app();
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/health")
                .method(Method::GET)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(header_str(&res, "x-content-type-options"), "nosniff");
    assert_eq!(header_str(&res, "x-frame-options"), "DENY");
    assert_eq!(header_str(&res, "referrer-policy"), "no-referrer");
    assert_eq!(header_str(&res, "cache-control"), "no-store");
    let csp = header_str(&res, "content-security-policy");
    assert!(csp.contains("default-src 'self'"), "csp={csp}");
    assert!(csp.contains("frame-ancestors 'none'"), "csp={csp}");
    assert_eq!(
        header_str(&res, "cross-origin-opener-policy"),
        "same-origin"
    );
}

#[tokio::test]
async fn forbidden_post_also_carries_security_headers() {
    let app = app();
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/terminal")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::ORIGIN, "https://example.com")
                .body(Body::from(r#"{"command":"echo gsv-hdr"}"#))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    assert_eq!(header_str(&res, "x-content-type-options"), "nosniff");
    assert_eq!(header_str(&res, "cache-control"), "no-store");
}

#[tokio::test]
async fn oversized_post_is_payload_too_large() {
    let app = app();
    let blob = "x".repeat(gsv::security::MAX_BODY_BYTES);
    let payload = format!(r#"{{"command":"{blob}"}}"#);
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/terminal")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::ORIGIN, "http://127.0.0.1:9999")
                .body(Body::from(payload))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(res.status(), StatusCode::PAYLOAD_TOO_LARGE);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .expect("body");
    let json: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    assert_eq!(json["ok"], false);
    assert!(json["error"]
        .as_str()
        .unwrap_or_default()
        .contains("too large"));
}
