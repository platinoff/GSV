//! Hub edge-proxy contracts — token + allowlist + rate-limit on `/api/edge`.
//!
//! Status GET is open and redacted. Forwards require `X-Gsv-Edge-Token` (or
//! Bearer). Denied paths (login/vm) stay 404. No live poolAI is required:
//! a loopback stub plays upstream; a discarded port is a 502.

use std::path::PathBuf;

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use gsv::boxes::edge;
use gsv::boxes::grid::GridBox;
use gsv::boxes::settings::{self, EdgeSettings, SettingsFile};
use gsv::server::router;
use gsv::AppState;
use serde_json::{json, Value};
use tokio::sync::broadcast;
use tower::ServiceExt;

fn temp_data(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("gsv-edge-http-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("mkdir");
    dir
}

fn dead_base() -> String {
    let l = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = l.local_addr().expect("addr").port();
    drop(l);
    format!("http://127.0.0.1:{port}/api/v1")
}

fn app_with(dir: PathBuf, base: &str) -> axum::Router {
    let (tx, _rx) = broadcast::channel(32);
    let mut state = AppState::new(
        Some(PathBuf::from(env!("CARGO_MANIFEST_DIR"))),
        Some(dir),
        tx,
    );
    state.grid = std::sync::Arc::new(GridBox::with_base(&state.data_dir, base));
    router(state)
}

async fn get_json(app: &axum::Router, path: &str) -> (StatusCode, Value) {
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(path)
                .method(Method::GET)
                .body(Body::empty())
                .expect("req"),
        )
        .await
        .expect("resp");
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .expect("body");
    (status, serde_json::from_slice(&bytes).unwrap_or_default())
}

async fn call(
    app: &axum::Router,
    method: Method,
    path: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut b = Request::builder().uri(path).method(method);
    if let Some(t) = token {
        b = b.header(edge::TOKEN_HEADER, t);
    }
    if body.is_some() {
        b = b.header(header::CONTENT_TYPE, "application/json");
    }
    let res = app
        .clone()
        .oneshot(
            b.body(Body::from(body.map(|v| v.to_string()).unwrap_or_default()))
                .expect("req"),
        )
        .await
        .expect("resp");
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .expect("body");
    (status, serde_json::from_slice(&bytes).unwrap_or_default())
}

async fn stub_upstream() -> String {
    async fn health() -> Json<Value> {
        Json(json!({ "ok": true, "nodes": { "edge-pc-01": {} }, "token": "leak" }))
    }
    async fn jobs(Json(_): Json<Value>) -> Json<Value> {
        Json(json!({ "ok": true, "password": "nope" }))
    }
    let app = Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/v1/jobs", post(jobs));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve");
    });
    format!("http://127.0.0.1:{}/api/v1", addr.port())
}

#[tokio::test]
async fn status_is_open_and_redacted() {
    let dir = temp_data("status");
    let file = SettingsFile {
        edge: EdgeSettings {
            token: "file-edge-secret".into(),
        },
        ..Default::default()
    };
    settings::save(&dir, &file).expect("save");
    let app = app_with(dir, &dead_base());
    let (st, json) = get_json(&app, "/api/edge").await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(json["ok"], true);
    assert_eq!(json["token_set"], true);
    assert_eq!(json["source"], "file");
    assert_eq!(json["service"]["kind"], "edge_token");
    assert_eq!(json["service"]["login_blocked"], true);
    assert_eq!(json["service"]["poolai_admin_default"], false);
    let raw = json.to_string();
    assert!(!raw.contains("file-edge-secret"), "{raw}");
    assert!(!raw.contains("admin123"), "{raw}");
}

#[tokio::test]
async fn health_lists_edge_proxy_without_secret() {
    let dir = temp_data("health");
    let app = app_with(dir, &dead_base());
    let (st, json) = get_json(&app, "/api/health").await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(json["edge_proxy"]["ok"], true);
    assert!(json["edge_proxy"]["allowlist"].as_array().is_some());
    assert!(!json.to_string().contains("bot_token"));
}

#[tokio::test]
async fn unset_token_refuses_forward() {
    let dir = temp_data("unset");
    let app = app_with(dir, &dead_base());
    let (st, json) = call(&app, Method::GET, "/api/edge/health", Some("x"), None).await;
    assert_eq!(st, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(json["ok"], false);
    assert_eq!(json["error"], "edge token unset");
}

#[tokio::test]
async fn deny_login_and_vm() {
    let dir = temp_data("deny");
    let file = SettingsFile {
        edge: EdgeSettings {
            token: "tok-deny-xxxxxxxx".into(),
        },
        ..Default::default()
    };
    settings::save(&dir, &file).expect("save");
    let app = app_with(dir, &dead_base());
    for path in ["/api/edge/login", "/api/edge/vm/instances"] {
        let (st, json) = call(&app, Method::GET, path, Some("tok-deny-xxxxxxxx"), None).await;
        assert_eq!(st, StatusCode::NOT_FOUND, "{path}");
        assert_eq!(json["error"], "path not on edge allowlist", "{path} {json}");
    }
}

#[tokio::test]
async fn wrong_token_is_401() {
    let dir = temp_data("401");
    let file = SettingsFile {
        edge: EdgeSettings {
            token: "expected-token-aa".into(),
        },
        ..Default::default()
    };
    settings::save(&dir, &file).expect("save");
    let app = app_with(dir, &dead_base());
    let (st, json) = call(&app, Method::GET, "/api/edge/health", Some("nope"), None).await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
    assert_eq!(json["error"], "edge token required");
}

#[tokio::test]
async fn allowlisted_get_and_post_redact_upstream() {
    let base = stub_upstream().await;
    let dir = temp_data("fwd");
    let file = SettingsFile {
        edge: EdgeSettings {
            token: "fwd-token-bbbbbbbb".into(),
        },
        ..Default::default()
    };
    settings::save(&dir, &file).expect("save");
    let app = app_with(dir, &base);
    let (st, json) = call(
        &app,
        Method::GET,
        "/api/edge/health",
        Some("fwd-token-bbbbbbbb"),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{json}");
    assert!(json["nodes"].get("edge-pc-01").is_some(), "{json}");
    assert!(json.get("token").is_none(), "{json}");
    let (st, json) = call(
        &app,
        Method::POST,
        "/api/edge/jobs",
        Some("fwd-token-bbbbbbbb"),
        Some(json!({"task":"x"})),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{json}");
    assert_eq!(json["ok"], true);
    assert!(json.get("password").is_none(), "{json}");
}

#[tokio::test]
async fn rate_limit_returns_429() {
    edge::clear_rate_limit();
    let dir = temp_data("429");
    let file = SettingsFile {
        edge: EdgeSettings {
            token: "rate-token-cccccccc".into(),
        },
        ..Default::default()
    };
    settings::save(&dir, &file).expect("save");
    let app = app_with(dir, &dead_base());
    let token = "rate-token-cccccccc";
    for _ in 0..edge::RATE_CAP {
        assert_eq!(edge::gate("GET", "health", token, token).unwrap(), "health");
    }
    let (st, json) = call(&app, Method::GET, "/api/edge/health", Some(token), None).await;
    assert_eq!(st, StatusCode::TOO_MANY_REQUESTS, "{json}");
    assert_eq!(json["error"], "rate limited");
    edge::clear_rate_limit();
}
