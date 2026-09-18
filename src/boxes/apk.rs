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
use crate::net;
use std::path::{Path, PathBuf};

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
/// Android package id (generated manifest; not a Java/gradle product).
pub const PACKAGE_ID: &str = "org.gsv.apk";
/// Launcher label.
pub const PACKAGE_LABEL: &str = "GSV";
/// Redmi 9 is Android 10; 24 covers 7+.
pub const MIN_SDK: u32 = 24;
pub const TARGET_SDK: u32 = 34;
/// Pipeline output name under `target/live/apk/` (not product source).
pub const MANIFEST_NAME: &str = "AndroidManifest.xml";

const NATIVE_FORBID: &[&str] = &[
    "webview",
    "chrome",
    "mini-app",
    "miniapp",
    "tensor",
    "webgpu",
    "kvm",
    "board",
    "bot",
    "host-tests",
    "start-worker",
    "wllama",
    "gguf",
    "screenshot",
];

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

/// One LAN join hop through hub `/api/edge` (no secrets).
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct JoinStep {
    pub method: &'static str,
    pub path: String,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<Value>,
}

/// Phone LAN address advertised in `register-remote`.
/// `GSV_APK_ADDR` wins; otherwise [`net::local_addr`] (loopback under cargo test).
pub fn peer_addr() -> String {
    std::env::var("GSV_APK_ADDR")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(net::local_addr)
}

/// GET health, then POST register-remote + pool/join. Never `:8091`.
pub fn join_steps(
    hub: &str,
    peer: &str,
    addr: &str,
    disk: &DiskReport,
) -> Result<Vec<JoinStep>, HubReject> {
    check_hub(hub)?;
    let health = edge_url(hub, "health")?;
    let register = edge_url(hub, "discovery/register-remote")?;
    let join_path = format!("virtual-nodes/{peer}/pool/join");
    let join = edge_url(hub, &join_path)?;
    Ok(vec![
        JoinStep {
            method: "GET",
            path: "health".into(),
            url: health,
            body: None,
        },
        JoinStep {
            method: "POST",
            path: "discovery/register-remote".into(),
            url: register,
            body: Some(registration_body(peer, addr, 0, disk)),
        },
        JoinStep {
            method: "POST",
            path: join_path,
            url: join,
            body: Some(pool_join_body()),
        },
    ])
}

fn token_present(token: Option<&str>) -> bool {
    token.map(str::trim).is_some_and(|s| !s.is_empty())
}

/// Redacted join wire. Never includes the edge token value.
pub fn join_wire(dry: bool, token: Option<&str>, steps: &[JoinStep], replies: &[Value]) -> Value {
    json!({
        "ok": true,
        "dry_run": dry,
        "origin": ORIGIN,
        "role": ROLE,
        "class": CLASS_EDGE,
        "token_header": token_header(),
        "token_set": token_present(token),
        "steps": steps,
        "replies": replies,
    })
}

/// LAN peer join. Default `dry` plans the hops and opens no sockets.
/// Live POSTs `X-Gsv-Edge-Token` and redacts upstream JSON. Never `:8091`.
pub async fn join_lan(
    hub: &str,
    peer: &str,
    addr: &str,
    disk: &DiskReport,
    dry: bool,
    token: Option<&str>,
) -> Result<Value, HubReject> {
    let steps = join_steps(hub, peer, addr, disk)?;
    if dry {
        return Ok(join_wire(true, token, &steps, &[]));
    }
    if !token_present(token) {
        return Err(HubReject {
            error: "edge token not set",
        });
    }
    let tok = token.map(str::trim).unwrap_or("");
    let client = reqwest::Client::builder()
        .timeout(edge::FORWARD_TIMEOUT)
        .no_proxy()
        .build()
        .map_err(|_| HubReject {
            error: "http client",
        })?;
    let mut replies = Vec::new();
    for step in &steps {
        let mut req = match step.method {
            "GET" => client.get(&step.url),
            _ => client.post(&step.url),
        };
        req = req.header(token_header(), tok);
        if let Some(body) = &step.body {
            req = req.json(body);
        }
        let resp = match req.send().await {
            Ok(r) => r,
            Err(_) => {
                return Err(HubReject {
                    error: "hub unreachable",
                });
            }
        };
        let status = resp.status().as_u16();
        let raw: Value = resp.json().await.unwrap_or_else(|_| json!({ "ok": false }));
        let body = edge::redact_json(&raw);
        replies.push(json!({ "status": status, "body": body }));
        if status >= 400 {
            let mut out = join_wire(false, token, &steps, &replies);
            out["ok"] = json!(false);
            out["error"] = json!("hub join failed");
            return Ok(out);
        }
    }
    Ok(join_wire(false, token, &steps, &replies))
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
            model_cache: model_cache(),
        },
        hub,
        identity: id,
        disk,
    }
}

