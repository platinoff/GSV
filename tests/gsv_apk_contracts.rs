//! Phone APK client contract (hub-apk-client).
//!
//! Host-side JSON + LAN join through `/api/edge`. No gradle, no Java, no `:8091`.

use std::path::PathBuf;

use gsv::boxes::apk::{self, CLASS_EDGE, DEFAULT_HUB, ORIGIN, ROLE};
use gsv::boxes::edge;
use gsv::boxes::xtask;
use serde_json::json;

#[test]
fn cargo_declares_gsv_apk_bin() {
    let manifest = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));
    assert!(manifest.contains("name = \"gsv-apk\""));
    assert!(manifest.contains("src/bin/gsv_apk.rs"));
}

#[test]
fn terminal_allows_gsv_apk() {
    assert!(gsv::boxes::terminal::WHITELIST.contains(&"gsv-apk"));
}

#[test]
fn never_8091_in_register_urls() {
    let urls: Vec<String> = apk::register_paths("redmi-01")
        .into_iter()
        .map(|p| apk::edge_url(DEFAULT_HUB, &p).expect("allowlisted"))
        .collect();
    for u in &urls {
        assert!(u.contains("/api/edge/"), "{u}");
        assert!(!u.contains("8091"), "{u}");
        assert!(!u.contains(":9800"), "{u}");
    }
    assert!(urls.iter().any(|u| u.contains("discovery/register-remote")));
    assert!(urls
        .iter()
        .any(|u| u.contains("virtual-nodes/redmi-01/pool/join")));
}

#[test]
fn identity_is_apk_edge_not_telegram_worker() {
    let id = apk::identity("redmi-01");
    assert_eq!(id.origin, ORIGIN);
    assert_eq!(id.role, ROLE);
    assert_eq!(id.class, CLASS_EDGE);
    assert!(id.telegram_proxy);
    assert_ne!(id.origin, "telegram_edge");
}

#[test]
fn register_json_has_no_secrets() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let disk = xtask::disk_report(&root, false);
    let body = apk::registration_body("redmi-01", "192.168.2.89", 0, &disk);
    let s = serde_json::to_string(&body).expect("json");
    assert!(!s.contains("bot_token"));
    assert!(!s.contains("GSV_EDGE_TOKEN"));
    assert_eq!(body["metadata"]["origin"], "apk_edge");
    assert!(edge::allowlisted("POST", "discovery/register-remote"));
}

#[test]
fn check_hub_rejects_poolai() {
    assert!(apk::check_hub("http://192.168.2.238:8091/api/v1").is_err());
    assert!(apk::check_hub("http://192.168.2.238:9999").is_ok());
}

#[test]
fn report_json_is_disk_and_settings_not_kvm() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let r = apk::report(&root, DEFAULT_HUB, "redmi-01");
    assert!(r.settings.wifi_debug);
    assert!(r.settings.model_cache.is_none());
    assert_eq!(r.settings.peer_id, "redmi-01");
    assert!(r.disk.ok);
    let v = serde_json::to_value(&r).expect("json");
    assert_eq!(v["settings"]["wifi_debug"], true);
    assert_eq!(v["settings"]["model_cache"], serde_json::Value::Null);
    assert!(v.get("screenshot").is_none());
}

#[test]
fn worker_policy_is_apk_edge_not_mini_app() {
    let p = apk::worker_policy_wire();
    assert_eq!(p["phone_worker"], ORIGIN);
    assert_eq!(p["mini_app"], false);
    assert_eq!(p["chrome"], false);
    assert_eq!(p["webview"], false);
    assert_eq!(p["webgpu"], "probe");
    assert_eq!(p["telenetis_freeze"], true);
    assert_eq!(p["tasks"], "virtual_node");
    assert!(apk::phone_worker_edge_ok(Some(&json!({"task": "x"}))).is_ok());
    assert!(apk::phone_worker_edge_ok(Some(&json!({"origin": "chrome"}))).is_err());
}

#[test]
fn telegram_is_proxy_not_phone_worker() {
    assert!(!apk::telegram_may_be_phone_worker());
    assert!(!apk::is_phone_worker_origin(apk::TELEGRAM_ORIGIN));
    let cmd = apk::telegram_passthrough("command").expect("command");
    assert_eq!(cmd.to, "apk");
    assert_eq!(cmd.origin, ORIGIN);
    assert!(apk::telegram_passthrough("tensor").is_err());
    assert!(apk::telegram_passthrough("Host tests").is_err());
    assert!(apk::telegram_passthrough("Start worker").is_err());
}

#[test]
fn service_account_is_edge_token_not_poolai_admin() {
    let v = gsv::boxes::edge::service_account_wire();
    assert_eq!(v["kind"], "edge_token");
    assert_eq!(v["header"], "x-gsv-edge-token");
    assert_eq!(v["login_blocked"], true);
    assert_eq!(v["poolai_admin_default"], false);
    let s = v.to_string();
    assert!(!s.contains("admin123"));
    assert!(!s.contains("admin/admin"));
}

#[test]
fn telenetis_surface_is_frozen_shell() {
    assert!(apk::telenetis_may_grow("identity").is_ok());
    assert!(apk::telenetis_may_grow("tensor").is_err());
    let v = apk::telenetis_surface_wire();
    assert_eq!(v["surface"], "shell");
    assert_eq!(v["phone_worker"], ORIGIN);
}

