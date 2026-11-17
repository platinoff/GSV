//! P4 protocol federation contracts (FM↔tickets, GSV side only).
//!
//! GSV stores FM **ids** in `ticket.fm_ref` (set at claim time or via
//! `POST /api/tickets/link`), answers `GET /api/tickets?fm=<id>` with only
//! the linked rows, and never imports FM prose. These contracts pin the
//! wire shape; the FM-side reader is poolAI's business.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use gsv::boxes::settings;
use gsv::server::router;
use gsv::AppState;
use serde_json::{json, Value};
use tokio::sync::broadcast;
use tower::ServiceExt;

fn nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

fn temp_kit(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "gsv-fm-fed-{tag}-{}-{}",
        std::process::id(),
        nanos()
    ));
    let _ = std::fs::create_dir_all(dir.join("docs/gsv"));
    let _ = std::fs::create_dir_all(dir.join("data"));
    dir
}

fn app_kit(kit: PathBuf) -> axum::Router {
    let data = kit.join("data");
    let (tx, _rx) = broadcast::channel(32);
    let state = AppState::new(Some(kit), Some(data), tx);
    router(state)
}

fn enable_claim(data: &Path) {
    settings::save(
        data,
        &settings::SettingsFile {
            workflows: settings::Workflows {
                enabled: vec!["ticket-claim".into()],
            },
            ..Default::default()
        },
    )
    .expect("save workflows");
}

async fn post_json(app: &axum::Router, path: &str, body: Value) -> (StatusCode, Value) {
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(path)
                .method(Method::POST)
                .header(axum::http::header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
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
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

async fn create_ticket(app: &axum::Router, title: &str) -> String {
    let (status, json) = post_json(
        app,
        "/api/tickets",
        json!({ "title": title, "body": "federation fixture", "product": "gsv" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert_eq!(json["ok"], true);
    json["ticket"]["id"].as_str().expect("id").to_string()
}

#[tokio::test]
async fn claim_with_fm_stamps_link_and_persists() {
    let kit = temp_kit("claim-fm");
    enable_claim(&kit.join("data"));
    let app = app_kit(kit);
    let id = create_ticket(&app, "claim fm fixture").await;

    let (status, json) = post_json(
        &app,
        "/api/tickets/claim",
        json!({ "id": id, "fm": "FM-017" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert_eq!(json["ok"], true);
    assert_eq!(json["ticket"]["fm_ref"], "FM-017");
    assert_eq!(json["ticket"]["status"], "in_progress");
}

#[tokio::test]
async fn claim_without_fm_leaves_fm_ref_absent_in_wire() {
    let kit = temp_kit("claim-nofm");
    enable_claim(&kit.join("data"));
    let app = app_kit(kit);
    let id = create_ticket(&app, "claim no fm fixture").await;

    let (status, json) = post_json(&app, "/api/tickets/claim", json!({ "id": id })).await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert_eq!(json["ok"], true);
    // skip_serializing_if = "String::is_empty" keeps the wire minimal.
    assert!(json["ticket"].get("fm_ref").is_none(), "{json}");
}

#[tokio::test]
async fn link_sets_fm_on_existing_ticket() {
    let kit = temp_kit("link-set");
    let app = app_kit(kit);
    let id = create_ticket(&app, "link fixture").await;

    let (status, json) = post_json(
        &app,
        "/api/tickets/link",
        json!({ "id": id, "fm": "PH-S1579" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert_eq!(json["ok"], true);
    assert_eq!(json["ticket"]["fm_ref"], "PH-S1579");
}

#[tokio::test]
async fn link_unknown_id_is_404() {
    let kit = temp_kit("link-404");
    let app = app_kit(kit);
    let (status, json) = post_json(
        &app,
        "/api/tickets/link",
        json!({ "id": "no-such-id", "fm": "FM-1" }),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(json["ok"], false);
}

#[tokio::test]
async fn link_rejects_garbage_fm() {
    let kit = temp_kit("link-400");
    let app = app_kit(kit);
    let id = create_ticket(&app, "link bad fm fixture").await;

    let (status, json) = post_json(
        &app,
        "/api/tickets/link",
        json!({ "id": id, "fm": "FM 9!" }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(json["ok"], false);

    let too_long = "A".repeat(130);
    let (status2, json2) = post_json(
        &app,
        "/api/tickets/link",
        json!({ "id": id, "fm": too_long }),
    )
    .await;
    assert_eq!(status2, StatusCode::BAD_REQUEST);
    assert_eq!(json2["ok"], false);
}

#[tokio::test]
async fn list_fm_filter_returns_only_linked_rows() {
    let kit = temp_kit("link-filter");
    enable_claim(&kit.join("data"));
    let app = app_kit(kit);

    let a = create_ticket(&app, "federation row A").await;
    let b = create_ticket(&app, "federation row B").await;
    let (_, ja) = post_json(
        &app,
        "/api/tickets/link",
        json!({ "id": a, "fm": "FM-007" }),
    )
    .await;
    assert_eq!(ja["ticket"]["fm_ref"], "FM-007");
    let (_, jb) = post_json(
        &app,
        "/api/tickets/link",
        json!({ "id": b, "fm": "PH-S156" }),
    )
    .await;
    assert_eq!(jb["ticket"]["fm_ref"], "PH-S156");

    let (status, json) = get_json(&app, "/api/tickets?fm=FM-007").await;
    assert_eq!(status, StatusCode::OK, "{json}");
    let rows = json["tickets"].as_array().expect("tickets").clone();
    assert_eq!(rows.len(), 1, "{json}");
    assert_eq!(rows[0]["fm_ref"], "FM-007");

    let (status2, json2) = get_json(&app, "/api/tickets?fm=PH-S156").await;
    assert_eq!(status2, StatusCode::OK);
    let rows2 = json2["tickets"].as_array().expect("tickets").clone();
    assert_eq!(rows2.len(), 1);
    assert_eq!(rows2[0]["fm_ref"], "PH-S156");

    // No filter keeps the full board.
    let (status3, json3) = get_json(&app, "/api/tickets").await;
    assert_eq!(status3, StatusCode::OK);
    let rows3 = json3["tickets"].as_array().expect("tickets").clone();
    assert_eq!(rows3.len(), 2);
}
