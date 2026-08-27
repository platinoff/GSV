use axum::extract::ws::{Message, WebSocket};
use axum::extract::{State, WebSocketUpgrade};
use axum::response::IntoResponse;
use futures::{SinkExt, StreamExt};

pub fn router(state: crate::state::AppState) -> axum::Router {
    axum::Router::new()
        .route("/ws", axum::routing::get(ws_handler))
        .with_state(state)
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<crate::state::AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: crate::state::AppState) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.flows_tx().subscribe();

    let mut send_task = tokio::spawn(async move {
        while let Ok(event) = rx.recv().await {
            let json = serde_json::to_string(&event).unwrap_or_default();
            if sender.send(Message::Text(json.into())).await.is_err() {
                break;
            }
        }
    });

    let mut recv_task =
        tokio::spawn(async move { while let Some(Ok(_)) = receiver.next().await {} });

    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::state::AppState;

    fn test_state() -> AppState {
        AppState::new(Config {
            bot_token: "test".to_string(),
            gsv_url: "http://127.0.0.1:9999".to_string(),
            port: 9800,
            jail_id: "test-jail".to_string(),
            godfather_channel_id: 0,
            webhook_url: None,
            public_url: None,
            tunnel_enabled: false,
            ngrok_bin: None,
        })
    }

    #[test]
    fn router_builds_without_panic() {
        let _app = router(test_state());
    }
}
