//! Settings / Godfather secret store contracts (band 166).
//!
//! Redacted GET/POST, env override, CSRF on POST, MCP read without `bot_token`,
//! `GET /data/gsv_settings.json` not served.

use std::path::PathBuf;

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use gsv::boxes::settings;
use gsv::boxes::ui::{render_card, CARD_NAMES};
use gsv::mcp;
use gsv::server::router;
use gsv::AppState;
use serde_json::{json, Value};
use tokio::sync::broadcast;
use tower::ServiceExt;

fn kit_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn temp_data(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("gsv-settings-http-{tag}-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn app_with_data(data: PathBuf) -> axum::Router {
    let (tx, _rx) = broadcast::channel(32);
    let state = AppState::new(Some(kit_root()), Some(data), tx);
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

async fn post_json(
    app: &axum::Router,
    path: &str,
    body: Value,
    origin: Option<&str>,
    site: Option<&str>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .uri(path)
        .method(Method::POST)
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(o) = origin {
        builder = builder.header(header::ORIGIN, o);
    }
    if let Some(s) = site {
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
async fn get_settings_missing_file_empty_ok() {
    let data = temp_data("get-empty");
    let app = app_with_data(data);
    let (status, json) = get_json(&app, "/api/settings").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["ok"], true);
    assert_eq!(json["token_set"], false);
    assert!(!settings::json_has_bot_token(&json), "{json}");
}

#[tokio::test]
async fn post_token_round_trip_redacted() {
    let data = temp_data("post");
    let app = app_with_data(data.clone());
    let (status, json) = post_json(
        &app,
        "/api/settings",
        json!({
            "godfather": {
                "channel_id": "-1001",
                "bot_token": "123:http-secret",
                "allowed_user_ids": ["9"]
            },
            "workflows": { "enabled": ["drain"] },
            "nope": true
        }),
        Some("http://127.0.0.1:9999"),
        Some("same-origin"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert_eq!(json["ok"], true);
    assert_eq!(json["token_set"], true);
    assert_eq!(json["godfather"]["channel_id"], "-1001");
    assert!(!settings::json_has_bot_token(&json), "{json}");
    let raw = serde_json::to_string(&json).expect("json");
    assert!(!raw.contains("123:http-secret"), "{raw}");
    let disk = std::fs::read_to_string(settings::store_path(&data)).expect("disk");
    assert!(disk.contains("123:http-secret"));
    let (gstatus, get) = get_json(&app, "/api/settings").await;
    assert_eq!(gstatus, StatusCode::OK);
    assert!(!settings::json_has_bot_token(&get), "{get}");
}

#[tokio::test]
async fn post_cross_site_is_forbidden() {
    let app = app_with_data(temp_data("csrf"));
    let (status, json) = post_json(
        &app,
        "/api/settings",
        json!({ "godfather": { "bot_token": "nope" } }),
        Some("https://example.com"),
        Some("cross-site"),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(json["ok"], false);
    assert!(json["error"]
        .as_str()
        .unwrap_or_default()
        .contains("cross-site"));
}

#[tokio::test]
async fn data_file_does_not_serve_settings() {
    let app = app_with_data(temp_data("data"));
    let (status, json) = get_json(&app, "/data/gsv_settings.json").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(json["ok"], false);
}

#[tokio::test]
async fn mcp_settings_has_no_bot_token() {
    let data = temp_data("mcp");
    settings::save(
        &data,
        &settings::SettingsFile {
            godfather: settings::Godfather {
                channel_id: "ch".into(),
                allowed_user_ids: vec![],
                bot_token: "mcp-secret-token".into(),
                poll: false,
                role: String::new(),
            },
            ..Default::default()
        },
    )
    .expect("save");
    let app = app_with_data(data);
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/mcp")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "jsonrpc": "2.0",
                        "id": 1,
                        "method": "tools/call",
                        "params": { "name": "gsv_settings", "arguments": {} }
                    })
                    .to_string(),
                ))
                .expect("req"),
        )
        .await
        .expect("resp");
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .expect("body");
    let json: Value = serde_json::from_slice(&bytes).expect("json");
    assert_eq!(json["result"]["isError"], false, "{json}");
    let text = json["result"]["content"][0]["text"].as_str().unwrap_or("");
    assert!(text.contains("token_set"), "{text}");
    assert!(!text.contains("bot_token"), "{text}");
    assert!(!text.contains("mcp-secret-token"), "{text}");
    assert!(mcp::tool_names().contains(&"gsv_settings"));
}

#[test]
fn card_settings_in_registry() {
    assert!(CARD_NAMES.contains(&"settings"));
    let empty = render_card(
        "settings",
        &json!({
            "ok": true,
            "token_set": false,
            "source": "none",
            "godfather": { "channel_id": "", "allowed_user_ids": [] },
            "workflows": { "enabled": [] }
        }),
    )
    .expect("empty");
    assert!(empty.contains("settings — no data"), "{empty}");
    let err = render_card("settings", &json!({ "ok": false, "error": "down" })).expect("err");
    assert!(err.contains("<span class='err'>down</span>"), "{err}");
}
