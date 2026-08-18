//! GSV Service Worker shell-cache contracts (band 148).
//!
//! Static UI assets are precached so `/` still opens when the live server is
//! down. The SW script is Rust-rendered (`GET /sw.js`); API / SSE / MCP stay
//! network-only.

use std::path::PathBuf;

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
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

async fn get(app: &axum::Router, uri: &str) -> (StatusCode, axum::http::HeaderMap, Vec<u8>) {
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(uri)
                .method(Method::GET)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    let status = res.status();
    let headers = res.headers().clone();
    let body = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .expect("body")
        .to_vec();
    (status, headers, body)
}

fn header_str(headers: &axum::http::HeaderMap, name: &str) -> String {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string()
}

#[tokio::test]
async fn sw_js_is_javascript_with_scope_header() {
    let app = app();
    let (status, headers, body) = get(&app, "/sw.js").await;
    assert_eq!(status, StatusCode::OK);
    let ct = header_str(&headers, "content-type");
    assert!(
        ct.contains("javascript"),
        "content-type must be javascript, got {ct}"
    );
    assert_eq!(header_str(&headers, "service-worker-allowed"), "/");
    let js = String::from_utf8(body).expect("utf8");
    assert!(
        js.contains("caches.open"),
        "install must open Cache API: {js}"
    );
    assert!(js.contains("gsv-shell"), "named shell cache: {js}");
    assert!(js.contains("\"/\""), "precache includes / : {js}");
    assert!(
        js.contains("/api/ui/load-palette"),
        "precache live palette: {js}"
    );
    assert!(
        js.contains("pathname==='/events'") || js.contains("pathname===\"/events\""),
        "must skip SSE /events: {js}"
    );
    assert!(
        js.contains("pathname==='/mcp'") || js.contains("pathname===\"/mcp\""),
        "must skip /mcp: {js}"
    );
    assert!(
        js.contains("req.method!=='GET'") || js.contains("req.method!==\"GET\""),
        "must ignore non-GET: {js}"
    );
}

#[tokio::test]
async fn api_sw_lists_precache_urls() {
    let app = app();
    let (status, _headers, body) = get(&app, "/api/sw").await;
    assert_eq!(status, StatusCode::OK);
    let json: Value = serde_json::from_slice(&body).expect("json");
    assert_eq!(json["ok"], true);
    assert_eq!(json["script"], "/sw.js");
    let urls = json["urls"].as_array().expect("urls");
    let as_str: Vec<&str> = urls.iter().filter_map(|v| v.as_str()).collect();
    assert!(as_str.contains(&"/"), "shell /: {as_str:?}");
    assert!(
        as_str.contains(&"/api/ui/load-palette"),
        "palette: {as_str:?}"
    );
    assert!(as_str.contains(&"/api/ui/load-theme"), "theme: {as_str:?}");
    assert!(
        !as_str.iter().any(|u| u.contains("/mcp") || *u == "/events"),
        "must not precache mcp/events: {as_str:?}"
    );
}

#[tokio::test]
async fn csp_allows_same_origin_service_worker() {
    let app = app();
    let (_status, headers, _body) = get(&app, "/api/health").await;
    let csp = header_str(&headers, "content-security-policy");
    assert!(
        csp.contains("worker-src 'self'"),
        "CSP must allow same-origin SW: {csp}"
    );
}

#[tokio::test]
async fn index_registers_sw_js() {
    let app = app();
    let (status, _headers, body) = get(&app, "/").await;
    assert_eq!(status, StatusCode::OK);
    let html = String::from_utf8(body).expect("utf8");
    assert!(
        html.contains("serviceWorker.register(\"/sw.js\")")
            || html.contains("serviceWorker.register('/sw.js')"),
        "thin glue must register /sw.js: missing in index"
    );
}

#[tokio::test]
async fn sw_js_does_not_claim_cross_origin() {
    let app = app();
    let (_status, _headers, body) = get(&app, "/sw.js").await;
    let js = String::from_utf8(body).expect("utf8");
    assert!(
        js.contains("u.origin!==self.location.origin")
            || js.contains("u.origin !== self.location.origin"),
        "must ignore cross-origin fetch: {js}"
    );
}
