//! Phone APK client contract (Rust-ratio peer, not a Mini App worker).
//!
//! The APK talks only to hub `/api/edge` with the edge token. It never
//! dials poolAI `:8091`. Telegram is a proxy/passthrough to this client,
//! not the worker. WiFi ADB stays the on-device debug/disk path.
//! Canon: [`GSV_AGI_PATH.md`](../../docs/gsv/GSV_AGI_PATH.md).

use serde::Serialize;
use serde_json::{json, Value};

use super::edge;
use super::xtask::{self, DiskReport};

/// Default hub (loopback). Phones override with the LAN `:9999`.
pub const DEFAULT_HUB: &str = "http://127.0.0.1:9999";
/// Metadata origin for discovery register.
pub const ORIGIN: &str = "apk_edge";
/// Grid class: phones do not hold tensors.
pub const CLASS_EDGE: &str = "edge";
/// Virtual-node role (same plane as telegram_edge workers, different origin).
pub const ROLE: &str = "virtual_node";
/// Default peer id until the device sets `GSV_APK_PEER`.
pub const DEFAULT_PEER: &str = "redmi-01";
/// Legacy Mini App / bot origin. Proxy/shell only — never the phone worker.
pub const TELEGRAM_ORIGIN: &str = "telegram_edge";

/// Why a hub URL is rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HubReject {
    pub error: &'static str,
}

/// Identity the APK registers as (no secrets).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ApkIdentity {
    pub peer_id: String,
    pub origin: String,
    pub role: String,
    pub class: String,
    pub telegram_proxy: bool,
}

/// Hub-readable settings (JSON, no secrets, no screenshot loops).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ApkSettings {
    pub hub: String,
    pub peer_id: String,
    pub telegram_proxy: bool,
    /// WiFi ADB stays the on-device debug/disk path.
    pub wifi_debug: bool,
    /// Native APK does not use Chrome/Mini App IDB. `None` until on-device store.
    pub model_cache: Option<String>,
}

/// Disk + identity snapshot for settings / debug.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ApkReport {
    pub ok: bool,
    pub hub: String,
    pub identity: ApkIdentity,
    pub disk: DiskReport,
    pub settings: ApkSettings,
}

/// Parse `GSV_APK_PEER` or the default rabbit id.
pub fn peer_id() -> String {
    std::env::var("GSV_APK_PEER")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_PEER.to_string())
}

/// Parse `GSV_HUB_URL` / `GSV_URL` or [`DEFAULT_HUB`].
pub fn hub_from_env() -> String {
    std::env::var("GSV_HUB_URL")
        .ok()
        .or_else(|| std::env::var("GSV_URL").ok())
        .map(|s| s.trim().trim_end_matches('/').to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_HUB.to_string())
}

/// Reject poolAI `:8091` and empty URLs. Hub must be GSV `:9999` (or path `/api/edge`).
pub fn check_hub(url: &str) -> Result<(), HubReject> {
    let u = url.trim();
    if u.is_empty() {
        return Err(HubReject {
            error: "hub url empty",
        });
    }
    let lower = u.to_ascii_lowercase();
    if lower.contains(":8091") || lower.contains("/api/v1/") {
        return Err(HubReject {
            error: "must not talk to poolAI :8091",
        });
    }
    if lower.contains(":9800") || lower.contains(":8080") || lower.contains(":20128") {
        return Err(HubReject {
            error: "hub must be GSV :9999 /api/edge",
        });
    }
    let has_9999 = lower.contains(":9999");
    let has_edge = lower.contains("/api/edge");
    if !has_9999 && !has_edge {
        return Err(HubReject {
            error: "hub must be GSV :9999 /api/edge",
        });
    }
    Ok(())
}

/// `{hub}/api/edge/{rel}` after [`check_hub`].
pub fn edge_url(hub: &str, rel: &str) -> Result<String, HubReject> {
    check_hub(hub)?;
    let base = hub.trim().trim_end_matches('/');
    let path = rel.trim().trim_start_matches('/');
    let listed = edge::normalize_path(path).is_some()
        && (edge::allowlisted("POST", path) || edge::allowlisted("GET", path));
    if !listed {
        return Err(HubReject {
            error: "path not on edge allowlist",
        });
    }
    if base.ends_with("/api/edge") {
        Ok(format!("{base}/{path}"))
    } else {
        Ok(format!("{base}/api/edge/{path}"))
    }
}

/// Discovery + pool-join relative paths (allowlisted POST).
pub fn register_paths(peer: &str) -> Vec<String> {
    vec![
        "discovery/register-remote".into(),
        format!("virtual-nodes/{peer}/pool/join"),
    ]
}

/// `X-Gsv-Edge-Token` header name (never log the value).
pub fn token_header() -> &'static str {
    edge::TOKEN_HEADER
}

pub fn identity(peer: &str) -> ApkIdentity {
    ApkIdentity {
        peer_id: peer.to_string(),
        origin: ORIGIN.into(),
        role: ROLE.into(),
        class: CLASS_EDGE.into(),
        telegram_proxy: true,
    }
}

