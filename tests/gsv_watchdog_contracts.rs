//! GSV live watchdog contracts (band 150).
//!
//! `gsv-live` only restarts while that process lives. The watchdog probes
//! `GET /api/health` and respawns `target/live/gsv-server.exe` after consecutive
//! failures (grace for update-apply). Spawn is skipped under cargo-test `deps/`.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use gsv::boxes::ui::{render_card, CARD_NAMES};
use gsv::boxes::watchdog::{self, Heartbeat};
use gsv::server::router;
use gsv::AppState;
use serde_json::Value;
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

#[test]
fn health_url_is_canon_loopback() {
    assert_eq!(
        watchdog::health_url(gsv::DEFAULT_HOST, gsv::DEFAULT_PORT),
        "http://127.0.0.1:9999/api/health"
    );
    assert_eq!(
        watchdog::apply_url(gsv::DEFAULT_HOST, gsv::DEFAULT_PORT),
        "http://127.0.0.1:9999/api/update/apply"
    );
}

#[test]
fn tick_resets_failures_on_ok() {
    let t = watchdog::tick(4, true, 3);
    assert!(t.probe_ok);
    assert_eq!(t.consecutive_failures, 0);
    assert!(!t.respawn);
}

#[test]
fn tick_respawns_after_fail_threshold() {
    let first = watchdog::tick(0, false, 2);
    assert!(!first.respawn);
    assert_eq!(first.consecutive_failures, 1);
    let second = watchdog::tick(1, false, 2);
    assert!(second.respawn);
    assert_eq!(second.consecutive_failures, 2);
}

#[test]
fn respawn_cooldown_blocks_fork_bomb() {
    assert!(watchdog::should_respawn(2, 2, 100, 120, 10));
    assert!(!watchdog::should_respawn(2, 2, 100, 105, 10));
    assert!(!watchdog::should_respawn(1, 2, 0, 1_000, 10));
}

