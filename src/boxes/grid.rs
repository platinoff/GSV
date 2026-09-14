//! Grid box (ALLBGP) — durable hub-side mirror of the poolAI `:8091` fleet view.
//!
//! poolAI keeps peers/VMs/seats in memory only; a restart blanks the grid. The
//! hub therefore keeps its own snapshot + a bounded history ring in
//! `{data_dir}/gsv_grid.json` (gitignored): every device, worker, virtual node
//! and Telegram seat the coordinator currently sees — refreshed on a 10 s loop
//! by `gsv-server` (SSE `event: grid` when the key changes) and on demand via
//! `GET /api/grid` / MCP `gsv_grid` (fresh=true). When poolAI is down the last
//! known topology is **kept** and marked stale (`poolai_alive:false` + error),
//! so OpenCode/Cursor keep a working view instead of an empty one.
//! Canon: `docs/gsv/GSV_VDC.md`.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::broadcast;
use tokio::sync::RwLock;

use crate::vision::rfc3339_now;

/// Env override for the poolAI API root (tests + alt ports).
pub const POOLAI_ENV: &str = "GSV_POOLAI_URL";
/// Default poolAI base on this box (`:8080` is llama, never assume 8080 here).
pub const DEFAULT_POOLAI_BASE: &str = "http://127.0.0.1:8091/api/v1";
/// History ring capacity (10 s ticks ≈ one minute of churn, cheap JSON).
pub const HISTORY_CAP: usize = 64;
/// Background refresh cadence (seconds).
pub const REFRESH_SECS: u64 = 10;

/// Normalize the poolAI base URL (trim slash, default when empty).
pub fn poolai_base_from(env: Option<String>) -> String {
    env.map(|s| s.trim().trim_end_matches('/').to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_POOLAI_BASE.to_string())
}

/// Process-env view of [`poolai_base_from`].
pub fn poolai_base() -> String {
    poolai_base_from(std::env::var(POOLAI_ENV).ok())
}

/// Durable mirror of the coordinator's fleet view + history ring.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct GridStore {
    pub generated_at: String,
    pub poolai_alive: bool,
    pub last_error: String,
    /// `/topology/nodes` — map of node_id → capacity record.
    pub nodes: Value,
    /// `/workers` — pool worker array.
    pub workers: Value,
    /// `/discovery/virtual-nodes` — registered edge/virtual nodes.
    pub virtual_nodes: Value,
    /// `/grid/telegram-seats` — seat limits + in-use count.
    pub seats: Value,
    /// `/status` — coordinator version/uptime.
    pub status: Value,
    /// Ring of `{ts, alive, nodes, workers, virtual_nodes, seats_used}`.
    pub history: Vec<Value>,
}

/// Shared grid box held by `AppState`.
pub struct GridBox {
    client: reqwest::Client,
    pub base_url: String,
    data_dir: PathBuf,
    pub store: RwLock<GridStore>,
}

fn store_path(data_dir: &Path) -> PathBuf {
    data_dir.join("gsv_grid.json")
}

fn load_store(data_dir: &Path) -> GridStore {
    std::fs::read_to_string(store_path(data_dir))
        .ok()
        .and_then(|s| serde_json::from_str::<GridStore>(&s).ok())
        .unwrap_or_default()
}

fn save_store(data_dir: &Path, store: &GridStore) {
    let Ok(raw) = serde_json::to_string_pretty(store) else {
        return;
    };
    let _ = std::fs::create_dir_all(data_dir);
    let tmp = data_dir.join("gsv_grid.json.tmp");
    if std::fs::write(&tmp, raw).is_ok() {
        let _ = std::fs::rename(&tmp, store_path(data_dir));
    }
}

fn obj_len(v: &Value) -> usize {
    v.as_object().map(|m| m.len()).unwrap_or(0)
}

fn arr_len(v: &Value) -> usize {
    if let Some(a) = v.as_array() {
        return a.len();
    }
    v.get("nodes")
        .and_then(Value::as_array)
        .map(|a| a.len())
        .unwrap_or(0)
}

fn seats_used(v: &Value) -> u64 {
    v.get("active_telegram_edge_workers")
        .and_then(Value::as_u64)
        .unwrap_or(0)
}

/// `{nodes, workers, virtual_nodes}` counts for wire + change key.
fn counts(store: &GridStore) -> (usize, usize, usize, u64) {
    (
        obj_len(store.nodes.get("nodes").unwrap_or(&Value::Null)),
        arr_len(&store.workers),
        arr_len(&store.virtual_nodes),
        seats_used(&store.seats),
    )
}

impl GridBox {
    /// Build from env + the durable snapshot on disk (survives restarts).
    pub fn new(data_dir: &Path) -> Self {
        Self::with_base(data_dir, &poolai_base())
    }

