//! Hub single-entry proxy for the poolAI edge plane (band 237).
//!
//! poolAI's discovery/jobs/virtual-nodes/grid surface has no JWT and no
//! attached rate limit. Clients must not talk to `:8091` off-box. This box
//! fronts an allowlisted reverse proxy on GSV `:9999`:
//!
//! - per-call token (`GSV_EDGE_TOKEN` env wins, else `settings.edge.token`)
//! - 20 req / 1s window per token
//! - path allowlist (edge plane only — no login / vm / admin)
//! - JSON key redaction on the way out
//!
//! Status `GET /api/edge` is open (redacted). Forwards require a token.
//! Canon: [`GSV_VDC.md`](../../docs/gsv/GSV_VDC.md) §6.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use crate::boxes::grid;
use crate::boxes::settings::{self, SettingsFile};

/// Env that overrides a file token without being persisted.
pub const TOKEN_ENV: &str = crate::boxes::settings::EDGE_TOKEN_ENV;
/// Header alias (also `Authorization: Bearer`).
pub const TOKEN_HEADER: &str = "x-gsv-edge-token";
/// Requests allowed per [`RATE_WINDOW`].
pub const RATE_CAP: u32 = 20;
/// Rate-limit window.
pub const RATE_WINDOW: Duration = Duration::from_secs(1);
/// Upstream HTTP timeout for a forwarded call.
pub const FORWARD_TIMEOUT: Duration = Duration::from_secs(5);

/// Edge-plane prefixes (relative to poolAI `/api/v1`).
pub const ALLOW_PREFIXES: &[&str] = &[
    "topology",
    "workers",
    "discovery",
    "virtual-nodes",
    "grid",
    "jobs",
    "health",
];

/// Secret-shaped JSON keys stripped from upstream bodies.
const REDACT_KEYS: &[&str] = &[
    "token",
    "bot_token",
    "password",
    "pass",
    "secret",
    "admin_password",
    "api_key",
    "authorization",
];

/// Rejection from [`gate`] (HTTP status + error; never a secret).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reject {
    pub status: u16,
    pub error: String,
}

struct RateInner {
    hits: HashMap<String, (Instant, u32)>,
}

impl RateInner {
    fn new() -> Self {
        Self {
            hits: HashMap::new(),
        }
    }
}

