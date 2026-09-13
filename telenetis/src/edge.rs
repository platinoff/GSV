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
use std::time::Duration;

/// poolAI HTTP client (5s whole-request budget, same as [`crate::gsv::client`]).
#[derive(Clone)]
pub struct PoolClient {
    http: reqwest::Client,
    base_url: String,
}

impl PoolClient {
    pub fn new(config: &Config) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .connect_timeout(Duration::from_secs(3))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            http,
            base_url: config.poolai_url.trim_end_matches('/').to_string(),
        }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    async fn get_json(&self, path: &str) -> Result<Value, TelenetisError> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self.http.get(&url).send().await?;
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
            v.telegram_user_id, v.peer_id, v.bound_at, tasks
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
        assert!(text.contains("edge-live-01"));
        assert!(text.contains("0 pending / 1 done"));
        assert!(text.contains("flat"));
        assert!(render_workers(&[], None).contains("(none bound"));
    }
}
