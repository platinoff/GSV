//! GSV keep-live aggregation contracts (band 223).
//!
//! Aggregation only, no respawn: probes GSV + Telenetis + llama-rs (heartbeat
//! file) + OmniRoute and exposes the report on Galaxy, `/api/health`, `/api/
//! keep-live` and MCP. `ok` stays `true` when a sub-service is down (the
//! `disk_ok` band-181 pattern).

use std::path::PathBuf;

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use gsv::boxes::ui::{render_card, CARD_NAMES};
use gsv::server::router;
use gsv::AppState;
use serde_json::{json, Value};
use tokio::sync::broadcast;
use tower::ServiceExt;

fn kit_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn app() -> axum::Router {
    let (tx, _rx) = broadcast::channel(64);
    let state = AppState::new(Some(kit_root()), None, tx);
    router(state)
}

/// Point every peer at a dead port + missing heartbeat so probes fail fast
/// (connection-refused) and are hermetic.
fn dead_peers() {
    std::env::set_var("GSV_KEEP_LIVE_GSV_URL", "http://127.0.0.1:59998/api/health");
    std::env::set_var(
        "GSV_KEEP_LIVE_TELENETIS_URL",
        "http://127.0.0.1:59997/health",
    );
    std::env::set_var("GSV_KEEP_LIVE_OMNIROUTE_URL", "http://127.0.0.1:59996");
    std::env::set_var(
        "LLAMA_HEARTBEAT_PATH",
        "C:/tmp/gsv-keep-live-missing-223.json",
    );
}

fn restore_peers() {
    std::env::remove_var("GSV_KEEP_LIVE_GSV_URL");
    std::env::remove_var("GSV_KEEP_LIVE_TELENETIS_URL");
    std::env::remove_var("GSV_KEEP_LIVE_OMNIROUTE_URL");
    std::env::remove_var("LLAMA_HEARTBEAT_PATH");
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

#[tokio::test]
async fn api_keep_live_and_health_merge_ok_stays_true() {
    dead_peers();
    let keep = get_json(&app(), "/api/keep-live").await;
    let health = get_json(&app(), "/api/health").await;
    restore_peers();
    assert_eq!(keep.0, StatusCode::OK);
    assert_eq!(keep.1["ok"], true);
    for key in ["gsv", "telenetis", "llama_rs", "omniroute"] {
        assert!(keep.1[key]["alive"].is_boolean(), "{key} missing alive");
    }
    assert_eq!(keep.1["gsv"]["url"], "http://127.0.0.1:59998/api/health");
    assert_eq!(health.0, StatusCode::OK);
    assert_eq!(health.1["ok"], true, "keep-live must not flip health ok");
    assert!(health.1["keep_live"]["llama_rs"]["alive"].is_boolean());
    assert_eq!(health.1["keep_live"]["gsv"]["alive"], false);
}

#[test]
fn render_keep_live_rows() {
    let d = json!({
        "ok": true,
        "gsv": { "alive": true, "url": "http://127.0.0.1:9999/api/health", "version": "0.223.0" },
        "telenetis": { "alive": false, "url": "http://127.0.0.1:9800/health" },
        "llama_rs": { "alive": false, "url": "S:/rust/llama-rs/target/live/llama_heartbeat.json" },
        "omniroute": { "alive": true, "url": "http://127.0.0.1:3000" }
    });
    let html = render_card("keep-live", &d).expect("rows");
    assert!(html.contains("keep-live 4 peers"), "{html}");
    assert!(html.contains("<th>peer</th>"), "{html}");
    assert!(html.contains("0.223.0"), "{html}");
    assert!(html.contains("<td>gsv</td>"), "{html}");
}

#[test]
fn render_keep_live_empty_and_error() {
    let empty = json!({ "ok": true });
    let html = render_card("keep-live", &empty).expect("empty");
    assert!(html.contains("keep-live — no data"), "{html}");
    let err = json!({ "ok": false, "error": "stand-error" });
    let html = render_card("keep-live", &err).expect("err");
    assert!(
        html.contains("<span class='err'>stand-error</span>"),
        "{html}"
    );
}

#[test]
fn card_names_include_keep_live() {
    assert!(CARD_NAMES.contains(&"keep-live"));
    assert_eq!(CARD_NAMES.len(), 43);
}
