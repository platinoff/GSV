//! GSV shared application state.
//!
//! `AppState` is handed to every axum handler via `State<AppState>`. It carries:
//! - repo root + data dir paths (repo defaults to this crate root, `S:/rust/GSV`)
//! - durable Tracker store (`Arc<RwLock<TrackerStore>>`)
//! - IDE session selection (in-memory)
//! - update flag (`Arc<AtomicBool>`) + build metadata
//! - SSE event broadcast sender (`/events`)

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use serde_json::Value;
use tokio::sync::{broadcast, RwLock};

use crate::boxes::omni::OmniRouter;
use crate::boxes::usage::UsageStore;
use crate::tracker::TrackerStore;

/// Process-local HTTP MCP session (band 184: last `tools/list` count).
#[derive(Clone, Copy, Debug, Default)]
pub struct McpHttpSession {
    /// Issue sequence (oldest evicted first).
    pub seq: u64,
    /// `tools/list` length last seen for this session (`0` = never listed).
    pub listed_tools: u32,
}

/// Shared application state for the GSV server.
#[derive(Clone)]
pub struct AppState {
    /// Repo root (this GSV crate / git root).
    pub repo_root: Arc<PathBuf>,
    /// Durable data directory (`{repo_root}/data`).
    pub data_dir: Arc<PathBuf>,
    /// Server build version (`CARGO_PKG_VERSION`).
    pub version: Arc<str>,
    /// Process start time (health/uptime).
    pub started_at: SystemTime,
    /// Tracker box durable store.
    pub tracker: Arc<RwLock<TrackerStore>>,
    /// OmniRouter box (provider/model catalog, config, OpenAI-compatible proxy).
    pub omni: Arc<OmniRouter>,
    /// Per-session token usage (OmniRouter + MCP + OmniRoute snapshot).
    pub usage: Arc<RwLock<UsageStore>>,
    /// Currently selected IDE session (in-memory selection).
    pub ide_selection: Arc<RwLock<Option<crate::boxes::ide::IdeSelection>>>,
    /// Currently selected VDT product id (in-memory; from `/api/products/select`).
    pub product_selected: Arc<Mutex<Option<String>>>,
    /// `true` once an update notification has been received.
    pub update_flag: Arc<AtomicBool>,
    /// MCP `logging/setLevel` index into [`crate::mcp::LOG_LEVELS`] (default `info`).
    pub mcp_log_level: Arc<AtomicU8>,
    /// MCP `resources/subscribe` URIs (process-local allowlist).
    pub mcp_subscriptions: Arc<std::sync::RwLock<BTreeSet<String>>>,
    /// Pending MCP notifications (`notifications/message`, `notifications/resources/updated`).
    pub mcp_notifications: Arc<Mutex<Vec<Value>>>,
    /// HTTP Streamable MCP sessions (`Mcp-Session-Id` → seq + listed count). Process-local.
    pub mcp_sessions: Arc<std::sync::RwLock<BTreeMap<String, McpHttpSession>>>,
    /// Last `tools/list` length this process served (`0` = client has not listed yet).
    pub mcp_listed_tools: Arc<AtomicU32>,
    /// Ticket MCP presence (heartbeat). Isolated per process/`AppState`.
    pub ticket_presence: Arc<crate::boxes::tickets::PresenceStore>,
    /// Monotonic sequence for new HTTP MCP session ids.
    pub mcp_session_seq: Arc<AtomicU64>,
    /// SSE event broadcast channel (string payloads, JSON).
    pub events: broadcast::Sender<String>,
}

impl AppState {
    /// Build a new `AppState`.
    ///
    /// `repo_root` defaults to this crate root (`CARGO_MANIFEST_DIR`) when `None`.
    /// `data_dir` defaults to `{repo_root}/data` when `None`.
    pub fn new(
        repo_root: Option<PathBuf>,
        data_dir: Option<PathBuf>,
        events: broadcast::Sender<String>,
    ) -> Self {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let root = repo_root.unwrap_or_else(|| manifest_dir.clone());
        let data = data_dir.unwrap_or_else(|| root.join("data"));
        let tracker = TrackerStore::load(&root, &data).unwrap_or_default();
        let omni = OmniRouter::new(&data);
        let usage = crate::boxes::usage::load(&data);
        Self {
            repo_root: Arc::new(root),
            data_dir: Arc::new(data),
            version: Arc::from(crate::gsv_version()),
            started_at: SystemTime::now(),
            tracker: Arc::new(RwLock::new(tracker)),
            omni: Arc::new(omni),
            usage: Arc::new(RwLock::new(usage)),
            ide_selection: Arc::new(RwLock::new(None)),
            product_selected: Arc::new(Mutex::new(None)),
            update_flag: Arc::new(AtomicBool::new(false)),
            mcp_log_level: Arc::new(AtomicU8::new(1)), // info — see mcp::LOG_LEVELS
            mcp_subscriptions: Arc::new(std::sync::RwLock::new(BTreeSet::new())),
            mcp_notifications: Arc::new(Mutex::new(Vec::new())),
            mcp_sessions: Arc::new(std::sync::RwLock::new(BTreeMap::new())),
            mcp_listed_tools: Arc::new(AtomicU32::new(0)),
            mcp_session_seq: Arc::new(AtomicU64::new(1)),
            ticket_presence: Arc::new(crate::boxes::tickets::new_presence_store()),
            events,
        }
    }