/// Body for `POST /api/edge/discovery/register-remote`.
pub fn registration_body(peer: &str, address: &str, port: u16, disk: &DiskReport) -> Value {
    let id = identity(peer);
    json!({
        "peer_id": id.peer_id,
        "address": address,
        "port": port,
        "protocol_version": "apk-1",
        "build_id": env!("CARGO_PKG_VERSION"),
        "capabilities": {
            "cpu_cores": 1,
            "memory_mb": 0,
            "gpu_devices": [],
            "supports_tensor_parallelism": false,
            "supports_pipeline_parallelism": false,
        },
        "metadata": {
            "origin": id.origin,
            "role": id.role,
            "class": id.class,
            "telegram_proxy": id.telegram_proxy,
            "disk_free_mb": disk.free_mb,
            "disk_ok": disk.ok,
        }
    })
}

/// Body for `POST /api/edge/virtual-nodes/{peer}/pool/join`.
pub fn pool_join_body() -> Value {
    json!({
        "max_memory_mb": 256,
        "max_concurrent_requests": 2,
    })
}

/// Telegram → APK (or Mini App chrome shell). Never a tensor/KVM worker.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct TelegramForward {
    pub ok: bool,
    pub kind: String,
    pub to: &'static str,
    pub origin: &'static str,
    pub action: &'static str,
}

fn norm_kind(raw: &str) -> String {
    raw.trim().to_ascii_lowercase().replace(['_', ' '], "-")
}

fn canon_kind(raw: &str) -> String {
    match norm_kind(raw).as_str() {
        "commands" | "cmd" => "command".into(),
        "bind" | "login" => "auth".into(),
        "open" | "deep-link" | "deeplink" | "startapp" => "open_apk".into(),
        "hosttests" | "host-test" | "host_tests" => "host-tests".into(),
        "startworker" | "start-worker" => "start-worker".into(),
        "webview-kvm" | "screenshot" => "kvm".into(),
        "gguf-worker" | "wllama" | "webgpu" => "tensor".into(),
        other => other.to_string(),
    }
}

/// Phone compute origin is only [`ORIGIN`] (`apk_edge`).
pub fn is_phone_worker_origin(origin: &str) -> bool {
    origin.trim().eq_ignore_ascii_case(ORIGIN)
}

/// Telegram / Mini App / WebView must not register as the phone worker.
pub fn telegram_may_be_phone_worker() -> bool {
    false
}

/// Hub policy: Telegram forwards auth/commands/open-APK; chrome may stay a
/// shell; tensor / Host tests / KVM / Start worker are rejected.
pub fn telegram_passthrough(kind: &str) -> Result<TelegramForward, HubReject> {
    let k = canon_kind(kind);
    if k.is_empty() {
        return Err(HubReject {
            error: "telegram kind empty",
        });
    }
    match k.as_str() {
        "auth" => Ok(TelegramForward {
            ok: true,
            kind: "auth".into(),
            to: "apk",
            origin: ORIGIN,
            action: "auth",
        }),
        "command" => Ok(TelegramForward {
            ok: true,
            kind: "command".into(),
            to: "apk",
            origin: ORIGIN,
            action: "command",
        }),
        "open_apk" => Ok(TelegramForward {
            ok: true,
            kind: "open_apk".into(),
            to: "apk",
            origin: ORIGIN,
            action: "open_apk",
        }),
        "shell" | "chrome" | "dashboard" | "board" | "identity" | "lan" => Ok(TelegramForward {
            ok: true,
            kind: k,
            to: "shell",
            origin: TELEGRAM_ORIGIN,
            action: "chrome",
        }),
        _ => Err(HubReject {
            error: "telegram is proxy to APK only",
        }),
    }
}

/// Telenetis Mini App is frozen at identity + chrome + LAN. Tensor/KVM/swarm
/// are probes, not growth. Phone worker stays [`ORIGIN`].
pub fn telenetis_may_grow(kind: &str) -> Result<(), HubReject> {
    let k = canon_kind(kind);
    match k.as_str() {
        "identity" | "chrome" | "shell" | "dashboard" | "board" | "lan" | "webhook" => Ok(()),
        _ => Err(HubReject {
            error: "telenetis surface frozen: shell only",
        }),
    }
}

/// Hub freeze wire (no secrets).
pub fn telenetis_surface_wire() -> Value {
    json!({
        "ok": true,
        "surface": "shell",
        "grow": ["identity", "chrome", "lan"],
        "probe_only": ["tensor", "webgpu"],
        "frozen": ["kvm", "swarm", "start-worker", "host-tests"],
        "phone_worker": ORIGIN,
    })
}

