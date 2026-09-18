//! poolAI edge-worker surface (edge-plane band).
//!
//! Phones already run Telegram, so the control plane rides poolAI +
//! Telenetis instead of a new protocol: a worker registers with
//! `origin=telegram_edge` + `role=virtual_node` and an ed25519-signed
//! capability document, binds its Telegram user id, joins the pool and
//! polls/completes tasks. Tensors stay on ggml-rpc-server
//! (`llama-rs/docs/DISTRIBUTED.md`). This module holds the server-testable
//! logic: a small poolAI HTTP client (mirrors `gsv::client`), the edge
//! read-model assembled from poolAI's bindings + seats + task status, and
//! the bot-reply renderer. Live shapes verified against poolAI 0.2.2.

use crate::config::Config;
use crate::error::TelenetisError;
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// llama_serve base URL (operator's box; loopback default is fine because
/// phones go through Telenetis, never direct).
pub fn llama_url() -> String {
    std::env::var("TELENETIS_LLAMA_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "http://127.0.0.1:8080".to_string())
}

/// Outbound LAN IPv4 of this host (std UDP trick: connect sends nothing,
/// the socket just learns its local address). `None` when offline.
pub fn local_lan_ip() -> Option<String> {
    let sock = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    sock.connect("8.8.8.8:80").ok()?;
    let ip = sock.local_addr().ok()?.ip();
    if ip.is_unspecified() || ip.is_loopback() {
        return None;
    }
    Some(ip.to_string())
}

/// Replace the host part of a loopback URL with the LAN IP so phones on
/// the same Wi-Fi can reach it. Non-loopback URLs pass through.
pub fn lan_url(loopback_url: &str) -> String {
    let Some(ip) = local_lan_ip() else {
        return loopback_url.to_string();
    };
    let out = loopback_url.to_string();
    for host in ["127.0.0.1", "localhost"] {
        if let Some(rest) = out.strip_prefix(&format!("http://{host}:")) {
            return format!("http://{ip}:{rest}");
        }
    }
    out
}

/// Mini App base URL for bot buttons: tunnel/public URL first (HTTPS,
/// works everywhere), else the LAN URL (same Wi-Fi), never 127.0.0.1
/// (on a phone that address is the phone itself).
pub fn mini_app_base(
    tunnel_url: Option<String>,
    public_url: Option<String>,
    port: u16,
    lan_ip: Option<String>,
) -> String {
    if let Some(u) = tunnel_url.filter(|u| !u.trim().is_empty()) {
        return u.trim_end_matches('/').to_string();
    }
    if let Some(u) = public_url.filter(|u| !u.trim().is_empty()) {
        return u.trim_end_matches('/').to_string();
    }
    match lan_ip.filter(|ip| !ip.trim().is_empty()) {
        Some(ip) => format!("http://{ip}:{port}"),
        None => format!("http://127.0.0.1:{port}"),
    }
}

/// Phone-reachable service map: every service addressable three ways —
/// loopback (this box), LAN (same Wi-Fi), and via Telenetis reverse proxy
/// (same origin, works out-of-NAT through the tunnel: no 127.0.0.1 leaks
/// to the phone). `public_base` is the ngrok URL when the tunnel is up.
pub fn service_endpoints(config: &Config, public_base: Option<&str>) -> serde_json::Value {
    let public = public_base
        .map(|u| u.trim_end_matches('/').to_string())
        .filter(|u| !u.is_empty());
    let svc = |name: &str, loopback: &str, via_path: &str| {
        let via = format!("/edge/upstream{via_path}");
        serde_json::json!({
            "name": name,
            "loopback": loopback,
            "lan": lan_url(loopback),
            "via_telenetis": via,
            "public": public.as_ref().map(|b| format!("{b}{via}")),
        })
    };
    serde_json::json!({
        "public_base": public,
        "services": [
            svc("llama", &llama_url(), "/llama"),
            svc("poolai", &config.poolai_url, "/poolai"),
        ],
    })
}

/// poolAI login credentials. **No compiled admin/admin123.**
/// Clients use hub `/api/edge` + `GSV_EDGE_TOKEN` / `TELENETIS_EDGE_TOKEN`.
/// Direct poolAI login is opt-in via `TELENETIS_POOLAI_USER` + `TELENETIS_POOLAI_PASS`.
pub fn poolai_user() -> Option<String> {
    std::env::var("TELENETIS_POOLAI_USER")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

pub fn poolai_pass() -> Option<String> {
    std::env::var("TELENETIS_POOLAI_PASS")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Hub edge token (never log the value). Env `GSV_EDGE_TOKEN` wins, else
/// `TELENETIS_EDGE_TOKEN`.
pub fn edge_token() -> Option<String> {
    std::env::var("GSV_EDGE_TOKEN")
        .ok()
        .or_else(|| std::env::var("TELENETIS_EDGE_TOKEN").ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Prefer hub `/api/edge` whenever an edge token is present.
pub fn prefers_hub_edge() -> bool {
    edge_token().is_some()
}

/// Strip `/api/v1` so the same PoolClient paths work on hub `/api/edge`.
pub fn rewrite_for_hub(path: &str) -> String {
    let p = path.trim();
    let p = p.strip_prefix('/').unwrap_or(p);
    let p = p.strip_prefix("api/v1/").unwrap_or(p);
    let p = p.strip_prefix("api/v1").unwrap_or(p);
    format!("/{p}")
}

/// poolAI HTTP client (5s whole-request budget, same as [`crate::gsv::client`]).
#[derive(Clone)]
pub struct PoolClient {
    http: reqwest::Client,
    base_url: String,
    token: Arc<RwLock<Option<String>>>,
    hub_edge: bool,
}

/// Allowlisted reverse-proxy target: phones talk to Telenetis, Telenetis
/// talks loopback. Anything else is rejected (no open proxy).
pub fn upstream_base(config: &Config, service: &str) -> Option<String> {
    match service.trim() {
        "llama" => Some(llama_url()),
        "poolai" => Some(config.poolai_url.trim_end_matches('/').to_string()),
        _ => None,
    }
}

impl PoolClient {
    pub fn new(config: &Config) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .connect_timeout(Duration::from_secs(3))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        let hub_edge = prefers_hub_edge();
        let base_url = if hub_edge {
            format!("{}/api/edge", config.gsv_url.trim_end_matches('/'))
        } else {
            config.poolai_url.trim_end_matches('/').to_string()
        };
        Self {
            http,
            base_url,
            token: Arc::new(RwLock::new(None)),
            hub_edge,
        }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn hub_edge(&self) -> bool {
        self.hub_edge
    }

    fn request_url(&self, path: &str) -> String {
        let p = if self.hub_edge {
            rewrite_for_hub(path)
        } else {
            path.to_string()
        };
        format!("{}{p}", self.base_url)
    }

    async fn get_json(&self, path: &str) -> Result<Value, TelenetisError> {
        let url = self.request_url(path);
        let mut req = self.http.get(&url);
        if self.hub_edge {
            let t = edge_token().ok_or_else(|| TelenetisError::Pool("edge token unset".into()))?;
            req = req.header("x-gsv-edge-token", t);
        }
        let resp = req.send().await?;
        let status = resp.status();
        if !status.is_success() {
            return Err(TelenetisError::Pool(format!("HTTP {status} from {url}")));
        }
        Ok(resp.json().await?)
    }

    pub async fn health(&self) -> Result<Value, TelenetisError> {
        self.get_json("/api/v1/health").await
    }

    pub async fn bindings(&self) -> Result<Value, TelenetisError> {
        self.get_json("/api/v1/virtual-nodes/telegram/bindings")
            .await
    }

    pub async fn seats(&self) -> Result<Value, TelenetisError> {
        self.get_json("/api/v1/grid/telegram-seats").await
    }

    pub async fn task_status(&self, peer_id: &str) -> Result<Value, TelenetisError> {
        self.get_json(&format!("/api/v1/virtual-nodes/{peer_id}/tasks/status"))
            .await
    }

    async fn login(&self) -> Result<String, TelenetisError> {
        if self.hub_edge {
            return Err(TelenetisError::Pool(
                "poolAI login blocked; use GSV_EDGE_TOKEN on /api/edge".into(),
            ));
        }
        let user = poolai_user().ok_or_else(|| {
            TelenetisError::Pool("TELENETIS_POOLAI_USER unset; use GSV_EDGE_TOKEN".into())
        })?;
        let pass = poolai_pass().ok_or_else(|| {
            TelenetisError::Pool("TELENETIS_POOLAI_PASS unset; use GSV_EDGE_TOKEN".into())
        })?;
        let url = format!("{}/api/v1/login", self.base_url);
        let resp = self
            .http
            .post(&url)
            .json(&serde_json::json!({
                "username": user,
                "password": pass,
            }))
            .send()
            .await?;
        let status = resp.status();
        if !status.is_success() {
            return Err(TelenetisError::Pool(format!("login HTTP {status}")));
        }
        resp.json::<Value>()
            .await?
            .get("token")
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| TelenetisError::Pool("login: no token".to_string()))
    }

    /// Service bearer token for proxied poolAI calls.
    pub async fn bearer_token(&self) -> Result<String, TelenetisError> {
        self.token().await
    }

    async fn token(&self) -> Result<String, TelenetisError> {
        if self.hub_edge {
            return edge_token().ok_or_else(|| TelenetisError::Pool("edge token unset".into()));
        }
        if let Some(t) = self.token.read().await.clone() {
            return Ok(t);
        }
        let t = self.login().await?;
        *self.token.write().await = Some(t.clone());
        Ok(t)
    }

    async fn authed(
        &self,
        method: &str,
        path: &str,
        body: Option<&Value>,
    ) -> Result<Value, TelenetisError> {
        for attempt in 0..2 {
            let t = self.token().await?;
            let url = self.request_url(path);
            let mut req = match method {
                "POST" => self.http.post(&url),
                "PUT" => self.http.put(&url),
                "DELETE" => self.http.delete(&url),
                _ => self.http.get(&url),
            };
            req = if self.hub_edge {
                req.header("x-gsv-edge-token", &t)
            } else {
                req.bearer_auth(&t)
            };
            if let Some(b) = body {
                req = req.json(b);
            }
            let resp = req.send().await?;
            let status = resp.status();
            if status.as_u16() == 401 && attempt == 0 {
                *self.token.write().await = None;
                continue;
            }
            if !status.is_success() {
                return Err(TelenetisError::Pool(format!("HTTP {status} from {url}")));
            }
            return Ok(resp.json().await?);
        }
        Err(TelenetisError::Pool("auth retry exhausted".to_string()))
    }

    /// VM instances (operator plane). See poolAI `VmCreateRequest`.
    pub async fn vm_list(&self) -> Result<Value, TelenetisError> {
        self.authed("GET", "/api/v1/vm/instances", None).await
    }

    pub async fn vm_create(
        &self,
        name: &str,
        cpu_cores: u16,
        memory_mb: u32,
    ) -> Result<Value, TelenetisError> {
        self.authed(
            "POST",
            "/api/v1/vm/instances",
            Some(&serde_json::json!({
                "name": name,
                "resources": {
                    "cpu_cores": cpu_cores,
                    "memory_mb": memory_mb,
                    "gpu_required": false,
                },
            })),
        )
        .await
    }

    pub async fn vm_start(&self, id: &str) -> Result<Value, TelenetisError> {
        // Start returns an empty 200 body on this build — normalize to JSON.
        let _ = self
            .authed("POST", &format!("/api/v1/vm/instances/{id}/start"), None)
            .await;
        Ok(Value::Null)
    }

    pub async fn vm_stop(&self, id: &str) -> Result<Value, TelenetisError> {
        let _ = self
            .authed("POST", &format!("/api/v1/vm/instances/{id}/stop"), None)
            .await;
        Ok(Value::Null)
    }

    pub async fn vm_health(&self, id: &str) -> Result<Value, TelenetisError> {
        self.authed("GET", &format!("/api/v1/vm/instances/{id}/health"), None)
            .await
    }

    async fn post_json(&self, path: &str, body: &Value) -> Result<Value, TelenetisError> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self.http.post(&url).json(body).send().await?;
        let status = resp.status();
        if !status.is_success() {
            return Err(TelenetisError::Pool(format!("HTTP {status} from {url}")));
        }
        Ok(resp.json().await?)
    }

    /// Enqueue any task type for a peer. Returns the task id.
    pub async fn enqueue_task(
        &self,
        peer_id: &str,
        task_type: &str,
        payload: Value,
    ) -> Result<String, TelenetisError> {
        let body = self
            .post_json(
                &format!("/api/v1/virtual-nodes/{peer_id}/tasks"),
                &serde_json::json!({
                    "task_type": task_type,
                    "payload": payload,
                }),
            )
            .await?;
        body.get("task")
            .and_then(|t| t.get("id"))
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| TelenetisError::Pool("enqueue: no task id".to_string()))
    }

    /// Enqueue a `llama_chat` task for a peer. Returns the task id.
    /// `model` is `"fast"` (interactive) or `"deep"` (27B).
    pub async fn enqueue_chat(
        &self,
        peer_id: &str,
        prompt: &str,
        max_tokens: u64,
        model: &str,
    ) -> Result<String, TelenetisError> {
        self.enqueue_task(
            peer_id,
            "llama_chat",
            serde_json::json!({"prompt": prompt, "max_tokens": max_tokens, "model": model}),
        )
        .await
    }

    /// Read a chat answer: `Ok(Some(text))` when completed,
    /// `Ok(None)` while the worker is still on it.
    pub async fn chat_answer(
        &self,
        peer_id: &str,
        task_id: &str,
    ) -> Result<Option<String>, TelenetisError> {
        let url = format!(
            "{}/api/v1/virtual-nodes/{}/tasks/result?task_id={}",
            self.base_url, peer_id, task_id
        );
        let resp = self.http.get(&url).send().await?;
        if resp.status().as_u16() == 404 {
            return Ok(None);
        }
        if !resp.status().is_success() {
            return Err(TelenetisError::Pool(format!(
                "HTTP {} from result",
                resp.status()
            )));
        }
        let v: Value = resp.json().await?;
        Ok(v.get("result")
            .and_then(|r| r.get("detail"))
            .and_then(Value::as_str)
            .map(str::to_string))
    }

    /// Latest completion record for a peer (any task type).
    pub async fn latest_result(&self, peer_id: &str) -> Result<Option<Value>, TelenetisError> {
        let url = format!(
            "{}/api/v1/virtual-nodes/{}/tasks/result",
            self.base_url, peer_id
        );
        let resp = self.http.get(&url).send().await?;
        if resp.status().as_u16() == 404 {
            return Ok(None);
        }
        if !resp.status().is_success() {
            return Err(TelenetisError::Pool(format!(
                "HTTP {} from result",
                resp.status()
            )));
        }
        let v: Value = resp.json().await?;
        Ok(v.get("result").cloned())
    }

    /// Peer id bound to a Telegram user id, if any.
    pub async fn peer_for_user(
        &self,
        telegram_user_id: &str,
    ) -> Result<Option<String>, TelenetisError> {
        let bindings = self.bindings().await?;
        Ok(assemble_views(&bindings)
            .into_iter()
            .find(|v| v.telegram_user_id == telegram_user_id)
            .map(|v| v.peer_id))
    }
}

/// VM name convention linking a poolAI VM to an edge peer: `{peer_id}-vm`.
pub fn vm_name_for_peer(peer_id: &str) -> String {
    format!("{}-vm", peer_id.trim())
}

/// One shard assignment parsed from a `llama_shard` completion detail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShardAssignment {
    pub peer_id: String,
    pub task_id: String,
    pub model: String,
    pub layers: String,
    pub rpc_reachable: bool,
}

/// Parse a completion record into a shard assignment. Returns `None` when
/// the record is not a `llama_shard` completion (other task types share
/// the same completion store).
pub fn parse_shard_assignment(peer_id: &str, record: &Value) -> Option<ShardAssignment> {
    let detail = record.get("detail")?.as_str()?;
    let d: Value = serde_json::from_str(detail).ok()?;
    Some(ShardAssignment {
        peer_id: peer_id.to_string(),
        task_id: record
            .get("task_id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        model: d
            .get("model")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        layers: d
            .get("assigned_layers")
            .and_then(Value::as_str)
            .map(str::to_string)?,
        rpc_reachable: d
            .get("rpc_reachable")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    })
}

/// Latest shard assignment per peer (layer map source of truth).
pub async fn shard_map(pool: &PoolClient, peers: &[String]) -> Vec<ShardAssignment> {
    let mut out = Vec::new();
    for peer in peers {
        let rec = match pool.latest_result(peer).await {
            Ok(Some(r)) => r,
            _ => continue,
        };
        if let Some(a) = parse_shard_assignment(peer, &rec) {
            out.push(a);
        }
    }
    out
}

/// Find a VM in a `/vm/instances` list body by name.
pub fn find_vm<'a>(list_body: &'a Value, name: &str) -> Option<&'a Value> {
    list_body
        .as_array()?
        .iter()
        .find(|v| v.get("name").and_then(Value::as_str).unwrap_or("") == name)
}

