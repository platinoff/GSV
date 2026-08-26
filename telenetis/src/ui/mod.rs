use crate::state::AppState;
use axum::{routing::get, Json, Router};
use serde_json::json;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(dashboard))
        .route("/health", get(health))
        .route("/api/status", get(status))
        .with_state(state)
}

async fn dashboard() -> &'static str {
    "Telenetis - Telegram Mini App for GSV"
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "service": "telenetis",
        "version": "0.1.0"
    }))
}

async fn status(state: axum::extract::State<AppState>) -> Json<serde_json::Value> {
    let tickets = state.tickets().await;
    let presence = state.presence_map().await;
    let flows = state.recent_flows(10).await;

    Json(json!({
        "online": state.is_online(),
        "jail_id": state.jail_id(),
        "tickets_count": tickets.len(),
        "workers_online": presence.len(),
        "recent_flows": flows.len(),
    }))
}
