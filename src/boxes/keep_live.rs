//! Keep-live health aggregation — GSV + Telenetis + llama-rs + OmniRoute.
//!
//! Band 223 is **aggregation only** (no respawn). Probes each peer with 1s
//! timeout and exposes the result on Galaxy + `/api/health` + MCP.
//! `ok` stays true when a sub-service is down (like `disk_ok` band 181).

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// One peer entry in the keep-live report.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KeepLiveEntry {
    pub alive: bool,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lag: Option<bool>,
    /// Probe latency in ms (HTTP peers only; llama_rs file probe has none).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
    /// Peer uptime in seconds when the probe body reports it (gsv self-probe
    /// is TCP-only, so the server injects its own `started_at` uptime).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uptime_secs: Option<u64>,
}

/// Full keep-live report (4 peers).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KeepLiveReport {
    pub gsv: KeepLiveEntry,
    pub telenetis: KeepLiveEntry,
    pub llama_rs: KeepLiveEntry,
    pub omniroute: KeepLiveEntry,
}

/// Default URLs (overridable via env for tests).
pub fn gsv_url() -> String {
    std::env::var("GSV_KEEP_LIVE_GSV_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:9999/api/health".into())
}
pub fn telenetis_url() -> String {
    std::env::var("GSV_KEEP_LIVE_TELENETIS_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:9800/health".into())
}
pub fn omniroute_url() -> String {
    std::env::var("OMNIROUTE_URL")
        .or_else(|_| std::env::var("GSV_KEEP_LIVE_OMNIROUTE_URL"))
        .unwrap_or_else(|_| "http://127.0.0.1:3000".into())
}
pub fn llama_heartbeat_path() -> PathBuf {
    std::env::var("LLAMA_HEARTBEAT_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("S:/rust/llama-rs/target/live/llama_heartbeat.json"))
}

/// Llama heartbeat file shape (written by llama-rs when GSV_LIVE=1).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct LlamaHeartbeat {
    pid: u32,
    #[serde(default)]
    model: String,
    epoch_secs: u64,
    #[serde(default)]
    bin_version: String,
}

/// Check if a heartbeat file is fresh (age <= 60s).
pub fn heartbeat_fresh(path: &Path, now: u64) -> bool {
    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(_) => return false,
    };
    let hb: LlamaHeartbeat = match serde_json::from_str(&text) {
        Ok(h) => h,
        Err(_) => return false,
    };
    now.saturating_sub(hb.epoch_secs) <= 60
}

/// Probe result: alive + version + latency + optional uptime parsed from body.
#[derive(Debug, Clone, Default)]
pub struct Probe {
    pub alive: bool,
    pub version: Option<String>,
    pub latency_ms: u64,
    pub uptime_secs: Option<u64>,
}

/// Probe an HTTP URL with 1s timeout.
/// Version is extracted from JSON `version` or `crate_version` if present;
/// uptime from `uptime_secs` when the peer body carries it.
pub fn probe_http_blocking(url: &str) -> Probe {
    let t0 = std::time::Instant::now();
    // Use raw TcpStream + minimal HTTP to avoid pulling tokio into the box unit tests.
    // For the wire (axum) we have an async version `probe_http` below.
    // Here we do a best-effort blocking probe with 1s timeout.
    let parsed = match url
        .strip_prefix("http://")
        .or_else(|| url.strip_prefix("https://"))
    {
        Some(rest) => rest,
        None => return Probe::default(),
    };
    let host_port = parsed.split('/').next().unwrap_or(parsed);
    let path = format!("/{}", parsed.split_once('/').map(|x| x.1).unwrap_or(""));
    let addr = if host_port.contains(':') {
        host_port.to_string()
    } else {
        format!("{host_port}:80")
    };
    let timeout = std::time::Duration::from_millis(800);
    let mut stream = match std::net::TcpStream::connect_timeout(
        &addr
            .parse()
            .unwrap_or_else(|_| "127.0.0.1:80".parse().unwrap()),
        timeout,
    ) {
        Ok(s) => s,
        Err(_) => return Probe::default(),
    };
    let _ = stream.set_read_timeout(Some(timeout));
    let _ = stream.set_write_timeout(Some(timeout));
    let req = format!("GET {path} HTTP/1.0\r\nHost: {host_port}\r\nConnection: close\r\n\r\n");
    use std::io::{Read, Write};
    if stream.write_all(req.as_bytes()).is_err() {
        return Probe::default();
    }
    let mut buf = [0u8; 4096];
    let n = match stream.read(&mut buf) {
        Ok(n) => n,
        Err(_) => return Probe::default(),
    };
    let text = String::from_utf8_lossy(&buf[..n]);
    let alive = text.contains("200")
        && (text.contains("\"ok\":true")
            || text.contains("\"ok\": true")
            || text.contains("\"status\":\"ok\"")
            || text.contains("\"status\": \"ok\""));
    // Try to extract version from body after \r\n\r\n
    let body = text.split("\r\n\r\n").nth(1).unwrap_or("");
    let (version, uptime_secs) = parse_body(body);
    Probe {
        alive,
        version,
        latency_ms: t0.elapsed().as_millis() as u64,
        uptime_secs,
    }
}

