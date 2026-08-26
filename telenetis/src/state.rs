use crate::config::Config;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

#[derive(Debug, Clone)]
pub struct BusEnvelope {
    pub v: u8,
    pub kind: String,
    pub body: String,
    pub from: String,
    pub ts: DateTime<Utc>,
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct WorkerPresence {
    pub jail_id: String,
    pub actor: String,
    pub ide: String,
    pub model: String,
    pub agent: String,
    pub rank: u8,
    pub status: WorkerStatus,
    pub last_heartbeat: DateTime<Utc>,
    pub timezone: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum WorkerStatus {
    Ready,
    Busy,
    Offline,
}

#[derive(Debug, Clone)]
pub struct TicketRow {
    pub id: String,
    pub title: String,
    pub body: String,
    pub status: String,
    pub product: String,
    pub claimed_by: Option<String>,
    pub scenario: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct FlowEvent {
    pub ts: DateTime<Utc>,
    pub jail_id: String,
    pub action: String,
    pub detail: String,
}

#[derive(Clone)]
pub struct AppState {
    config: Config,
    bus_queue: Arc<RwLock<Vec<BusEnvelope>>>,
    presence: Arc<RwLock<HashMap<String, WorkerPresence>>>,
    tickets: Arc<RwLock<Vec<TicketRow>>>,
    flows: Arc<RwLock<Vec<FlowEvent>>>,
    flows_tx: broadcast::Sender<FlowEvent>,
    online: Arc<std::sync::atomic::AtomicBool>,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        let (flows_tx, _) = broadcast::channel(256);
        Self {
            config,
            bus_queue: Arc::new(RwLock::new(Vec::new())),
            presence: Arc::new(RwLock::new(HashMap::new())),
            tickets: Arc::new(RwLock::new(Vec::new())),
            flows: Arc::new(RwLock::new(Vec::new())),
            flows_tx,
            online: Arc::new(std::sync::atomic::AtomicBool::new(true)),
        }
    }

    pub fn jail_id(&self) -> &str {
        &self.config.jail_id
    }

    pub fn is_online(&self) -> bool {
        self.online.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub async fn push_bus(&self, env: BusEnvelope) {
        self.bus_queue.write().await.push(env);
    }

    pub async fn bus_queue(&self) -> Vec<BusEnvelope> {
        self.bus_queue.read().await.clone()
    }

    pub async fn update_presence(&self, presence: WorkerPresence) {
        self.presence
            .write()
            .await
            .insert(presence.jail_id.clone(), presence);
    }

    pub async fn presence_map(&self) -> HashMap<String, WorkerPresence> {
        self.presence.read().await.clone()
    }

    pub async fn set_tickets(&self, tickets: Vec<TicketRow>) {
        *self.tickets.write().await = tickets;
    }

    pub async fn tickets(&self) -> Vec<TicketRow> {
        self.tickets.read().await.clone()
    }

    pub async fn push_flow(&self, event: FlowEvent) {
        let mut flows = self.flows.write().await;
        flows.push(event.clone());
        if flows.len() > 1000 {
            flows.drain(0..500);
        }
        let _ = self.flows_tx.send(event);
    }

    pub fn flows_tx(&self) -> &broadcast::Sender<FlowEvent> {
        &self.flows_tx
    }

    pub async fn recent_flows(&self, limit: usize) -> Vec<FlowEvent> {
        let flows = self.flows.read().await;
        flows.iter().rev().take(limit).cloned().collect()
    }
}
