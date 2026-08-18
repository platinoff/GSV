//! GSV drain fingerprint contracts (band 146).
//!
//! JSONL append/latest, `GET /api/fingerprints`, ops card `fingerprints`.
//! Version tests compare `env!("CARGO_PKG_VERSION")` (no hardcoded `0.1.0`).

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use gsv::boxes::fingerprint::{self, Fingerprint};
use gsv::boxes::ui::{render_card, CARD_NAMES};
use gsv::server::router;
use gsv::AppState;
use serde_json::Value;
use tokio::sync::broadcast;
use tower::ServiceExt;

fn kit_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn tmp_jsonl() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!("gsv-fp-{}-{}.jsonl", std::process::id(), nanos))
}

fn sample(summary: &str) -> Fingerprint {
    Fingerprint {
        ts: "2026-08-18T03:00:00Z".into(),
        actor: "agent".into(),
        ide: "cursor".into(),
        model: "grok-4.6".into(),
        agent: "orchestrator".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        git_head: Some("abc1234".into()),
        band: Some("146".into()),
        summary: summary.into(),
        product: "gsv".into(),
    }
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

#[test]
fn jsonl_path_is_under_docs_gsv() {
    let p = fingerprint::jsonl_path(&kit_root());
    assert!(p.ends_with(Path::new("docs/gsv/fingerprints.jsonl")));
}

#[test]
fn legacy_jsonl_without_product_defaults_to_gsv() {
    let raw = r#"{"ts":"2026-08-18T15:06:27Z","actor":"agent","ide":"cursor","model":"grok-4.6","agent":"orchestrator","version":"0.149.0","git_head":"6ef8cd1","band":"149","summary":"omniroute PRODUCTS.md"}"#;
    let fp: Fingerprint = serde_json::from_str(raw).expect("legacy row");
    assert_eq!(fp.product, "gsv");
    assert_eq!(fp.version, "0.149.0");
}

#[test]
fn pkg_version_reads_cargo_and_npm() {
    let dir = std::env::temp_dir().join(format!("gsv-fp-ver-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("tmpdir");
    std::fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"1.2.3\"\n",
    )
    .expect("toml");
    assert_eq!(fingerprint::pkg_version(&dir).as_deref(), Some("1.2.3"));
    let npm = dir.join("npm-only");
    std::fs::create_dir_all(&npm).expect("npm dir");
    std::fs::write(
        npm.join("package.json"),
        r#"{"name":"omniroute","version":"3.8.50"}"#,
    )
    .expect("json");
    assert_eq!(fingerprint::pkg_version(&npm).as_deref(), Some("3.8.50"));
}

#[test]
fn wire_debug_separates_gsv_server_from_selected_product() {
    let w = fingerprint::wire(&kit_root(), Some("omniroute"), 3);
    assert_eq!(w["ok"], true);
    assert_eq!(w["server_product"], "gsv");
    assert_eq!(w["server_version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(w["selected"], "omniroute");
    assert_eq!(w["cross_product"], true);
    let sel_ver = w["selected_version"].as_str().unwrap_or("");
    assert!(!sel_ver.is_empty(), "{w}");
    assert_ne!(
        sel_ver,
        env!("CARGO_PKG_VERSION"),
        "omniroute npm version must not be GSV crate version"
    );
}

#[test]
fn append_then_latest_newest_first() {
    let path = tmp_jsonl();
    let _ = std::fs::remove_file(&path);
    fingerprint::append(&path, &sample("first")).expect("append first");
    let mut second = sample("second");
    second.ts = "2026-08-18T03:01:00Z".into();
    fingerprint::append(&path, &second).expect("append second");
    let got = fingerprint::latest(&path, 10);
    assert_eq!(got.len(), 2);
    assert_eq!(got[0].summary, "second");
    assert_eq!(got[1].summary, "first");
    assert_eq!(got[0].version, env!("CARGO_PKG_VERSION"));
    let one = fingerprint::latest(&path, 1);
    assert_eq!(one.len(), 1);
    assert_eq!(one[0].summary, "second");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn latest_missing_file_is_empty() {
    let path = std::env::temp_dir().join("gsv-fp-missing-no-such.jsonl");
    let _ = std::fs::remove_file(&path);
    assert!(fingerprint::latest(&path, 20).is_empty());
}

#[test]
fn clamp_limit_defaults_and_caps() {
    assert_eq!(fingerprint::clamp_limit(None), 20);
    assert_eq!(fingerprint::clamp_limit(Some(0)), 1);
    assert_eq!(fingerprint::clamp_limit(Some(500)), 100);
}

#[tokio::test]
async fn fingerprints_http_ok_shape() {
    let (app, _state) = app();
    let (status, json) = get(&app, "/api/fingerprints").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["ok"], true);
    assert!(json["fingerprints"].is_array());
    assert_eq!(json["path"], "docs/gsv/fingerprints.jsonl");
}

#[tokio::test]
async fn fingerprints_http_honors_limit() {
    let (app, _state) = app();
    let (status, json) = get(&app, "/api/fingerprints?limit=1").await;
    assert_eq!(status, StatusCode::OK);
    let arr = json["fingerprints"].as_array().expect("array");
    assert!(arr.len() <= 1);
}

#[test]
fn render_fingerprints_lists_latest_row() {
    let wire = serde_json::json!({
        "ok": true,
        "path": "docs/gsv/fingerprints.jsonl",
        "server_product": "gsv",
        "server_version": env!("CARGO_PKG_VERSION"),
        "selected": "omniroute",
        "selected_version": "3.8.50",
        "cross_product": true,
        "count": 1,
        "fingerprints": [{
            "ts": "2026-08-18T03:00:00Z",
            "actor": "agent",
            "ide": "cursor",
            "model": "grok-4.6",
            "agent": "orchestrator",
            "version": env!("CARGO_PKG_VERSION"),
            "git_head": "abc1234",
            "band": "146",
            "product": "gsv",
            "summary": "band 146"
        }]
    });
    let html = render_card("fingerprints", &wire).expect("fingerprints card");
    assert!(html.contains("cursor"), "{html}");
    assert!(html.contains("grok-4.6"), "{html}");
    assert!(html.contains("orchestrator"), "{html}");
    assert!(html.contains("band 146"), "{html}");
    assert!(html.contains(">product<"), "{html}");
    assert!(html.contains("gsv"), "{html}");
    assert!(html.contains("server"), "{html}");
}

#[test]
fn render_fingerprints_empty_and_error() {
    let empty = serde_json::json!({ "ok": true, "fingerprints": [] });
    let html = render_card("fingerprints", &empty).expect("empty");
    assert!(html.contains("fingerprints — no data"), "{html}");
    let err = serde_json::json!({ "ok": false, "error": "stand-error" });
    let html = render_card("fingerprints", &err).expect("err");
    assert!(
        html.contains("<span class='err'>stand-error</span>"),
        "{html}"
    );
}

#[test]
fn card_names_include_fingerprints() {
    assert!(CARD_NAMES.contains(&"fingerprints"));
}

#[test]
fn bump_and_fingerprint_are_rust() {
    let root = kit_root();
    assert!(root.join("src/boxes/fingerprint.rs").is_file());
    assert!(root.join("src/boxes/xtask.rs").is_file());
    assert!(!root.join("scripts/gsv-bump-version.sh").is_file());
    assert!(!root.join("scripts/gsv-fingerprint.sh").is_file());
}

fn write_pkg_toml(dir: &Path, version: &str) -> PathBuf {
    std::fs::create_dir_all(dir).expect("tmpdir");
    let toml = dir.join("Cargo.toml");
    std::fs::write(
        &toml,
        format!(
            "[workspace]\nresolver = \"2\"\n\n[package]\nname = \"gsv\"\nversion = \"{version}\"\nedition = \"2021\"\n"
        ),
    )
    .expect("write toml");
    toml
}

#[test]
fn bump_version_sets_minor_to_band() {
    let dir = std::env::temp_dir().join(format!("gsv-bump-band-{}", std::process::id()));
    let toml = write_pkg_toml(&dir, "0.1.3");
    let ver = fingerprint::bump_package_version(&toml, 149).expect("bump");
    assert_eq!(ver, "0.149.0");
    let text = std::fs::read_to_string(&toml).expect("read toml");
    assert!(
        text.contains("version = \"0.149.0\""),
        "minor = band: {text}"
    );
    assert!(text.contains("[workspace]"), "keep workspace: {text}");
}

#[test]
fn bump_version_patches_within_same_band() {
    let dir = std::env::temp_dir().join(format!("gsv-bump-patch-{}", std::process::id()));
    let toml = write_pkg_toml(&dir, "0.149.0");
    let ver = fingerprint::bump_package_version(&toml, 149).expect("bump");
    assert_eq!(ver, "0.149.1");
    let text = std::fs::read_to_string(&toml).expect("read toml");
    assert!(
        text.contains("version = \"0.149.1\""),
        "same-band patch +1: {text}"
    );
}

#[test]
fn bump_version_rejects_missing_package() {
    let dir = std::env::temp_dir().join(format!("gsv-bump-noband-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("tmpdir");
    let toml = dir.join("Cargo.toml");
    std::fs::write(&toml, "[workspace]\nresolver = \"2\"\n").expect("write");
    assert!(fingerprint::bump_package_version(&toml, 149).is_err());
}

#[test]
fn fingerprint_record_appends_jsonl_and_prints_trailers() {
    let path = tmp_jsonl();
    let _ = std::fs::remove_file(&path);
    let (fp, stdout) = fingerprint::record(fingerprint::RecordOpts {
        kit_root: &kit_root(),
        jsonl: Some(&path),
        product_root: &kit_root(),
        actor: "agent",
        ide: "cursor",
        model: "grok-4.6",
        agent: "orchestrator",
        band: Some("146"),
        summary: "contract",
        product: "gsv",
    })
    .expect("record");
    assert!(stdout.contains("Gsv-Actor: agent"), "{stdout}");
    assert!(stdout.contains("Gsv-Ide: cursor"), "{stdout}");
    assert!(stdout.contains("Gsv-Model: grok-4.6"), "{stdout}");
    assert_eq!(fp.ide, "cursor");
    assert_eq!(fp.summary, "contract");
    assert_eq!(fp.version, env!("CARGO_PKG_VERSION"));
    assert_eq!(fp.product, "gsv");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn fingerprint_record_uses_selected_product_pkg_version() {
    let path = tmp_jsonl();
    let _ = std::fs::remove_file(&path);
    let prod = std::env::temp_dir().join(format!("gsv-fp-omni-{}", std::process::id()));
    std::fs::create_dir_all(&prod).expect("prod");
    std::fs::write(
        prod.join("package.json"),
        r#"{"name":"omniroute","version":"3.8.50"}"#,
    )
    .expect("json");
    let (fp, stdout) = fingerprint::record(fingerprint::RecordOpts {
        kit_root: &kit_root(),
        jsonl: Some(&path),
        product_root: &prod,
        actor: "agent",
        ide: "cursor",
        model: "grok-4.6",
        agent: "orchestrator",
        band: None,
        summary: "omni drain",
        product: "omniroute",
    })
    .expect("record");
    assert!(stdout.contains("Gsv-Product: omniroute"), "{stdout}");
    assert_eq!(fp.product, "omniroute");
    assert_eq!(fp.version, "3.8.50");
    assert_ne!(fp.version, env!("CARGO_PKG_VERSION"));
    let _ = std::fs::remove_file(&path);
}