/// Async probe for the axum wire (1s timeout via reqwest).
pub async fn probe_http(url: &str) -> Probe {
    let t0 = std::time::Instant::now();
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(1))
        .build()
    {
        Ok(c) => c,
        Err(_) => return Probe::default(),
    };
    let resp = match client.get(url).send().await {
        Ok(r) => r,
        Err(_) => return Probe::default(),
    };
    let status = resp.status().as_u16();
    let text = resp.text().await.unwrap_or_default();
    let alive = status == 200
        && (text.contains("\"ok\":true")
            || text.contains("\"ok\": true")
            || text.contains("\"status\":\"ok\"")
            || text.contains("\"status\": \"ok\""));
    let (version, uptime_secs) = parse_body(&text);
    Probe {
        alive,
        version,
        latency_ms: t0.elapsed().as_millis() as u64,
        uptime_secs,
    }
}

/// Extract `version`/`crate_version` and optional `uptime_secs` from a JSON body.
fn parse_body(body: &str) -> (Option<String>, Option<u64>) {
    let v: Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(_) => return (None, None),
    };
    let version = v
        .get("version")
        .or_else(|| v.get("crate_version"))
        .and_then(|x| x.as_str())
        .map(|s| s.to_string());
    let uptime_secs = v
        .get("uptime_secs")
        .or_else(|| v.get("uptime"))
        .and_then(|x| x.as_u64());
    (version, uptime_secs)
}

/// Build a report by probing all 4 peers.
/// `ok` is always true (like disk_ok) — sub-service down does not make GSV `ok:false`.
pub fn report() -> KeepLiveReport {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // gsv probe is TCP to avoid recursion (health includes keep_live).
    // Respect GSV_KEEP_LIVE_GSV_URL override for tests (fail-open).
    let gsv_url_str = gsv_url();
    let (gsv_alive, gsv_ver, gsv_lat) = if gsv_url_str == "http://127.0.0.1:9999/api/health" {
        let t0 = std::time::Instant::now();
        let alive = std::net::TcpStream::connect_timeout(
            &"127.0.0.1:9999".parse().unwrap(),
            std::time::Duration::from_millis(500),
        )
        .is_ok();
        let ver = crate::boxes::update::crate_version(&PathBuf::from(env!("CARGO_MANIFEST_DIR")));
        (alive, ver, t0.elapsed().as_millis() as u64)
    } else {
        let p = probe_http_blocking(&gsv_url_str);
        (p.alive, p.version, p.latency_ms)
    };
    let tel = probe_http_blocking(&telenetis_url());
    let omni = probe_http_blocking(&omniroute_url());
    let llama_alive = heartbeat_fresh(&llama_heartbeat_path(), now);
    KeepLiveReport {
        gsv: KeepLiveEntry {
            alive: gsv_alive,
            url: gsv_url_str.clone(),
            version: gsv_ver,
            lag: None,
            latency_ms: Some(gsv_lat),
            uptime_secs: None,
        },
        telenetis: KeepLiveEntry {
            alive: tel.alive,
            url: telenetis_url(),
            version: tel.version,
            lag: None,
            latency_ms: Some(tel.latency_ms),
            uptime_secs: tel.uptime_secs,
        },
        llama_rs: KeepLiveEntry {
            alive: llama_alive,
            url: llama_heartbeat_path().to_string_lossy().to_string(),
            version: None,
            lag: None,
            latency_ms: None,
            uptime_secs: None,
        },
        omniroute: KeepLiveEntry {
            alive: omni.alive,
            url: omniroute_url(),
            version: None,
            lag: None,
            latency_ms: Some(omni.latency_ms),
            uptime_secs: omni.uptime_secs,
        },
    }
}