/// Hub URL the APK / host report advertises (`http://<local>:9999`).
pub fn hub_url() -> String {
    format!(
        "http://{}:{}",
        crate::net::local_addr(),
        crate::DEFAULT_PORT
    )
}

/// Optional on-device model cache path (`GSV_APK_CACHE`). Native APK does not use IDB.
pub fn model_cache() -> Option<String> {
    std::env::var("GSV_APK_CACHE")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Wireless ADB serial (`host:port`) from `GSV_APK_ADB`. Empty until paired.
pub fn adb_serial() -> String {
    std::env::var("GSV_APK_ADB")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_default()
}

/// Host ADB binary (WiFi debug). Never a screenshot tool.
pub fn adb_bin(repo_root: &Path) -> PathBuf {
    let name = if cfg!(windows) { "adb.exe" } else { "adb" };
    repo_root
        .join("target")
        .join("adb")
        .join("platform-tools")
        .join(name)
}

/// WiFi-debug ADB verbs. `screencap` / `screenshot` are never allowed.
pub const ADB_ALLOW: &[&str] = &[
    "devices",
    "connect",
    "pair",
    "disconnect",
    "logcat",
    "shell",
    "push",
    "pull",
    "reverse",
];

/// True when `verb` is a WiFi-debug hop (pair/connect/logcat/df). No screenshot loops.
pub fn adb_ok(verb: &str) -> Result<(), HubReject> {
    let lower = verb.trim().to_ascii_lowercase();
    if lower.is_empty() {
        return Err(HubReject {
            error: "adb verb empty",
        });
    }
    if lower.contains("screencap") || lower.contains("screenshot") {
        return Err(HubReject {
            error: "adb forbids screenshot loops",
        });
    }
    let head = lower.split_whitespace().next().unwrap_or("");
    if ADB_ALLOW.contains(&head) {
        Ok(())
    } else {
        Err(HubReject {
            error: "adb verb not on wifi-debug allowlist",
        })
    }
}

/// Planned ADB hops (no sockets). Hub reads JSON; do not pull PNGs.
pub fn adb_plan(repo_root: &Path, serial: &str) -> Value {
    let bin = adb_bin(repo_root);
    json!({
        "ok": true,
        "wifi_debug": true,
        "screenshot": false,
        "kvm": false,
        "bin": bin.display().to_string(),
        "bin_exists": bin.is_file(),
        "serial": serial,
        "verbs": ADB_ALLOW,
        "forbid": ["screencap", "screenshot"],
        "hops": [
            {"verb": "devices", "argv": ["devices"]},
            {"verb": "shell", "argv": ["shell", "df", "-h", "/data"]},
            {"verb": "logcat", "argv": ["logcat", "-d", "-t", "20", "GSV:I", "*:S"]},
        ],
    })
}

/// Compact health line (no secrets, no screenshots).
pub fn health_wire(repo_root: &Path) -> Value {
    let r = report(repo_root, &hub_url(), &peer_id());
    json!({
        "ok": r.ok,
        "wifi_debug": true,
        "screenshot": false,
        "kvm": false,
        "package": PACKAGE_ID,
        "origin": ORIGIN,
        "disk_ok": r.disk.ok,
        "free_mb": r.disk.free_mb,
        "model_cache_set": r.settings.model_cache.is_some(),
    })
}

/// Hub-readable disk + settings + ADB plan. Replaces screenshot loops.
pub fn disk_settings_wire(repo_root: &Path) -> Value {
    let r = report(repo_root, &hub_url(), &peer_id());
    json!({
        "ok": r.ok,
        "origin": ORIGIN,
        "role": ROLE,
        "class": CLASS_EDGE,
        "package": PACKAGE_ID,
        "wifi_debug": true,
        "screenshot": false,
        "kvm": false,
        "hub": r.hub,
        "peer_id": r.identity.peer_id,
        "disk": r.disk,
        "settings": r.settings,
        "identity": r.identity,
        "adb": adb_plan(repo_root, &adb_serial()),
    })
}

/// Surfaces allowed inside the native APK (join / disk / settings / wifi-debug).
pub fn native_ok(kind: &str) -> Result<(), HubReject> {
    let k = canon_kind(kind);
    if k.is_empty() {
        return Err(HubReject {
            error: "native kind empty",
        });
    }
    if NATIVE_FORBID.iter().any(|f| k == *f || k.contains(f)) {
        return Err(HubReject {
            error: "native apk forbids webview/board/bot/kvm/tensor",
        });
    }
    match k.as_str() {
        "join" | "register" | "disk" | "settings" | "wifi-debug" | "adb" | "health" | "auth"
        | "command" | "open_apk" | "lan" => Ok(()),
        _ => Err(HubReject {
            error: "native apk forbids webview/board/bot/kvm/tensor",
        }),
    }
}

/// Android process entry: same LAN join as `gsv-apk join`. Never a WebView.
pub fn android_entry() -> &'static str {
    "join_lan"
}