#[test]
fn join_dry_run_plans_hops_without_sockets() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let disk = xtask::disk_report(&root, false);
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("rt");
    let v = rt
        .block_on(apk::join_lan(
            DEFAULT_HUB,
            "redmi-01",
            "192.168.2.89",
            &disk,
            true,
            Some("edge-secret-must-not-leak"),
        ))
        .expect("dry");
    assert_eq!(v["ok"], true);
    assert_eq!(v["dry_run"], true);
    assert_eq!(v["origin"], ORIGIN);
    assert_eq!(v["steps"].as_array().expect("steps").len(), 3);
    let raw = v.to_string();
    assert!(!raw.contains("edge-secret-must-not-leak"), "{raw}");
    assert!(!raw.contains("8091"), "{raw}");
    assert!(raw.contains("/api/edge/health"), "{raw}");
}

#[tokio::test]
async fn join_live_posts_token_header_and_redacts() {
    use std::sync::{Arc, Mutex};

    use axum::extract::State;
    use axum::http::HeaderMap;
    use axum::routing::{get, post};
    use axum::{Json, Router};
    use serde_json::Value;

    #[derive(Clone)]
    struct Seen {
        header: Arc<Mutex<Option<String>>>,
        origin: Arc<Mutex<Option<String>>>,
    }

    async fn health() -> Json<Value> {
        Json(json!({ "ok": true, "token": "upstream-leak" }))
    }

    async fn register(
        State(seen): State<Seen>,
        headers: HeaderMap,
        Json(body): Json<Value>,
    ) -> Json<Value> {
        let h = headers
            .get(edge::TOKEN_HEADER)
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);
        *seen.header.lock().expect("h") = h;
        *seen.origin.lock().expect("o") = body
            .pointer("/metadata/origin")
            .and_then(Value::as_str)
            .map(str::to_string);
        Json(json!({ "ok": true, "password": "nope" }))
    }

    async fn join() -> Json<Value> {
        Json(json!({ "ok": true }))
    }

    let seen = Seen {
        header: Arc::new(Mutex::new(None)),
        origin: Arc::new(Mutex::new(None)),
    };
    let app = Router::new()
        .route("/api/edge/health", get(health))
        .route("/api/edge/discovery/register-remote", post(register))
        .route("/api/edge/virtual-nodes/redmi-01/pool/join", post(join))
        .with_state(seen.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve");
    });
    let hub = format!("http://127.0.0.1:{}/api/edge", addr.port());
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let disk = xtask::disk_report(&root, false);
    let v = apk::join_lan(
        &hub,
        "redmi-01",
        "192.168.2.89",
        &disk,
        false,
        Some("edge-secret-must-not-leak"),
    )
    .await
    .expect("join");
    assert_eq!(v["ok"], true, "{v}");
    assert_eq!(v["dry_run"], false, "{v}");
    let raw = v.to_string();
    assert!(!raw.contains("edge-secret-must-not-leak"), "{raw}");
    assert!(!raw.contains("upstream-leak"), "{raw}");
    assert!(!raw.contains("nope"), "{raw}");
    assert!(!raw.contains("8091"), "{raw}");
    assert_eq!(
        seen.header.lock().expect("h").as_deref(),
        Some("edge-secret-must-not-leak")
    );
    assert_eq!(seen.origin.lock().expect("o").as_deref(), Some(ORIGIN));
}

#[tokio::test]
async fn join_live_without_token_refuses_sockets() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let disk = xtask::disk_report(&root, false);
    let err = apk::join_lan(DEFAULT_HUB, "redmi-01", "127.0.0.1", &disk, false, None)
        .await
        .expect_err("token");
    assert_eq!(err.error, "edge token not set");
}

#[test]
fn native_package_rejects_webview_and_java_product() {
    assert_eq!(apk::android_entry(), "join_lan");
    assert!(apk::native_ok("join").is_ok());
    assert!(apk::native_ok("webview").is_err());
    assert!(apk::native_ok("chrome").is_err());
    let v = apk::native_package_wire();
    assert_eq!(v["webview"], false);
    assert_eq!(v["gradle"], false);
    assert_eq!(v["java"], false);
    assert_eq!(v["package"], "org.gsv.apk");
    let xml = apk::android_manifest();
    assert!(xml.contains("org.gsv.apk"));
    assert!(!xml.to_ascii_lowercase().contains("webview"));
    assert!(!xml.contains("8091"));
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for walk in [root.join("src"), root.join("tests")] {
        let it = walkdir_names(&walk);
        assert!(
            !it.iter().any(|n| n.ends_with(".java")
                || n.ends_with(".kt")
                || n.ends_with(".gradle")
                || n == "AndroidManifest.xml"),
            "product tree must not contain java/gradle/manifest: {it:?}"
        );
    }
}

fn walkdir_names(dir: &std::path::Path) -> Vec<String> {
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir(dir) else {
        return out;
    };
    for e in rd.flatten() {
        let path = e.path();
        if path.is_dir() {
            out.extend(walkdir_names(&path));
        } else if let Some(n) = path.file_name().and_then(|s| s.to_str()) {
            out.push(n.to_string());
        }
    }
    out
}