    /// Explicit base (tests / alt deploys) + disk-loaded durable store.
    pub fn with_base(data_dir: &Path, base_url: &str) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(900))
            .no_proxy()
            .build()
            .unwrap_or_default();
        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            data_dir: data_dir.to_path_buf(),
            store: RwLock::new(load_store(data_dir)),
        }
    }

    async fn get_json(&self, path: &str) -> Result<Value, String> {
        let url = format!("{}/{}", self.base_url.trim_end_matches('/'), path);
        let res = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("{path}: {e}"))?;
        if !res.status().is_success() {
            return Err(format!("{path}: HTTP {}", res.status().as_u16()));
        }
        res.json::<Value>()
            .await
            .map_err(|e| format!("{path}: {e}"))
    }

    /// Fetch the five fleet endpoints. On any failure the previous topology is
    /// **kept** (ALLBGP stays announced) and only `poolai_alive`/`last_error`
    /// go stale. Persists + pushes a history entry when data changed.
    /// Returns `Some(payload)` when the change key moved (SSE-worthy).
    pub async fn refresh(&self) -> Option<Value> {
        let mut next = self.store.read().await.clone();
        let mut errors: Vec<String> = Vec::new();
        match self.get_json("topology/nodes").await {
            Ok(v) => next.nodes = v,
            Err(e) => errors.push(e),
        }
        match self.get_json("workers").await {
            Ok(v) => next.workers = v,
            Err(e) => errors.push(e),
        }
        match self.get_json("discovery/virtual-nodes").await {
            Ok(v) => next.virtual_nodes = v,
            Err(e) => errors.push(e),
        }
        match self.get_json("grid/telegram-seats").await {
            Ok(v) => next.seats = v,
            Err(e) => errors.push(e),
        }
        match self.get_json("status").await {
            Ok(v) => next.status = v,
            Err(e) => errors.push(e),
        }
        let alive = errors.len() < 5;
        next.poolai_alive = alive;
        next.last_error = if alive {
            String::new()
        } else {
            format!(
                "poolai unreachable at {} ({})",
                self.base_url,
                errors.join("; ")
            )
        };
        next.generated_at = rfc3339_now();
        let (nodes, workers, vnodes, used) = counts(&next);
        let entry = json!({
            "ts": next.generated_at,
            "alive": alive,
            "nodes": nodes,
            "workers": workers,
            "virtual_nodes": vnodes,
            "seats_used": used,
        });
        let prev = self.store.read().await;
        let (p_nodes, p_workers, p_vnodes, p_used) = counts(&prev);
        let prev_alive = prev.poolai_alive;
        drop(prev);
        let changed = (p_nodes, p_workers, p_vnodes, p_used) != (nodes, workers, vnodes, used)
            || alive != prev_alive;
        next.history.push(entry);
        if next.history.len() > HISTORY_CAP {
            let excess = next.history.len() - HISTORY_CAP;
            next.history.drain(0..excess);
        }
        *self.store.write().await = next.clone();
        save_store(&self.data_dir, &next);
        changed.then(|| {
            json!({"poolai_alive": alive, "nodes": nodes, "workers": workers,
                   "virtual_nodes": vnodes, "seats_used": used})
        })
    }

    /// Redacted-safe wire (no auth headers/keys ever cross from poolAI anyway).
    pub async fn wire(&self) -> Value {
        let store = self.store.read().await;
        let (nodes, workers, vnodes, used) = counts(&store);
        json!({
            "ok": true,
            "base_url": self.base_url,
            "generated_at": store.generated_at,
            "poolai_alive": store.poolai_alive,
            "stale": !store.poolai_alive,
            "last_error": store.last_error,
            "counts": {
                "nodes": nodes,
                "workers": workers,
                "virtual_nodes": vnodes,
                "seats_used": used,
                "seat_limit": store.seats.get("seat_limit").and_then(Value::as_u64),
            },
            "nodes": store.nodes,
            "workers": store.workers,
            "virtual_nodes": store.virtual_nodes,
            "seats": store.seats,
            "status": store.status,
            "history_len": store.history.len(),
            "history": store.history,
        })
    }
}

/// `gsv-server` background loop: refresh every [`REFRESH_SECS`] and emit SSE
/// `event: grid` when the fleet key moves.
pub fn spawn_grid_loop(grid: Arc<GridBox>, events: broadcast::Sender<String>) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(REFRESH_SECS)).await;
            if let Some(payload) = grid.refresh().await {
                if let Ok(raw) = serde_json::to_string(&payload) {
                    let _ = events.send(format!("event: grid\ndata: {raw}"));
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn poolai_base_default_and_override() {
        assert_eq!(poolai_base_from(None), DEFAULT_POOLAI_BASE);
        assert_eq!(poolai_base_from(Some("  ".into())), DEFAULT_POOLAI_BASE);
        assert_eq!(
            poolai_base_from(Some("http://127.0.0.1:9/api/v1/".into())),
            "http://127.0.0.1:9/api/v1"
        );
    }

    fn store_stub() -> GridStore {
        GridStore {
            nodes: json!({"nodes": {"a": {}, "b": {}}}),
            workers: json!([{"id": "a"}, {"id": "b"}]),
            virtual_nodes: json!({"nodes": [{"peer": {"peer_id": "a"}}]}),
            seats: json!({"seat_limit": 5, "active_telegram_edge_workers": 2}),
            ..Default::default()
        }
    }

    #[test]
    fn counts_read_poolai_shapes() {
        let (n, w, v, used) = counts(&store_stub());
        assert_eq!((n, w, v, used), (2, 2, 1, 2));
    }

    #[test]
    fn history_ring_caps() {
        let dir = std::env::temp_dir().join(format!("gsv-grid-ring-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        // Dead base: refresh must never touch a live coordinator in tests.
        let grid = GridBox::with_base(&dir, "http://127.0.0.1:9/api/v1");
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            for _ in 0..(HISTORY_CAP + 10) {
                grid.refresh().await;
            }
            let store = grid.store.read().await;
            assert_eq!(store.history.len(), HISTORY_CAP);
            assert!(!store.poolai_alive, "dead base must flip poolai_alive");
            assert!(store.last_error.contains("poolai unreachable"));
        });
        // Persisted across a fresh load (durable mirror, not process memory).
        let reloaded = load_store(&dir);
        assert!(!reloaded.poolai_alive);
        assert_eq!(reloaded.history.len(), HISTORY_CAP);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
