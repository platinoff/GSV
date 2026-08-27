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
    tunnel_url: Arc<RwLock<Option<String>>>,
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
            tunnel_url: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn set_tunnel_url(&self, url: String) {
        *self.tunnel_url.write().await = Some(url);
    }

    pub async fn tunnel_url(&self) -> Option<String> {
        self.tunnel_url.read().await.clone()
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
        let mut queue = self.bus_queue.write().await;
        queue.push(env);
        if queue.len() > 1000 {
            queue.drain(0..500);
        }
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

/// Register the bot itself (Telenetis jail) as an online worker so `/roles`
/// and `/status` show `@GsvOfficialBot` as Ready even when no other worker is
/// connected. Refreshed on a heartbeat and via `/reconnect`.
pub fn register_self_presence(state: &AppState) {
    let presence = worker_presence(
        state.jail_id(),
        "bot",
        "telegram",
        "n/a",
        "telenetis",
        2,
        WorkerStatus::Ready,
    );
    let state = state.clone();
    tokio::spawn(async move {
        state.update_presence(presence).await;
    });
}

/// Build a [`WorkerPresence`] with the current timestamp and UTC timezone.
pub fn worker_presence(
    jail_id: &str,
    actor: &str,
    ide: &str,
    model: &str,
    agent: &str,
    rank: u8,
    status: WorkerStatus,
) -> WorkerPresence {
    WorkerPresence {
        jail_id: jail_id.to_string(),
        actor: actor.to_string(),
        ide: ide.to_string(),
        model: model.to_string(),
        agent: agent.to_string(),
        rank,
        status,
        last_heartbeat: chrono::Utc::now(),
        timezone: "UTC".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    fn test_config() -> Config {
        Config {
            bot_token: "test".to_string(),
            gsv_url: "http://127.0.0.1:9999".to_string(),
            port: 9800,
            jail_id: "test-jail".to_string(),
            godfather_channel_id: 0,
            webhook_url: None,
            public_url: None,
            tunnel_enabled: false,
            ngrok_bin: None,
        }
    }

    #[tokio::test]
    async fn appstate_creation_and_jail_id() {
        let state = AppState::new(test_config());
        assert_eq!(state.jail_id(), "test-jail");
        assert!(state.is_online());
    }

    #[tokio::test]
    async fn push_flow_caps_at_1000() {
        let state = AppState::new(test_config());
        for i in 0..1005 {
            state
                .push_flow(FlowEvent {
                    ts: Utc::now(),
                    jail_id: "jail".to_string(),
                    action: format!("action_{}", i),
                    detail: "detail".to_string(),
                })
                .await;
        }
        let flows = state.recent_flows(2000).await;
        assert!(flows.len() <= 1000);
        assert!(flows.len() >= 500);
    }

    #[tokio::test]
    async fn update_presence_and_tickets() {
        let state = AppState::new(test_config());
        state
            .update_presence(WorkerPresence {
                jail_id: "jail-02".to_string(),
                actor: "agent".to_string(),
                ide: "cursor".to_string(),
                model: "test".to_string(),
                agent: "orchestrator".to_string(),
                rank: 5,
                status: WorkerStatus::Ready,
                last_heartbeat: Utc::now(),
                timezone: "UTC".to_string(),
            })
            .await;
        let map = state.presence_map().await;
        assert_eq!(map.len(), 1);
        state
            .set_tickets(vec![TicketRow {
                id: "t-1".to_string(),
                title: "Test".to_string(),
                body: "body".to_string(),
                status: "open".to_string(),
                product: "gsv".to_string(),
                claimed_by: None,
                scenario: None,
            }])
            .await;
        let tickets = state.tickets().await;
        assert_eq!(tickets.len(), 1);
    }
}