/// Async version for the axum wire.
pub async fn report_async() -> KeepLiveReport {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let gsv_url_str = gsv_url();
    let (gsv_alive, gsv_ver, gsv_lat) = if gsv_url_str == "http://127.0.0.1:9999/api/health" {
        let t0 = std::time::Instant::now();
        let alive = tokio::time::timeout(
            std::time::Duration::from_millis(500),
            tokio::net::TcpStream::connect("127.0.0.1:9999"),
        )
        .await
        .is_ok_and(|r| r.is_ok());
        let ver = crate::boxes::update::crate_version(&PathBuf::from(env!("CARGO_MANIFEST_DIR")));
        (alive, ver, t0.elapsed().as_millis() as u64)
    } else {
        let p = probe_http(&gsv_url_str).await;
        (p.alive, p.version, p.latency_ms)
    };
    let tel = probe_http(&telenetis_url()).await;
    let omni = probe_http(&omniroute_url()).await;
    let llama_alive = heartbeat_fresh(&llama_heartbeat_path(), now);
    KeepLiveReport {
        gsv: KeepLiveEntry {
            alive: gsv_alive,
            url: gsv_url_str.clone(),
            version: gsv_ver,
            lag: None,
            latency_ms: Some(gsv_lat),
            uptime_secs: None,
        },
        telenetis: KeepLiveEntry {
            alive: tel.alive,
            url: telenetis_url(),
            version: tel.version,
            lag: None,
            latency_ms: Some(tel.latency_ms),
            uptime_secs: tel.uptime_secs,
        },
        llama_rs: KeepLiveEntry {
            alive: llama_alive,
            url: llama_heartbeat_path().to_string_lossy().to_string(),
            version: None,
            lag: None,
            latency_ms: None,
            uptime_secs: None,
        },
        omniroute: KeepLiveEntry {
            alive: omni.alive,
            url: omniroute_url(),
            version: None,
            lag: None,
            latency_ms: Some(omni.latency_ms),
            uptime_secs: omni.uptime_secs,
        },
    }
}

/// One-line keep-live hint, e.g. "gsv/telenetis up · llama_rs/omniroute down".
pub fn hint(r: &KeepLiveReport) -> String {
    let mut up = Vec::new();
    let mut down = Vec::new();
    for (name, alive) in [
        ("gsv", r.gsv.alive),
        ("telenetis", r.telenetis.alive),
        ("llama_rs", r.llama_rs.alive),
        ("omniroute", r.omniroute.alive),
    ] {
        if alive {
            up.push(name);
        } else {
            down.push(name);
        }
    }
    if down.is_empty() {
        format!("{} up", up.join("/"))
    } else if up.is_empty() {
        format!("{} down", down.join("/"))
    } else {
        format!("{} up · {} down", up.join("/"), down.join("/"))
    }
}

/// Wire for GET /api/keep-live (ok always true).
pub fn wire() -> Value {
    let r = report();
    json!({
        "ok": true,
        "hint": hint(&r),
        "gsv": r.gsv,
        "telenetis": r.telenetis,
        "llama_rs": r.llama_rs,
        "omniroute": r.omniroute,
    })
}

/// Async wire for axum.
pub async fn wire_async() -> Value {
    let r = report_async().await;
    json!({
        "ok": true,
        "hint": hint(&r),
        "gsv": r.gsv,
        "telenetis": r.telenetis,
        "llama_rs": r.llama_rs,
        "omniroute": r.omniroute,
    })
}