#[test]
fn parse_health_ok_requires_200_and_ok_true() {
    assert!(watchdog::parse_health_ok(200, r#"{"ok":true}"#));
    assert!(!watchdog::parse_health_ok(200, r#"{"ok":false}"#));
    assert!(!watchdog::parse_health_ok(503, r#"{"ok":true}"#));
    assert!(!watchdog::parse_health_ok(200, "not-json"));
}

#[test]
fn parse_apply_ok_requires_applying() {
    assert!(watchdog::parse_apply_ok(
        200,
        r#"{"ok":true,"applying":true}"#
    ));
    assert!(!watchdog::parse_apply_ok(200, r#"{"ok":true}"#));
    assert!(!watchdog::parse_apply_ok(
        200,
        r#"{"ok":false,"applying":true}"#
    ));
}

#[test]
fn parse_health_probe_reads_version_lag() {
    let p = watchdog::parse_health_probe(
        200,
        r#"{"ok":true,"version_lag":true,"crate_version":"0.165.0"}"#,
    );
    assert!(p.ok);
    assert!(p.version_lag);
    let p = watchdog::parse_health_probe(200, r#"{"ok":true}"#);
    assert!(p.ok);
    assert!(!p.version_lag);
    let p = watchdog::parse_health_probe(503, r#"{"ok":true,"version_lag":true}"#);
    assert!(!p.ok);
}

#[test]
fn lockstep_action_never_stays_probe_ok() {
    assert_eq!(watchdog::lockstep_action(true), "lockstep-apply");
    assert_eq!(watchdog::lockstep_action(false), "lockstep-fail");
}

#[test]
fn oneshot_apply_when_peer_running_and_newer() {
    assert!(watchdog::should_oneshot_apply(true, true));
    assert!(!watchdog::should_oneshot_apply(true, false));
    assert!(!watchdog::should_oneshot_apply(false, true));
}

#[test]
fn apply_origin_is_loopback_http() {
    assert_eq!(
        watchdog::apply_origin("127.0.0.1", 9999),
        "http://127.0.0.1:9999"
    );
}

#[test]
fn heartbeat_reads_old_json_without_lockstep_fields() {
    let dir = std::env::temp_dir().join(format!("gsv-wd-oldjson-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("tmpdir");
    let path = dir.join("watchdog.json");
    std::fs::write(
        &path,
        r#"{"ts":"t","epoch_secs":1,"pid":7,"last_ok":true,"consecutive_failures":0,"last_action":"probe-ok","host":"127.0.0.1","port":9999}"#,
    )
    .expect("write old");
    let got = watchdog::read_heartbeat(&path).expect("compat");
    assert_eq!(got.pid, 7);
    assert_eq!(got.last_apply_status, 0);
    assert!(got.lockstep_note.is_empty());
}

#[test]
fn lockstep_cooldown_matches_respawn() {
    assert!(watchdog::should_lockstep(true, 100, 120, 10));
    assert!(!watchdog::should_lockstep(true, 100, 105, 10));
    assert!(!watchdog::should_lockstep(false, 0, 1_000, 10));
}

#[test]
fn debug_newer_than_live_when_debug_mtime_greater() {
    let dir = std::env::temp_dir().join(format!("gsv-wd-newer-{}", std::process::id()));
    let debug_dir = dir.join("target/debug");
    let live_dir = dir.join("target/live");
    std::fs::create_dir_all(&debug_dir).expect("debug dir");
    std::fs::create_dir_all(&live_dir).expect("live dir");
    let debug = debug_dir.join("gsv-server.exe");
    let live = live_dir.join("gsv-server.exe");
    std::fs::write(&live, b"old").expect("live");
    std::thread::sleep(std::time::Duration::from_millis(1100));
    std::fs::write(&debug, b"new").expect("debug");
    assert!(
        watchdog::debug_newer_than_live(&dir),
        "debug mtime must beat live"
    );
}

#[test]
fn heartbeat_roundtrip_and_freshness() {
    let dir = std::env::temp_dir().join(format!("gsv-wd-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("tmpdir");
    let path = dir.join("watchdog.json");
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let hb = Heartbeat {
        ts: "2026-08-18T15:00:00Z".into(),
        epoch_secs: now,
        pid: 42,
        last_ok: true,
        consecutive_failures: 0,
        last_action: "probe-ok".into(),
        host: "127.0.0.1".into(),
        port: 9999,
        last_apply_status: 0,
        lockstep_note: String::new(),
    };
    watchdog::write_heartbeat(&path, &hb).expect("write");
    let got = watchdog::read_heartbeat(&path).expect("read");
    assert_eq!(got.pid, 42);
    assert!(watchdog::heartbeat_fresh(&got, now, 20));
    assert!(!watchdog::heartbeat_fresh(&got, now + 30, 20));
}

#[test]
fn copy_debug_to_live_makes_live_exe() {
    let dir = std::env::temp_dir().join(format!("gsv-wd-copy-{}", std::process::id()));
    let debug_dir = dir.join("target/debug");
    let live_dir = dir.join("target/live");
    std::fs::create_dir_all(&debug_dir).expect("debug dir");
    let debug = debug_dir.join("gsv-server.exe");
    std::fs::write(&debug, b"fake").expect("debug exe");
    let live = watchdog::copy_debug_to_live(&dir).expect("copy");
    assert_eq!(live, live_dir.join("gsv-server.exe"));
    assert_eq!(std::fs::read(&live).expect("read live"), b"fake");
}

#[test]
fn copy_debug_to_live_copies_mcp_when_present() {
    let dir = std::env::temp_dir().join(format!("gsv-wd-mcp-{}", std::process::id()));
    let debug_dir = dir.join("target/debug");
    std::fs::create_dir_all(&debug_dir).expect("debug dir");
    std::fs::write(debug_dir.join("gsv-server.exe"), b"server").expect("server");
    std::fs::write(debug_dir.join("gsv-mcp.exe"), b"mcp").expect("mcp");
    let live = watchdog::copy_debug_to_live(&dir).expect("copy");
    assert_eq!(std::fs::read(&live).expect("read server"), b"server");
    let mcp_live = dir.join("target/live").join(watchdog::mcp_exe_name());
    assert_eq!(std::fs::read(&mcp_live).expect("read mcp"), b"mcp");
}

#[test]
fn copy_debug_to_live_copies_watchdog_when_present() {
    let dir = std::env::temp_dir().join(format!("gsv-wd-wdog-{}", std::process::id()));
    let debug_dir = dir.join("target/debug");
    std::fs::create_dir_all(&debug_dir).expect("debug dir");
    std::fs::write(debug_dir.join("gsv-server.exe"), b"server").expect("server");
    std::fs::write(debug_dir.join("gsv-watchdog.exe"), b"wdog").expect("watchdog");
    let _ = watchdog::copy_debug_to_live(&dir).expect("copy");
    let wd_live = dir.join("target/live").join(watchdog::watchdog_exe_name());
    assert_eq!(std::fs::read(&wd_live).expect("read watchdog"), b"wdog");
}

#[test]
fn spawn_live_skipped_in_cargo_test_harness() {
    let out = watchdog::spawn_live(&kit_root(), "127.0.0.1", 9999).expect("spawn");
    assert_eq!(out, watchdog::SpawnOutcome::HarnessSkipped);
}

#[test]
fn spawn_live_windows_flags_hide_console() {
    assert_eq!(
        watchdog::SPAWN_LIVE_WINDOWS_FLAGS & gsv::vision::CREATE_NO_WINDOW,
        gsv::vision::CREATE_NO_WINDOW,
        "detached live copy must also set CREATE_NO_WINDOW"
    );
    const CREATE_BREAKAWAY_FROM_JOB: u32 = 0x0100_0000;
    assert_eq!(
        watchdog::SPAWN_LIVE_WINDOWS_FLAGS & CREATE_BREAKAWAY_FROM_JOB,
        0,
        "CREATE_BREAKAWAY_FROM_JOB causes Access denied (5) inside Cursor/job objects"
    );
}

#[test]
fn watchdog_bin_declared() {
    let root = kit_root();
    assert!(!root.join("scripts/gsv-watchdog.sh").is_file());
    assert!(!root.join("scripts/gsv-watchdog-install.sh").is_file());
    let toml = std::fs::read_to_string(root.join("Cargo.toml")).expect("toml");
    assert!(
        toml.contains("name = \"gsv-watchdog\""),
        "Cargo.toml must declare gsv-watchdog bin"
    );
    assert!(
        toml.contains("name = \"gsv-live\""),
        "Cargo.toml must declare gsv-live bin"
    );
    assert!(
        toml.contains("name = \"gsv-xtask\""),
        "Cargo.toml must declare gsv-xtask bin"
    );
}

#[tokio::test]
async fn api_watchdog_and_health_expose_alive() {
    let app = app();
    let (status, json) = get_json(&app, "/api/watchdog").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["ok"], true);
    assert!(json["alive"].is_boolean(), "{json}");
    assert!(json["path"]
        .as_str()
        .unwrap_or_default()
        .contains("watchdog.json"));
    assert!(json["debug_newer"].is_boolean(), "{json}");

    let (status, json) = get_json(&app, "/api/health").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["ok"], true);
    assert!(json["watchdog_alive"].is_boolean(), "{json}");
    assert!(json["crate_version"].is_string(), "{json}");
    assert!(json["version_lag"].is_boolean(), "{json}");
}

#[test]
fn health_card_lists_watchdog() {
    let html = render_card(
        "health",
        &serde_json::json!({
            "ok": true,
            "name": "Galaxy StarWalker Vision",
            "version": "0.149.0",
            "crate_version": "0.161.0",
            "version_lag": true,
            "uptime_secs": 1,
            "update_available": false,
            "watchdog_alive": true
        }),
    )
    .expect("health card");
    assert!(html.contains("watchdog"), "{html}");
    assert!(html.contains("crate_version"), "{html}");
    assert!(html.contains("version_lag"), "{html}");
}

#[test]
fn render_watchdog_lists_heartbeat() {
    let html = render_card(
        "watchdog",
        &serde_json::json!({
            "ok": true,
            "alive": true,
            "path": "S:/rust/GSV/target/live/watchdog.json",
            "epoch_secs": 1,
            "age_secs": 3,
            "consecutive_failures": 0,
            "pid": 4242,
            "debug_newer": true,
            "last_apply_status": 403,
            "lockstep_note": "cross-site POST rejected",
            "last_action": "lockstep-fail"
        }),
    )
    .expect("watchdog card");
    assert!(html.contains("watchdog.json"), "{html}");
    assert!(html.contains("lockstep-fail"), "{html}");
    assert!(html.contains("4242"), "{html}");
    assert!(html.contains("debug_newer"), "{html}");
    assert!(html.contains("last_apply_status"), "{html}");
    assert!(html.contains("403"), "{html}");
    assert!(html.contains("cross-site POST rejected"), "{html}");
}

#[test]
fn render_watchdog_empty_and_error() {
    let empty = serde_json::json!({ "ok": true });
    let html = render_card("watchdog", &empty).expect("empty");
    assert!(html.contains("watchdog — no data"), "{html}");
    let err = serde_json::json!({ "ok": false, "error": "stand-error" });
    let html = render_card("watchdog", &err).expect("err");
    assert!(
        html.contains("<span class='err'>stand-error</span>"),
        "{html}"
    );
}

#[test]
fn card_names_include_watchdog() {
    assert!(CARD_NAMES.contains(&"watchdog"));
    assert_eq!(CARD_NAMES.len(), 38);
}