/// Disk + identity (WiFi-debug / APK settings page). Hub reads this JSON
/// instead of screenshot loops.
pub fn report(repo_root: &std::path::Path, hub: &str, peer: &str) -> ApkReport {
    let disk = xtask::disk_report(repo_root, false);
    let hub = hub.trim().trim_end_matches('/').to_string();
    let id = identity(peer);
    ApkReport {
        ok: check_hub(&hub).is_ok() && disk.ok,
        settings: ApkSettings {
            hub: hub.clone(),
            peer_id: id.peer_id.clone(),
            telegram_proxy: id.telegram_proxy,
            wifi_debug: true,
            model_cache: None,
        },
        hub,
        identity: id,
        disk,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boxes::ENV_LOCK;
    use std::path::PathBuf;

    fn root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn rejects_poolai_8091() {
        assert!(check_hub("http://192.168.2.238:8091/api/v1").is_err());
        assert!(check_hub("http://127.0.0.1:8091").is_err());
        assert!(check_hub("http://host/api/v1/topology").is_err());
    }

    #[test]
    fn rejects_telenetis_and_llama() {
        assert!(check_hub("http://192.168.2.238:9800").is_err());
        assert!(check_hub("http://192.168.2.238:8080").is_err());
        assert!(check_hub("http://192.168.2.238:20128").is_err());
        assert!(check_hub("http://127.0.0.1").is_err());
        assert!(check_hub("http://localhost").is_err());
    }

    #[test]
    fn accepts_gsv_9999() {
        assert!(check_hub("http://127.0.0.1:9999").is_ok());
        assert!(check_hub("http://192.168.2.238:9999").is_ok());
        assert!(check_hub("http://192.168.2.238:9999/api/edge").is_ok());
    }

    #[test]
    fn edge_url_never_8091() {
        let u = edge_url("http://192.168.2.238:9999", "discovery/register-remote").unwrap();
        assert!(u.contains("/api/edge/discovery/register-remote"));
        assert!(!u.contains("8091"));
        assert!(edge_url("http://127.0.0.1:8091", "health").is_err());
    }

    #[test]
    fn register_paths_are_allowlisted() {
        for p in register_paths("redmi-01") {
            assert!(
                edge::allowlisted("POST", &p),
                "{p} must be on the edge allowlist"
            );
        }
    }

    #[test]
    fn registration_is_apk_origin_not_telegram_worker() {
        let disk = xtask::disk_report(&root(), false);
        let body = registration_body("redmi-01", "192.168.2.89", 0, &disk);
        assert_eq!(body["metadata"]["origin"], ORIGIN);
        assert_eq!(body["metadata"]["role"], ROLE);
        assert_eq!(body["metadata"]["class"], CLASS_EDGE);
        assert_eq!(body["metadata"]["telegram_proxy"], true);
        assert!(body.get("bot_token").is_none());
        assert!(body["metadata"].get("bot_token").is_none());
    }

    #[test]
    fn report_ok_on_this_repo() {
        let r = report(&root(), DEFAULT_HUB, DEFAULT_PEER);
        assert!(r.ok, "disk+hub should be ok in the GSV tree");
        assert_eq!(r.identity.origin, ORIGIN);
        assert!(r.identity.telegram_proxy);
        assert!(r.settings.wifi_debug);
        assert!(r.settings.model_cache.is_none());
        assert!(r.disk.free_mb.is_some() || r.disk.notes.iter().any(|n| !n.is_empty()));
        let s = serde_json::to_string(&r).expect("json");
        assert!(!s.contains("screenshot"));
        assert!(!s.contains("bot_token"));
    }

    #[test]
    fn env_peer_and_hub() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("GSV_APK_PEER", "  phone-x  ");
        std::env::set_var("GSV_HUB_URL", "http://192.168.2.238:9999/");
        assert_eq!(peer_id(), "phone-x");
        assert_eq!(hub_from_env(), "http://192.168.2.238:9999");
        std::env::remove_var("GSV_APK_PEER");
        std::env::remove_var("GSV_HUB_URL");
    }

    #[test]
    fn token_header_is_edge_header() {
        assert_eq!(token_header(), "x-gsv-edge-token");
    }

    #[test]
    fn telegram_forwards_to_apk_not_tensor() {
        assert!(!telegram_may_be_phone_worker());
        assert!(is_phone_worker_origin(ORIGIN));
        assert!(!is_phone_worker_origin(TELEGRAM_ORIGIN));
        let auth = telegram_passthrough("bind").expect("auth");
        assert_eq!(auth.to, "apk");
        assert_eq!(auth.origin, ORIGIN);
        assert_eq!(auth.action, "auth");
        let open = telegram_passthrough("startapp").expect("open");
        assert_eq!(open.action, "open_apk");
        let shell = telegram_passthrough("dashboard").expect("shell");
        assert_eq!(shell.to, "shell");
        assert_eq!(shell.action, "chrome");
        assert!(telegram_passthrough("tensor").is_err());
        assert!(telegram_passthrough("host-tests").is_err());
        assert!(telegram_passthrough("kvm").is_err());
        assert!(telegram_passthrough("start-worker").is_err());
        assert!(telegram_passthrough("wllama").is_err());
        assert!(telenetis_may_grow("identity").is_ok());
        assert!(telenetis_may_grow("chrome").is_ok());
        assert!(telenetis_may_grow("tensor").is_err());
        assert!(telenetis_may_grow("swarm").is_err());
        assert!(telenetis_may_grow("kvm").is_err());
        let w = telenetis_surface_wire();
        assert_eq!(w["surface"], "shell");
        assert_eq!(w["phone_worker"], ORIGIN);
    }
}