/// Inject the server's own uptime into the GSV entry (self-probe is TCP-only).
pub fn stub_gsv_uptime(v: &mut Value, uptime_secs: u64) {
    v["gsv"]["uptime_secs"] = json!(uptime_secs);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn heartbeat_fresh_true_and_false() {
        let dir = std::env::temp_dir().join(format!("gsv-keep-live-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("hb.json");
        let now = 1_700_000_000u64;
        let fresh = LlamaHeartbeat {
            pid: 123,
            model: "test".into(),
            epoch_secs: now - 30,
            bin_version: "0.222.2".into(),
        };
        fs::write(&path, serde_json::to_string(&fresh).unwrap()).unwrap();
        assert!(heartbeat_fresh(&path, now));
        let stale = LlamaHeartbeat {
            pid: 123,
            model: "test".into(),
            epoch_secs: now - 61,
            bin_version: "0.222.2".into(),
        };
        fs::write(&path, serde_json::to_string(&stale).unwrap()).unwrap();
        assert!(!heartbeat_fresh(&path, now));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn heartbeat_missing_is_not_alive() {
        let p = Path::new("/tmp/gsv-keep-live-missing-12345.json");
        assert!(!heartbeat_fresh(p, 1_700_000_000));
    }

    #[test]
    fn probe_http_blocking_down_is_not_alive() {
        // Use a port that is not listening (fail-open).
        let p = probe_http_blocking("http://127.0.0.1:59999/health");
        assert!(!p.alive);
    }

    #[test]
    fn report_ok_stays_true_when_peers_down() {
        let _guard = crate::boxes::ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        // Point all peers to a non-listening port, report should still be ok:true via wire().
        std::env::set_var("GSV_KEEP_LIVE_GSV_URL", "http://127.0.0.1:59998/api/health");
        std::env::set_var(
            "GSV_KEEP_LIVE_TELENETIS_URL",
            "http://127.0.0.1:59997/health",
        );
        std::env::set_var("GSV_KEEP_LIVE_OMNIROUTE_URL", "http://127.0.0.1:59996");
        std::env::set_var(
            "LLAMA_HEARTBEAT_PATH",
            "/tmp/gsv-keep-live-missing-99999.json",
        );
        let v = wire();
        assert_eq!(v["ok"], true);
        assert_eq!(v["gsv"]["alive"], false);
        assert_eq!(v["telenetis"]["alive"], false);
        assert_eq!(v["llama_rs"]["alive"], false);
        assert_eq!(v["omniroute"]["alive"], false);
        std::env::remove_var("GSV_KEEP_LIVE_GSV_URL");
        std::env::remove_var("GSV_KEEP_LIVE_TELENETIS_URL");
        std::env::remove_var("GSV_KEEP_LIVE_OMNIROUTE_URL");
        std::env::remove_var("LLAMA_HEARTBEAT_PATH");
    }

    #[test]
    fn hint_lists_up_and_down_peers() {
        let mk = |alive: bool| KeepLiveEntry {
            alive,
            url: "http://127.0.0.1:1".into(),
            version: None,
            lag: None,
            latency_ms: Some(1),
            uptime_secs: None,
        };
        let all_up = KeepLiveReport {
            gsv: mk(true),
            telenetis: mk(true),
            llama_rs: mk(true),
            omniroute: mk(true),
        };
        assert_eq!(hint(&all_up), "gsv/telenetis/llama_rs/omniroute up");
        let mixed = KeepLiveReport {
            gsv: mk(true),
            telenetis: mk(false),
            llama_rs: mk(true),
            omniroute: mk(false),
        };
        assert_eq!(hint(&mixed), "gsv/llama_rs up · telenetis/omniroute down");
    }

    #[test]
    fn stub_gsv_uptime_injects_server_uptime() {
        let mut v = serde_json::json!({ "gsv": { "alive": true } });
        stub_gsv_uptime(&mut v, 77);
        assert_eq!(v["gsv"]["uptime_secs"], 77);
    }
}