fn rate() -> MutexGuard<'static, RateInner> {
    static RATE: OnceLock<Mutex<RateInner>> = OnceLock::new();
    RATE.get_or_init(|| Mutex::new(RateInner::new()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

/// Drop the in-memory rate window (tests).
pub fn clear_rate_limit() {
    rate().hits.clear();
}

/// Constant-time equality for equal-length secrets. Length mismatch is false.
pub fn token_eq(a: &str, b: &str) -> bool {
    let aa = a.as_bytes();
    let bb = b.as_bytes();
    if aa.len() != bb.len() || aa.is_empty() {
        return false;
    }
    aa.iter().zip(bb).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// Strip `/`, `api/v1/`, and reject `..`. Empty → None.
pub fn normalize_path(raw: &str) -> Option<String> {
    let mut p = raw.trim().trim_start_matches('/').replace('\\', "/");
    if p.contains("..") {
        return None;
    }
    if let Some(rest) = p.strip_prefix("api/v1/") {
        p = rest.to_string();
    } else if p == "api/v1" {
        return None;
    }
    let p = p.trim_start_matches('/').trim_end_matches('/');
    if p.is_empty() {
        None
    } else {
        Some(p.to_ascii_lowercase())
    }
}

/// First path segment of a normalized relative path.
fn first_seg(path: &str) -> &str {
    path.split('/').next().unwrap_or(path)
}

/// True when method + path is on the edge-plane allowlist.
pub fn allowlisted(method: &str, path: &str) -> bool {
    let Some(norm) = normalize_path(path) else {
        return false;
    };
    let m = method.trim().to_ascii_uppercase();
    if m != "GET" && m != "POST" {
        return false;
    }
    let head = first_seg(&norm);
    if !ALLOW_PREFIXES.contains(&head) {
        return false;
    }
    if head == "health" && m != "GET" {
        return false;
    }
    true
}

/// Non-empty `GSV_EDGE_TOKEN`, if set.
pub fn env_token() -> Option<String> {
    settings::env_edge_token()
}

/// Expected token: env wins over `settings.edge.token`.
pub fn expected_token(file: &SettingsFile, env: Option<&str>) -> String {
    if let Some(e) = env.map(str::trim).filter(|s| !s.is_empty()) {
        return e.to_string();
    }
    file.edge.token.trim().to_string()
}

/// Load expected token from disk + env (never echoed).
pub fn expected_token_for(data_dir: &Path) -> String {
    let file = settings::load_result(data_dir).unwrap_or_default();
    expected_token(&file, env_token().as_deref())
}

/// Bearer / `X-Gsv-Edge-Token` presentation.
pub fn presented_token(authorization: Option<&str>, header: Option<&str>) -> String {
    if let Some(h) = header.map(str::trim).filter(|s| !s.is_empty()) {
        return h.to_string();
    }
    let Some(auth) = authorization.map(str::trim).filter(|s| !s.is_empty()) else {
        return String::new();
    };
    auth.strip_prefix("Bearer ")
        .or_else(|| auth.strip_prefix("bearer "))
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("")
        .to_string()
}

/// Take one slot in the per-token window. False → over cap.
pub fn rate_ok(key: &str) -> bool {
    if key.is_empty() {
        return false;
    }
    let mut g = rate();
    let now = Instant::now();
    g.hits
        .retain(|_, (t, _)| now.duration_since(*t) < RATE_WINDOW);
    let entry = g.hits.entry(key.to_string()).or_insert((now, 0));
    if now.duration_since(entry.0) >= RATE_WINDOW {
        *entry = (now, 0);
    }
    if entry.1 >= RATE_CAP {
        return false;
    }
    entry.1 += 1;
    true
}

/// Recursively strip secret-shaped keys from JSON (id redaction, band-174 class).
pub fn redact_json(v: &Value) -> Value {
    match v {
        Value::Object(map) => {
            let mut out = serde_json::Map::new();
            for (k, val) in map {
                if REDACT_KEYS.iter().any(|rk| k.eq_ignore_ascii_case(rk)) {
                    continue;
                }
                out.insert(k.clone(), redact_json(val));
            }
            Value::Object(out)
        }
        Value::Array(items) => Value::Array(items.iter().map(redact_json).collect()),
        other => other.clone(),
    }
}

/// Redacted status wire (`token_set`, never the secret).
pub fn status_wire(data_dir: &Path) -> Value {
    let file = settings::load_result(data_dir).unwrap_or_default();
    let env = env_token();
    let env_set = env.as_deref().is_some_and(|s| !s.is_empty());
    let file_set = !file.edge.token.trim().is_empty();
    let source = if env_set {
        "env"
    } else if file_set {
        "file"
    } else {
        "none"
    };
    json!({
        "ok": true,
        "token_set": env_set || file_set,
        "source": source,
        "allowlist": ALLOW_PREFIXES,
        "rate": { "window_ms": RATE_WINDOW.as_millis() as u64, "cap": RATE_CAP },
        "poolai_base": grid::poolai_base(),
    })
}

/// Auth + allowlist + rate. Does not forward.
pub fn gate(method: &str, path: &str, presented: &str, expected: &str) -> Result<String, Reject> {
    let Some(norm) = normalize_path(path) else {
        return Err(Reject {
            status: 404,
            error: "path not on edge allowlist".into(),
        });
    };
    if !allowlisted(method, &norm) {
        return Err(Reject {
            status: 404,
            error: "path not on edge allowlist".into(),
        });
    }
    if expected.is_empty() {
        return Err(Reject {
            status: 503,
            error: "edge token unset".into(),
        });
    }
    if !token_eq(presented, expected) {
        return Err(Reject {
            status: 401,
            error: "edge token required".into(),
        });
    }
    if !rate_ok(presented) {
        return Err(Reject {
            status: 429,
            error: "rate limited".into(),
        });
    }
    Ok(norm)
}

fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(FORWARD_TIMEOUT)
        .no_proxy()
        .build()
        .unwrap_or_default()
}

/// Forward an already-gated relative path to poolAI. Body is JSON for POST.
pub async fn forward(
    base_url: &str,
    method: &str,
    rel_path: &str,
    body: Option<&Value>,
) -> Result<(u16, Value), String> {
    let url = format!(
        "{}/{}",
        base_url.trim_end_matches('/'),
        rel_path.trim_start_matches('/')
    );
    let client = http_client();
    let m = method.trim().to_ascii_uppercase();
    let req = match m.as_str() {
        "GET" => client.get(&url),
        "POST" => {
            let mut r = client.post(&url);
            if let Some(b) = body {
                r = r.json(b);
            }
            r
        }
        other => return Err(format!("method {other} not forwarded")),
    };
    let res = req.send().await.map_err(|e| e.to_string())?;
    let status = res.status().as_u16();
    let val = match res.json::<Value>().await {
        Ok(v) => redact_json(&v),
        Err(_) => json!({ "ok": false, "error": "non-json upstream" }),
    };
    Ok((status, val))
}

/// Gate then forward. `base_url` is the poolAI `/api/v1` root.
pub async fn dispatch(
    data_dir: &Path,
    base_url: &str,
    method: &str,
    path: &str,
    presented: &str,
    body: Option<&Value>,
) -> Result<(u16, Value), Reject> {
    let expected = expected_token_for(data_dir);
    let norm = gate(method, path, presented, &expected)?;
    match forward(base_url, method, &norm, body).await {
        Ok(pair) => Ok(pair),
        Err(e) => Err(Reject {
            status: 502,
            error: format!("upstream: {e}"),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boxes::settings::EdgeSettings;
    use std::path::PathBuf;

    fn temp(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "gsv-edge-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::create_dir_all(&dir);
        dir
    }

    #[test]
    fn normalize_strips_api_v1_and_rejects_dotdot() {
        assert_eq!(
            normalize_path("/api/v1/topology/nodes").as_deref(),
            Some("topology/nodes")
        );
        assert_eq!(normalize_path("Workers").as_deref(), Some("workers"));
        assert_eq!(normalize_path("../etc/passwd"), None);
        assert_eq!(normalize_path(""), None);
        assert_eq!(normalize_path("/api/v1/"), None);
    }

    #[test]
    fn allowlist_edge_plane_only() {
        assert!(allowlisted("GET", "topology/nodes"));
        assert!(allowlisted("POST", "virtual-nodes/a54-01/tasks"));
        assert!(allowlisted("GET", "health"));
        assert!(!allowlisted("POST", "health"));
        assert!(!allowlisted("GET", "login"));
        assert!(!allowlisted("POST", "vm/instances"));
        assert!(!allowlisted("GET", "users"));
        assert!(!allowlisted("DELETE", "workers"));
        assert!(!allowlisted("GET", "../grid"));
    }

    #[test]
    fn token_eq_is_constant_time_shaped() {
        assert!(token_eq("abc", "abc"));
        assert!(!token_eq("abc", "abd"));
        assert!(!token_eq("abc", "ab"));
        assert!(!token_eq("", ""));
    }

    #[test]
    fn presented_token_reads_bearer_and_header() {
        assert_eq!(presented_token(Some("Bearer secret-1"), None), "secret-1");
        assert_eq!(
            presented_token(Some("Bearer ignored"), Some("header-wins")),
            "header-wins"
        );
        assert_eq!(presented_token(None, None), "");
    }

    #[test]
    fn redact_json_drops_secret_keys() {
        let v = json!({
            "id": "edge-pc-01",
            "token": "leak",
            "nested": { "password": "x", "ok": true },
            "bot_token": "nope"
        });
        let out = redact_json(&v);
        assert_eq!(out["id"], "edge-pc-01");
        assert!(out.get("token").is_none(), "{out}");
        assert!(out.get("bot_token").is_none(), "{out}");
        assert_eq!(out["nested"]["ok"], true);
        assert!(out["nested"].get("password").is_none());
    }

    #[test]
    fn gate_requires_token_and_allowlist() {
        clear_rate_limit();
        assert_eq!(gate("GET", "login", "tok", "tok").unwrap_err().status, 404);
        assert_eq!(gate("GET", "health", "tok", "").unwrap_err().status, 503);
        assert_eq!(
            gate("GET", "health", "wrong", "right-token-xx")
                .unwrap_err()
                .status,
            401
        );
        assert_eq!(
            gate("GET", "health", "right-token-xx", "right-token-xx").unwrap(),
            "health"
        );
    }

    #[test]
    fn rate_limit_trips_after_cap() {
        clear_rate_limit();
        let key = "rate-key-unique";
        for _ in 0..RATE_CAP {
            assert!(rate_ok(key));
        }
        assert!(!rate_ok(key));
        clear_rate_limit();
        assert!(rate_ok(key));
    }

    #[test]
    fn status_wire_never_echoes_token() {
        let dir = temp("status");
        let file = SettingsFile {
            edge: EdgeSettings {
                token: "file-secret-edge".into(),
            },
            ..Default::default()
        };
        settings::save(&dir, &file).expect("save");
        let w = status_wire(&dir);
        assert_eq!(w["ok"], true);
        assert_eq!(w["token_set"], true);
        assert_eq!(w["source"], "file");
        let raw = serde_json::to_string(&w).expect("json");
        assert!(!raw.contains("file-secret-edge"), "{raw}");
        assert!(w["allowlist"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "grid"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn expected_token_env_wins() {
        let file = SettingsFile {
            edge: EdgeSettings {
                token: "file".into(),
            },
            ..Default::default()
        };
        assert_eq!(expected_token(&file, Some("env-tok")), "env-tok");
        assert_eq!(expected_token(&file, None), "file");
        assert_eq!(expected_token(&SettingsFile::default(), None), "");
    }

    #[tokio::test]
    async fn forward_hits_loopback_stub() {
        async fn ok_json(axum::Json(_): axum::Json<Value>) -> axum::Json<Value> {
            axum::Json(json!({ "ok": true, "id": "n", "token": "leak" }))
        }
        async fn get_ok() -> axum::Json<Value> {
            axum::Json(json!({ "nodes": { "edge-pc-01": {} }, "password": "x" }))
        }
        let app = axum::Router::new()
            .route("/api/v1/health", axum::routing::get(get_ok))
            .route("/api/v1/jobs", axum::routing::post(ok_json));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().expect("addr");
        tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve");
        });
        let base = format!("http://127.0.0.1:{}/api/v1", addr.port());
        let (st, v) = forward(&base, "GET", "health", None).await.expect("get");
        assert_eq!(st, 200);
        assert!(v.get("password").is_none(), "{v}");
        assert!(v["nodes"].get("edge-pc-01").is_some(), "{v}");
        let (st, v) = forward(&base, "POST", "jobs", Some(&json!({"a":1})))
            .await
            .expect("post");
        assert_eq!(st, 200);
        assert_eq!(v["ok"], true);
        assert!(v.get("token").is_none(), "{v}");
    }
}