    /// Queue a JSON-RPC notification for the next stdio flush.
    pub fn push_mcp_notification(&self, value: Value) {
        if let Ok(mut q) = self.mcp_notifications.lock() {
            q.push(value);
            const CAP: usize = 64;
            if q.len() > CAP {
                let drop_n = q.len() - CAP;
                q.drain(0..drop_n);
            }
        }
    }

    /// Take queued MCP notifications (stdio NDJSON; HTTP SSE when Accept asks).
    pub fn drain_mcp_notifications(&self) -> Vec<Value> {
        self.mcp_notifications
            .lock()
            .map(|mut q| q.drain(..).collect())
            .unwrap_or_default()
    }

    /// Sorted snapshot of `resources/subscribe` URIs.
    pub fn mcp_subscription_list(&self) -> Vec<String> {
        self.mcp_subscriptions
            .read()
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Issue a process-local HTTP MCP session id (`Mcp-Session-Id`).
    pub fn mcp_issue_session(&self) -> String {
        let seq = self.mcp_session_seq.fetch_add(1, Ordering::Relaxed);
        let id = crate::mcp::new_mcp_session_id(seq);
        if let Ok(mut map) = self.mcp_sessions.write() {
            map.insert(
                id.clone(),
                McpHttpSession {
                    seq,
                    listed_tools: 0,
                },
            );
            let cap = crate::mcp::MCP_SESSION_CAP;
            while map.len() > cap {
                let oldest = map
                    .iter()
                    .min_by_key(|(_, s)| s.seq)
                    .map(|(k, _)| k.clone());
                match oldest {
                    Some(k) => {
                        map.remove(&k);
                    }
                    None => break,
                }
            }
        }
        id
    }

    /// True when `id` is a live HTTP MCP session.
    pub fn mcp_session_ok(&self, id: &str) -> bool {
        crate::mcp::valid_mcp_session_id(id)
            && self
                .mcp_sessions
                .read()
                .map(|m| m.contains_key(id))
                .unwrap_or(false)
    }

    /// Drop an HTTP MCP session. Returns whether it existed.
    pub fn mcp_session_delete(&self, id: &str) -> bool {
        self.mcp_sessions
            .write()
            .map(|mut m| m.remove(id).is_some())
            .unwrap_or(false)
    }

    /// Count of live HTTP MCP sessions.
    pub fn mcp_session_count(&self) -> usize {
        self.mcp_sessions.read().map(|m| m.len()).unwrap_or(0)
    }

    /// Record that `tools/list` returned `n` tools (process-wide + optional session).
    pub fn mcp_mark_listed(&self, session: Option<&str>, n: u32) {
        self.mcp_listed_tools.store(n, Ordering::Relaxed);
        let Some(id) = session else {
            return;
        };
        if let Ok(mut map) = self.mcp_sessions.write() {
            if let Some(row) = map.get_mut(id) {
                row.listed_tools = n;
            }
        }
    }

    /// Last `tools/list` length this process served (`0` = none yet).
    pub fn mcp_listed_tool_count(&self) -> u32 {
        self.mcp_listed_tools.load(Ordering::Relaxed)
    }

    /// How many HTTP sessions have called `tools/list`.
    pub fn mcp_session_listed_count(&self) -> usize {
        self.mcp_sessions
            .read()
            .map(|m| m.values().filter(|s| s.listed_tools > 0).count())
            .unwrap_or(0)
    }

    /// Reset the update flag (used after a UI "Update" handshake).
    pub fn clear_update(&self) {
        self.update_flag.store(false, Ordering::SeqCst);
    }

    /// Read the update flag.
    pub fn update_available(&self) -> bool {
        self.update_flag.load(Ordering::SeqCst)
    }

    /// Emit an SSE event to all connected `/events` clients.
    pub fn emit(&self, event: impl Into<String>) {
        let _ = self.events.send(event.into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> AppState {
        let (tx, _rx) = broadcast::channel(16);
        AppState::new(None, None, tx)
    }

    #[test]
    fn default_repo_root_is_gsv_crate() {
        let s = state();
        assert!(
            s.repo_root.ends_with("GSV"),
            "repo_root={} data_dir={}",
            s.repo_root.display(),
            s.data_dir.display()
        );
        assert!(s.data_dir.ends_with("GSV/data") || s.data_dir.ends_with("GSV\\data"));
    }

    #[test]
    fn update_flag_toggle() {
        let s = state();
        assert!(!s.update_available());
        s.update_flag.store(true, Ordering::SeqCst);
        assert!(s.update_available());
        s.clear_update();
        assert!(!s.update_available());
    }
}