/// One virtual worker row: Telegram binding + poolAI VM behind it.
#[derive(Debug, Clone)]
pub struct EdgeVmView {
    pub telegram_user_id: String,
    pub peer_id: String,
    pub vm_id: Option<String>,
    pub vm_status: Option<String>,
    pub vm_health: Option<String>,
}

/// Assemble virtual-worker rows: bindings × `{peer}-vm` instances × health.
/// `filter` matches telegram user id or peer id exactly (`@` stripped).
/// Shared by the `/vm` bot command and the Mini App JSON.
pub async fn vm_views(
    pool: &PoolClient,
    filter: Option<&str>,
) -> Result<Vec<EdgeVmView>, TelenetisError> {
    let bindings = pool.bindings().await?;
    let views = assemble_views(&bindings);
    let vms = pool.vm_list().await.ok();
    let mut rows = Vec::new();
    for v in &views {
        if let Some(want) = filter {
            let w = want.trim().trim_start_matches('@');
            if v.telegram_user_id != w && v.peer_id != w {
                continue;
            }
        }
        let vm = vms
            .as_ref()
            .and_then(|list| find_vm(list, &vm_name_for_peer(&v.peer_id)));
        let (vm_id, vm_status) = match vm {
            Some(m) => (
                m.get("id").and_then(|x| x.as_str()).map(str::to_string),
                m.get("status").and_then(|x| x.as_str()).map(str::to_string),
            ),
            None => (None, None),
        };
        let mut health = None;
        if let Some(ref vid) = vm_id {
            if let Ok(h) = pool.vm_health(vid).await {
                health = h.get("status").and_then(|x| x.as_str()).map(str::to_string);
            }
        }
        rows.push(EdgeVmView {
            telegram_user_id: v.telegram_user_id.clone(),
            peer_id: v.peer_id.clone(),
            vm_id,
            vm_status,
            vm_health: health,
        });
    }
    Ok(rows)
}