/// Native package contract (no secrets, no gradle/Java product files).
pub fn native_package_wire() -> Value {
    json!({
        "ok": true,
        "native": true,
        "webview": false,
        "chrome": false,
        "mini_app": false,
        "package": PACKAGE_ID,
        "label": PACKAGE_LABEL,
        "min_sdk": MIN_SDK,
        "target_sdk": TARGET_SDK,
        "origin": ORIGIN,
        "role": ROLE,
        "class": CLASS_EDGE,
        "hub": "/api/edge",
        "entry": android_entry(),
        "activities": ["join", "disk", "settings"],
        "forbid": NATIVE_FORBID,
        "gradle": false,
        "java": false,
        "manifest": MANIFEST_NAME,
    })
}

/// AndroidManifest.xml generated from Rust. Pipeline output, not product source.
pub fn android_manifest() -> String {
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
    package="{PACKAGE_ID}">
    <uses-sdk android:minSdkVersion="{MIN_SDK}" android:targetSdkVersion="{TARGET_SDK}" />
    <uses-permission android:name="android.permission.INTERNET" />
    <uses-permission android:name="android.permission.ACCESS_NETWORK_STATE" />
    <application android:label="{PACKAGE_LABEL}" android:usesCleartextTraffic="true">
        <activity android:name="{PACKAGE_ID}.JoinActivity" android:exported="true">
            <intent-filter>
                <action android:name="android.intent.action.MAIN" />
                <category android:name="android.intent.category.LAUNCHER" />
            </intent-filter>
        </activity>
    </application>
</manifest>
"#
    )
}

