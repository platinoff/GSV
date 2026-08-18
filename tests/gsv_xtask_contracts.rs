//! GSV xtask / disk / rust-dev contracts (band 153).
//!
//! Product tests, benches, and scripts are `.rs`. `GET /api/xtask` and
//! `GET /api/disk` wrap the same box as MCP `gsv_xtask` / `gsv_disk`.

use std::fs;
use std::path::PathBuf;

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use gsv::boxes::xtask;
use gsv::server::router;
use gsv::AppState;
use serde_json::Value;
use tokio::sync::broadcast;
use tower::ServiceExt;

fn kit_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn app() -> axum::Router {
    let (tx, _rx) = broadcast::channel(8);
    let state = AppState::new(Some(kit_root()), None, tx);
    router(state)
}

async fn get_json(app: &axum::Router, uri: &str) -> (StatusCode, Value) {
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
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .expect("body");
    let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, json)
}

#[tokio::test]
async fn api_xtask_catalog_and_disk() {
    let app = app();
    let (status, json) = get_json(&app, "/api/xtask").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["ok"], true);
    assert_eq!(json["invoke"], "cargo xtask <task>");
    let (status, disk) = get_json(&app, "/api/disk").await;
    assert_eq!(status, StatusCode::OK);
    assert!(disk["ok"].is_boolean(), "{disk}");
    assert!(disk["target_dir"].as_str().is_some(), "{disk}");
    let (status, bad) = get_json(&app, "/api/xtask?task=push").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(bad["ok"], false);
}

#[test]
fn product_dirs_have_no_shell_or_ps_harnesses() {
    let root = kit_root();
    for dir in ["scripts", "bin"] {
        let p = root.join(dir);
        if !p.is_dir() {
            continue;
        }
        for e in fs::read_dir(&p).expect("read") {
            let name = e.expect("entry").file_name().to_string_lossy().to_string();
            assert!(
                !name.ends_with(".sh") && !name.ends_with(".ps1") && !name.ends_with(".ps"),
                "product automation must be .rs: {dir}/{name}"
            );
        }
    }
    assert!(root.join("src/bin/gsv_xtask.rs").is_file());
    assert!(root.join("benches/gsv_dev.rs").is_file());
    assert!(root.join("docs/gsv/GSV_RUST_DEV.md").is_file());
}

#[test]
fn cargo_alias_xtask() {
    let cfg = fs::read_to_string(kit_root().join(".cargo/config.toml")).expect("config");
    assert!(
        cfg.contains("xtask = \"run --quiet --bin gsv-xtask --\""),
        "{cfg}"
    );
}

#[test]
fn mcp_readonly_tasks_are_catalog_products_disk() {
    assert_eq!(xtask::MCP_TASKS, &["catalog", "products", "disk"]);
}