/// Markdown reply for `/vm` (all) and `/vm <user|peer>` (one).
/// Numeric Telegram ids are masked like in [`render_workers`].
pub fn render_vms(views: &[EdgeVmView]) -> String {
    let mut out = String::from("*My VMs*\n\n");
    if views.is_empty() {
        out.push_str("  (none — bind a peer first, then create its VM)\n");
    }
    for v in views {
        let vm = match (&v.vm_id, &v.vm_status) {
            (Some(id), Some(st)) => format!("`{}` [{}]", &id[..8.min(id.len())], st),
            _ => "(no VM — create `{peer}-vm`)".replace("{peer}", &v.peer_id),
        };
        let health = v.vm_health.as_deref().unwrap_or("n/a");
        out.push_str(&format!(
            "  {} ↔ `{}`\n  VM {} · health {}\n",
            mask_id(&v.telegram_user_id),
            v.peer_id,
            vm,
            health
        ));
    }
    out
}

/// One edge worker row for the Mini App screen and `/worker` replies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdgeWorkerView {
    pub telegram_user_id: String,
    pub peer_id: String,
    pub bound_at: String,
    pub pending: u64,
    pub completed: u64,
    pub tasks_ok: bool,
}

/// Assemble the view from poolAI's bindings list. Task counters are filled
/// per peer by [`PoolClient::task_status`]; pass an empty map for list-only.
pub fn assemble_views(bindings_body: &Value) -> Vec<EdgeWorkerView> {
    bindings_body
        .get("bindings")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|b| {
                    Some(EdgeWorkerView {
                        telegram_user_id: b.get("telegram_user_id")?.as_str()?.to_string(),
                        peer_id: b.get("peer_id")?.as_str()?.to_string(),
                        bound_at: b
                            .get("bound_at")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string(),
                        pending: 0,
                        completed: 0,
                        tasks_ok: false,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Fill task counters from a `/tasks/status` body (`{pending, completed}`).
pub fn apply_task_status(view: &mut EdgeWorkerView, status_body: &Value) {
    view.pending = status_body
        .get("pending")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    view.completed = status_body
        .get("completed")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    view.tasks_ok = true;
}

/// Find one worker by Telegram user id or peer id (exact match).
pub fn find_worker<'a>(views: &'a [EdgeWorkerView], id: &str) -> Option<&'a EdgeWorkerView> {
    let id = id.trim().trim_start_matches('@');
    views
        .iter()
        .find(|v| v.telegram_user_id == id || v.peer_id == id)
}

/// Mask a numeric Telegram user id for chat surfaces (`5035500793` →
/// `503…793`). Non-numeric handles pass through (public anyway); peer ids
/// are internal names and stay full. Full ids live only in the
/// authenticated Mini App JSON, never in bot replies.
pub fn mask_id(id: &str) -> String {
    let id = id.trim().trim_start_matches('@');
    if id.len() >= 6 && id.bytes().all(|b| b.is_ascii_digit()) {
        format!("{}…{}", &id[..3], &id[id.len() - 3..])
    } else {
        id.to_string()
    }
}

/// Markdown reply for `/worker` (list) and `/worker <id>` (detail).
pub fn render_workers(views: &[EdgeWorkerView], seats: Option<&Value>) -> String {
    let mut out = String::from("*Edge workers*\n\n");
    if views.is_empty() {
        out.push_str("  (none bound — `/start` on a phone, then bind its peer)\n");
    }
    for v in views {
        let tasks = if v.tasks_ok {
            format!("{} pending / {} done", v.pending, v.completed)
        } else {
            "tasks n/a".to_string()
        };
        out.push_str(&format!(
            "  `{}` ↔ `{}`\n  bound {} · {}\n",
            mask_id(&v.telegram_user_id),
            v.peer_id,
            v.bound_at,
            tasks
        ));
    }
    if let Some(s) = seats {
        let policy = s.get("seat_policy").and_then(Value::as_str).unwrap_or("?");
        let active = s
            .get("active_telegram_edge_workers")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        out.push_str(&format!("\nSeats: policy `{policy}`, {active} active"));
        if let Some(limit) = s.get("seat_limit").and_then(Value::as_u64) {
            out.push_str(&format!(" / {limit}"));
        }
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn test_config() -> Config {
        Config {
            bot_token: "test".to_string(),
            gsv_url: "http://127.0.0.1:9999".to_string(),
            poolai_url: "http://127.0.0.1:8091".to_string(),
            port: 9800,
            jail_id: "test-jail".to_string(),
            godfather_channel_id: 0,
            webhook_url: None,
            webhook_secret: None,
            public_url: None,
            tunnel_enabled: false,
            ngrok_bin: None,
        }
    }

    fn sample_bindings() -> Value {
        // Live shape from poolAI 0.2.2 (verified 2026-09-13).
        json!({"bindings": [
            {"telegram_user_id": "999001", "peer_id": "edge-live-01",
             "bound_at": "2026-09-13T20:57:38Z"},
        ]})
    }

    #[test]
    fn assemble_views_parses_live_shape() {
        let views = assemble_views(&sample_bindings());
        assert_eq!(views.len(), 1);
        assert_eq!(views[0].telegram_user_id, "999001");
        assert_eq!(views[0].peer_id, "edge-live-01");
        assert!(!views[0].tasks_ok);
    }

    #[test]
    fn assemble_views_empty_without_bindings() {
        assert!(assemble_views(&json!({})).is_empty());
        assert!(assemble_views(&json!({"bindings": []})).is_empty());
    }

    #[test]
    fn apply_task_status_fills_counters() {
        let mut views = assemble_views(&sample_bindings());
        apply_task_status(&mut views[0], &json!({"pending": 0, "completed": 1}));
        assert_eq!(views[0].pending, 0);
        assert_eq!(views[0].completed, 1);
        assert!(views[0].tasks_ok);
    }

    #[test]
    fn find_worker_by_user_or_peer() {
        let views = assemble_views(&sample_bindings());
        assert!(find_worker(&views, "999001").is_some());
        assert!(find_worker(&views, "edge-live-01").is_some());
        assert!(find_worker(&views, "@999001").is_some());
        assert!(find_worker(&views, "nobody").is_none());
    }

    #[test]
    fn render_lists_and_details() {
        let mut views = assemble_views(&sample_bindings());
        apply_task_status(&mut views[0], &json!({"pending": 0, "completed": 1}));
        let text = render_workers(
            &views,
            Some(&json!({"seat_policy": "flat", "active_telegram_edge_workers": 0})),
        );
        assert_eq!(views[0].telegram_user_id, "999001");
        assert!(text.contains("edge-live-01"));
        assert!(text.contains("0 pending / 1 done"));
        assert!(text.contains("flat"));
        assert!(render_workers(&[], None).contains("(none bound"));
    }

    #[test]
    fn mask_id_hides_numeric_ids() {
        assert_eq!(mask_id("5035500793"), "503…793");
        assert_eq!(mask_id("999001"), "999…001");
        // Short ids, handles and peer names pass through.
        assert_eq!(mask_id("12345"), "12345");
        assert_eq!(mask_id("platinofff"), "platinofff");
        assert_eq!(mask_id("@platinofff"), "platinofff");
        assert_eq!(mask_id("edge-live-01"), "edge-live-01");
    }

    #[test]
    fn render_never_leaks_full_numeric_id() {
        let views = assemble_views(&sample_bindings());
        let text = render_workers(&views, None);
        assert!(!text.contains("999001"), "{text}");
        assert!(text.contains("999…001"), "{text}");
    }

    #[test]
    fn vm_name_convention() {
        assert_eq!(vm_name_for_peer("a54-01"), "a54-01-vm");
    }

    #[test]
    fn find_vm_by_name() {
        // Live shape from poolAI 0.2.2 (verified 2026-09-13).
        let list = json!([{
            "id": "3f992073-e6c8-49c6-b281-d9517370e38d",
            "name": "a54-01-vm",
            "status": "Running",
        }]);
        let vm = find_vm(&list, "a54-01-vm").expect("found");
        assert_eq!(vm["status"], "Running");
        assert!(find_vm(&list, "nope-vm").is_none());
    }

    #[test]
    fn upstream_allowlist() {
        let cfg = test_config();
        assert_eq!(
            super::upstream_base(&cfg, "llama"),
            Some("http://127.0.0.1:8080".to_string())
        );
        assert_eq!(
            super::upstream_base(&cfg, "poolai"),
            Some("http://127.0.0.1:8091".to_string())
        );
        assert_eq!(super::upstream_base(&cfg, "evil"), None);
        assert_eq!(super::upstream_base(&cfg, "../x"), None);
    }

    #[test]
    fn lan_url_shapes() {
        // Non-loopback passes through untouched.
        assert_eq!(
            super::lan_url("https://example.com:9/x"),
            "https://example.com:9/x"
        );
        // Loopback either converts to the LAN IP or (offline) stays.
        let out = super::lan_url("http://127.0.0.1:8080/v1");
        assert!(out.starts_with("http://"), "{out}");
        assert!(!out.contains("localhost"), "{out}");
    }

    #[test]
    fn mini_app_base_prefers_tunnel_then_lan() {
        assert_eq!(
            super::mini_app_base(
                Some("https://abc.ngrok.io".into()),
                Some("https://pub.example".into()),
                9800,
                Some("192.168.2.238".into())
            ),
            "https://abc.ngrok.io"
        );
        assert_eq!(
            super::mini_app_base(None, Some("https://pub.example/".into()), 9800, None),
            "https://pub.example"
        );
        assert_eq!(
            super::mini_app_base(None, None, 9800, Some("192.168.2.238".into())),
            "http://192.168.2.238:9800"
        );
        // No tunnel and no LAN: loopback is the honest last resort.
        assert_eq!(
            super::mini_app_base(None, None, 9800, None),
            "http://127.0.0.1:9800"
        );
    }

    #[test]
    fn local_lan_ip_runs() {
        // May be None offline; must never panic and never be loopback.
        if let Some(ip) = super::local_lan_ip() {
            assert!(!ip.starts_with("127."), "{ip}");
            assert!(ip.parse::<std::net::Ipv4Addr>().is_ok(), "{ip}");
        }
    }

    #[test]
    fn endpoints_shape() {
        let cfg = test_config();
        let v = super::service_endpoints(&cfg, Some("https://abc.ngrok.io"));
        assert_eq!(v["services"][0]["name"], "llama");
        assert_eq!(v["services"][0]["via_telenetis"], "/edge/upstream/llama");
        assert_eq!(
            v["services"][0]["public"],
            "https://abc.ngrok.io/edge/upstream/llama"
        );
        assert_eq!(v["public_base"], "https://abc.ngrok.io");
    }

    #[test]
    fn parse_shard_assignment_shapes() {
        let rec = json!({
            "task_id": "t1",
            "detail": r#"{"model":"lama-2.8","assigned_layers":"0-16","rpc_endpoint":"h:1","rpc_reachable":false,"worker":"a54-01"}"#,
        });
        let a = super::parse_shard_assignment("a54-01", &rec).expect("shard");
        assert_eq!(a.layers, "0-16");
        assert_eq!(a.model, "lama-2.8");
        assert!(!a.rpc_reachable);
        // Non-shard completions (ping/chat answers) are skipped.
        assert!(super::parse_shard_assignment(
            "a54-01",
            &json!({"task_id": "t2", "detail": "pong"}),
        )
        .is_none());
        assert!(super::parse_shard_assignment("a54-01", &json!({})).is_none());
    }

    #[test]
    fn render_vms_masks_ids() {
        let views = vec![EdgeVmView {
            telegram_user_id: "5035500793".to_string(),
            peer_id: "a54-01".to_string(),
            vm_id: Some("3f992073-e6c8-49c6-b281-d9517370e38d".to_string()),
            vm_status: Some("Running".to_string()),
            vm_health: Some("healthy".to_string()),
        }];
        let text = render_vms(&views);
        assert!(!text.contains("5035500793"), "{text}");
        assert!(text.contains("503…793"), "{text}");
        assert!(text.contains("a54-01"), "{text}");
        assert!(text.contains("Running"), "{text}");
        assert!(render_vms(&[]).contains("(none"));
    }

    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn rewrite_for_hub_strips_api_v1() {
        assert_eq!(rewrite_for_hub("/api/v1/health"), "/health");
        assert_eq!(
            rewrite_for_hub("/api/v1/virtual-nodes/redmi-01/pool/join"),
            "/virtual-nodes/redmi-01/pool/join"
        );
        assert_eq!(rewrite_for_hub("health"), "/health");
        assert!(!rewrite_for_hub("/api/v1/login").contains("8091"));
    }

    #[test]
    fn no_compiled_poolai_admin_password() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let old_u = std::env::var("TELENETIS_POOLAI_USER").ok();
        let old_p = std::env::var("TELENETIS_POOLAI_PASS").ok();
        std::env::remove_var("TELENETIS_POOLAI_USER");
        std::env::remove_var("TELENETIS_POOLAI_PASS");
        assert!(poolai_user().is_none());
        assert!(poolai_pass().is_none());
        match old_u {
            Some(v) => std::env::set_var("TELENETIS_POOLAI_USER", v),
            None => std::env::remove_var("TELENETIS_POOLAI_USER"),
        }
        match old_p {
            Some(v) => std::env::set_var("TELENETIS_POOLAI_PASS", v),
            None => std::env::remove_var("TELENETIS_POOLAI_PASS"),
        }
    }

    #[test]
    fn hub_edge_when_token_set() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let old_g = std::env::var("GSV_EDGE_TOKEN").ok();
        let old_t = std::env::var("TELENETIS_EDGE_TOKEN").ok();
        std::env::remove_var("TELENETIS_EDGE_TOKEN");
        std::env::set_var("GSV_EDGE_TOKEN", "test-edge-token");
        assert!(prefers_hub_edge());
        let c = PoolClient::new(&test_config());
        assert!(c.hub_edge());
        assert!(c.base_url().ends_with("/api/edge"));
        assert!(!c.base_url().contains("8091"));
        std::env::remove_var("GSV_EDGE_TOKEN");
        match old_g {
            Some(v) => std::env::set_var("GSV_EDGE_TOKEN", v),
            None => std::env::remove_var("GSV_EDGE_TOKEN"),
        }
        match old_t {
            Some(v) => std::env::set_var("TELENETIS_EDGE_TOKEN", v),
            None => std::env::remove_var("TELENETIS_EDGE_TOKEN"),
        }
    }
}