/// Write the generated manifest under `dir` (typically `target/live/apk`).
pub fn write_manifest(dir: &Path) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(dir)?;
    let path = dir.join(MANIFEST_NAME);
    std::fs::write(&path, android_manifest())?;
    Ok(path)
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
        std::env::set_var("GSV_APK_ADDR", "  192.168.2.89  ");
        std::env::set_var("GSV_APK_CACHE", "  /data/gsv/models  ");
        std::env::set_var("GSV_APK_ADB", "  192.168.2.89:46147  ");
        assert_eq!(peer_id(), "phone-x");
        assert_eq!(hub_from_env(), "http://192.168.2.238:9999");
        assert_eq!(peer_addr(), "192.168.2.89");
        assert_eq!(model_cache().as_deref(), Some("/data/gsv/models"));
        assert_eq!(adb_serial(), "192.168.2.89:46147");
        std::env::remove_var("GSV_APK_PEER");
        std::env::remove_var("GSV_HUB_URL");
        std::env::remove_var("GSV_APK_ADDR");
        std::env::remove_var("GSV_APK_CACHE");
        std::env::remove_var("GSV_APK_ADB");
    }

    #[test]
    fn join_steps_are_hub_edge_not_8091() {
        let disk = xtask::disk_report(&root(), false);
        let steps = join_steps(DEFAULT_HUB, "redmi-01", "192.168.2.89", &disk).expect("steps");
        assert_eq!(steps.len(), 3);
        assert_eq!(steps[0].method, "GET");
        assert_eq!(steps[0].path, "health");
        assert_eq!(steps[1].path, "discovery/register-remote");
        assert_eq!(steps[2].path, "virtual-nodes/redmi-01/pool/join");
        for s in &steps {
            assert!(s.url.contains("/api/edge/"), "{}", s.url);
            assert!(!s.url.contains("8091"), "{}", s.url);
            assert!(!s.url.contains(":9800"), "{}", s.url);
        }
        let origin = steps[1].body.as_ref().unwrap()["metadata"]["origin"].as_str();
        assert_eq!(origin, Some(ORIGIN));
        let wire = join_wire(true, Some("edge-secret-must-not-leak"), &steps, &[]);
        let raw = wire.to_string();
        assert_eq!(wire["dry_run"], true);
        assert_eq!(wire["token_set"], true);
        assert!(!raw.contains("edge-secret-must-not-leak"), "{raw}");
        assert!(!raw.contains("8091"), "{raw}");
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

    #[test]
    fn native_package_is_not_webview() {
        assert_eq!(android_entry(), "join_lan");
        assert!(native_ok("join").is_ok());
        assert!(native_ok("disk").is_ok());
        assert!(native_ok("settings").is_ok());
        assert!(native_ok("wifi-debug").is_ok());
        assert!(native_ok("webview").is_err());
        assert!(native_ok("Chrome").is_err());
        assert!(native_ok("mini-app").is_err());
        assert!(native_ok("tensor").is_err());
        assert!(native_ok("board").is_err());
        assert!(native_ok("kvm").is_err());
        let w = native_package_wire();
        assert_eq!(w["native"], true);
        assert_eq!(w["webview"], false);
        assert_eq!(w["chrome"], false);
        assert_eq!(w["mini_app"], false);
        assert_eq!(w["gradle"], false);
        assert_eq!(w["java"], false);
        assert_eq!(w["package"], PACKAGE_ID);
        assert_eq!(w["entry"], "join_lan");
        let xml = android_manifest();
        assert!(xml.contains(PACKAGE_ID));
        assert!(xml.contains("INTERNET"));
        assert!(!xml.to_ascii_lowercase().contains("webview"));
        assert!(!xml.contains("8091"));
        assert!(!xml.contains("screenshot"));
        let dir = std::env::temp_dir().join(format!("gsv-apk-manifest-{}", std::process::id()));
        let path = write_manifest(&dir).expect("write");
        assert_eq!(
            path.file_name().and_then(|n| n.to_str()),
            Some(MANIFEST_NAME)
        );
        let on_disk = std::fs::read_to_string(&path).expect("read");
        assert_eq!(on_disk, xml);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn wifi_debug_disk_settings_not_screenshots() {
        assert!(adb_ok("devices").is_ok());
        assert!(adb_ok("pair").is_ok());
        assert!(adb_ok("connect").is_ok());
        assert!(adb_ok("shell").is_ok());
        assert!(adb_ok("logcat").is_ok());
        assert!(adb_ok("screencap").is_err());
        assert!(adb_ok("exec-out screencap").is_err());
        assert!(adb_ok("screenshot").is_err());
        let plan = adb_plan(&root(), "192.168.2.89:46147");
        assert_eq!(plan["wifi_debug"], true);
        assert_eq!(plan["screenshot"], false);
        assert_eq!(plan["kvm"], false);
        let hops = plan["hops"].as_array().expect("hops");
        assert!(hops.iter().any(|h| h["verb"] == "shell"));
        let hops_s = serde_json::to_string(&plan["hops"]).expect("hops json");
        assert!(!hops_s.contains("screencap"), "{hops_s}");
        let w = disk_settings_wire(&root());
        assert_eq!(w["wifi_debug"], true);
        assert_eq!(w["screenshot"], false);
        assert_eq!(w["kvm"], false);
        assert_eq!(w["settings"]["wifi_debug"], true);
        let hops_s = serde_json::to_string(&w["adb"]["hops"]).expect("adb hops");
        assert!(!hops_s.contains("screencap"), "{hops_s}");
        let s = w.to_string();
        assert!(!s.contains("8091"));
        assert!(!s.contains("bot_token"));
        let h = health_wire(&root());
        assert_eq!(h["screenshot"], false);
        assert_eq!(h["wifi_debug"], true);
    }
}
