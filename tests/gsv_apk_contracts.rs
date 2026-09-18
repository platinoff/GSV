//! Phone APK client contract (hub-apk-client).
//!
//! Host-side JSON only. No gradle, no Java, no `:8091`.

use std::path::PathBuf;

use gsv::boxes::apk::{self, CLASS_EDGE, DEFAULT_HUB, ORIGIN, ROLE};
use gsv::boxes::edge;
use gsv::boxes::xtask;

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
