//! GSV VDT products picker contracts (band 145).
//!
//! Discovery mirrors `scripts/list-vdt-products.sh` (workspace ∪ sibling git ∪ kit).
//! HTTP: list / select / open / scan. Unknown id → 404 `{ok:false}`.

use std::path::PathBuf;

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use gsv::boxes::products;
use gsv::boxes::ui::{render_card, CARD_NAMES};
use gsv::server::router;
use gsv::AppState;
use serde_json::Value;
use tokio::sync::broadcast;
use tower::ServiceExt;

fn kit_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn app() -> (axum::Router, AppState) {
    let (tx, _rx) = broadcast::channel(64);
    let state = AppState::new(Some(kit_root()), None, tx);
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

#[test]
fn discover_includes_gsv_kit() {
    let root = kit_root();
    let rows = products::discover(&root);
    assert!(
        rows.iter()
            .any(|r| r.id == "gsv" && r.kind == "rust" && r.registered),
        "gsv rust registered missing: {rows:?}"
    );
}

#[test]
fn discover_paths_use_forward_slashes() {
    let rows = products::discover(&kit_root());
    for row in &rows {
        assert!(
            !row.path.contains('\\'),
            "path must use / not \\: {}",
            row.path
        );
    }
}

#[test]
fn discover_dedups_kit_root() {
    let rows = products::discover(&kit_root());
    let gsv = rows.iter().filter(|r| r.id == "gsv").count();
    assert_eq!(gsv, 1, "kit must appear once: {rows:?}");
}

#[test]
fn unknown_id_is_not_in_discover_set() {
    let rows = products::discover(&kit_root());
    assert!(products::lookup(&rows, "not-a-product").is_none());
}

#[tokio::test]
async fn products_list_returns_ok_rows() {
    let (app, _state) = app();
    let (status, json) = get(&app, "/api/products").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["ok"], true);
    let products = json["products"].as_array().expect("products array");
    assert!(products
        .iter()
        .any(|p| p["id"] == "gsv" && p["registered"] == true));
    assert!(json["selected"].is_null());
}

#[tokio::test]
async fn products_select_unknown_id_is_404() {
    let (app, _state) = app();
    let (status, json) = post(
        &app,
        "/api/products/select",
        serde_json::json!({ "id": "not-a-product" }),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(json["ok"], false);
    assert!(json["error"].is_string());
}

#[tokio::test]
async fn products_open_unknown_id_is_404() {
    let (app, _state) = app();
    let (status, json) = post(
        &app,
        "/api/products/open",
        serde_json::json!({ "id": "not-a-product" }),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(json["ok"], false);
    assert!(json["error"].is_string());
}

#[tokio::test]
async fn products_select_then_list_and_scan() {
    let (app, _state) = app();
    let (status, json) = post(
        &app,
        "/api/products/select",
        serde_json::json!({ "id": "gsv" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["ok"], true);
    assert_eq!(json["selected"], "gsv");

    let (status, json) = get(&app, "/api/products").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["selected"], "gsv");

    let (status, json) = get(&app, "/api/products/scan").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["ok"], true);
    assert_eq!(json["kind"], "rust");
    assert_eq!(json["registered"], true);
    assert_eq!(json["handoff_exists"], true);
    assert_eq!(json["next_exists"], true);
    assert_eq!(json["cargo_name"], "gsv");
    assert!(!json["git_head"].as_str().unwrap_or_default().is_empty());
}

#[tokio::test]
async fn products_scan_without_selection_is_400() {
    let (app, _state) = app();
    let (status, json) = get(&app, "/api/products/scan").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(json["ok"], false);
}

#[tokio::test]
async fn products_open_gsv_is_confined() {
    let (app, _state) = app();
    let (status, json) = post(
        &app,
        "/api/products/open",
        serde_json::json!({ "id": "gsv" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["ok"], true);
    assert_eq!(json["opened"], true);
    let how = json["how"].as_str().unwrap_or_default();
    assert!(how == "explorer" || how == "cursor", "how={how}");
}

#[test]
fn render_products_has_select_and_open_actions() {
    let wire = serde_json::json!({
        "ok": true,
        "selected": "gsv",
        "products": [{
            "id": "gsv",
            "name": "GSV",
            "path": "S:/rust/GSV",
            "kind": "rust",
            "registered": true,
            "source": "workspace",
            "git": true,
            "cargo": true
        }]
    });
    let html = render_card("products", &wire).expect("products card");
    assert!(html.contains("data-action='product-select'"), "{html}");
    assert!(html.contains("data-product-id='gsv'"), "{html}");
    assert!(html.contains("data-action='product-open'"), "{html}");
}

#[test]
fn card_names_include_products() {
    assert!(CARD_NAMES.contains(&"products"));
}

#[test]
fn products_md_registers_omniroute() {
    let text =
        std::fs::read_to_string(kit_root().join("docs/gsv/PRODUCTS.md")).expect("PRODUCTS.md");
    assert!(
        text.contains("| **omniroute**"),
        "owner-opt-in omniroute row missing"
    );
    assert!(
        text.contains("npm test"),
        "omniroute test command should be npm test: {text}"
    );
}

#[test]
fn discover_omniroute_registered_when_sibling_present() {
    let root = kit_root();
    let sibling = root.parent().map(|p| p.join("omniroute"));
    let Some(path) = sibling.filter(|p| p.is_dir()) else {
        return;
    };
    let rows = products::discover(&root);
    let row = rows
        .iter()
        .find(|r| r.id == "omniroute")
        .unwrap_or_else(|| {
            panic!(
                "omniroute sibling at {} not discovered: {rows:?}",
                path.display()
            )
        });
    assert!(row.registered, "omniroute must be registered: {row:?}");
    assert_eq!(row.kind, "node", "{row:?}");
    let scan = products::scan(&root, "omniroute").expect("scan omniroute");
    assert!(scan.handoff_exists, "AGENTS.md counts as handoff: {scan:?}");
    assert!(scan.next_exists, "docs/ROADMAP.md counts as next: {scan:?}");
}
