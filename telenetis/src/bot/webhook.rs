use crate::state::AppState;
use axum::{routing::post, Json, Router};
use serde_json::Value;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/webhook", post(handle_webhook))
        .with_state(state)
}

async fn handle_webhook(
    _state: axum::extract::State<AppState>,
    Json(update): Json<Value>,
) -> &'static str {
    tracing::info!("Received Telegram update: {:?}", update.get("message"));
    "ok"
}
