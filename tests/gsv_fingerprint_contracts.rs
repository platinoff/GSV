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

fn msys_bash() -> std::process::Command {
    let msys = PathBuf::from("C:/msys64/usr/bin/bash.exe");
    if msys.is_file() {
        std::process::Command::new(msys)
    } else {
        std::process::Command::new("bash")
    }
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
            "summary": "band 146"
        }]
    });
    let html = render_card("fingerprints", &wire).expect("fingerprints card");
    assert!(html.contains("cursor"), "{html}");
    assert!(html.contains("grok-4.6"), "{html}");
    assert!(html.contains("orchestrator"), "{html}");
    assert!(html.contains("band 146"), "{html}");
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
fn bump_and_fingerprint_scripts_exist() {
    let root = kit_root();
    assert!(root.join("scripts/gsv-bump-version.sh").is_file());
    assert!(root.join("scripts/gsv-fingerprint.sh").is_file());
}

#[test]
fn bump_version_script_increments_package_patch() {
    let dir = std::env::temp_dir().join(format!("gsv-bump-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("tmpdir");
    let toml = dir.join("Cargo.toml");
    std::fs::write(
        &toml,
        "[workspace]\nresolver = \"2\"\n\n[package]\nname = \"gsv\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .expect("write toml");
    let script = kit_root().join("scripts/gsv-bump-version.sh");
    let st = msys_bash().arg(&script).arg(&toml).status().expect("bash");
    assert!(st.success(), "bump script exit");
    let text = std::fs::read_to_string(&toml).expect("read toml");
    assert!(text.contains("version = \"0.1.1\""), "patch +1: {text}");
    assert!(text.contains("[workspace]"), "keep workspace: {text}");
}

#[test]
fn fingerprint_script_appends_jsonl_and_prints_trailers() {
    let path = tmp_jsonl();
    let _ = std::fs::remove_file(&path);
    let script = kit_root().join("scripts/gsv-fingerprint.sh");
    let out = msys_bash()
        .arg(&script)
        .env("GSV_FINGERPRINT_FILE", &path)
        .env("GSV_ACTOR", "agent")
        .env("GSV_IDE", "cursor")
        .env("GSV_MODEL", "grok-4.6")
        .env("GSV_AGENT", "orchestrator")
        .env("GSV_BAND", "146")
        .env("GSV_SUMMARY", "contract")
        .output()
        .expect("bash");
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Gsv-Actor: agent"), "{stdout}");
    assert!(stdout.contains("Gsv-Ide: cursor"), "{stdout}");
    assert!(stdout.contains("Gsv-Model: grok-4.6"), "{stdout}");
    let text = std::fs::read_to_string(&path).expect("jsonl");
    let fp: Fingerprint = serde_json::from_str(text.trim()).expect("parse");
    assert_eq!(fp.ide, "cursor");
    assert_eq!(fp.summary, "contract");
    assert_eq!(fp.version, env!("CARGO_PKG_VERSION"));
    let _ = std::fs::remove_file(&path);
}
