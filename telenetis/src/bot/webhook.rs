use crate::bot::commands::{command_response, parse_command, Command};
use crate::bot::telegram::TelegramBot;
use crate::state::{AppState, FlowEvent};
use axum::{extract::State, routing::post, Json, Router};
use chrono::Utc;
use serde_json::Value;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/webhook", post(handle_webhook))
        .with_state(state)
}

#[derive(Debug, PartialEq)]
pub enum UpdateKind {
    Message,
    CallbackQuery,
    MyChatMember,
    Unknown,
}

pub fn classify_update(value: &Value) -> UpdateKind {
    if value.get("message").is_some() {
        UpdateKind::Message
    } else if value.get("callback_query").is_some() {
        UpdateKind::CallbackQuery
    } else if value.get("my_chat_member").is_some() {
        UpdateKind::MyChatMember
    } else {
        UpdateKind::Unknown
    }
}

fn extract_message_text(update: &Value) -> Option<String> {
    update
        .get("message")
        .and_then(|m| m.get("text"))
        .and_then(|t| t.as_str())
        .map(|s| s.to_string())
}

async fn handle_webhook(State(state): State<AppState>, Json(update): Json<Value>) -> &'static str {
    let kind = classify_update(&update);
    tracing::info!("Telegram update kind={:?}", kind);

    match kind {
        UpdateKind::Message => {
            if let Some(text) = extract_message_text(&update) {
                let chat_id = update
                    .get("message")
                    .and_then(|m| m.get("chat"))
                    .and_then(|c| c.get("id"))
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let from = update
                    .get("message")
                    .and_then(|m| m.get("from"))
                    .and_then(|f| f.get("username"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");

                let response_text = if let Some(cmd_str) = parse_command(&text) {
                    let base = cmd_str.split('@').next().unwrap_or(&cmd_str);
                    let cmd = Command::from_str(base);
                    command_response(&cmd)
                } else {
                    format!("Echo: {}", text)
                };

                state
                    .push_flow(FlowEvent {
                        ts: Utc::now(),
                        jail_id: state.jail_id().to_string(),
                        action: "telegram_message".to_string(),
                        detail: format!("chat={} from={} text={}", chat_id, from, text),
                    })
                    .await;

                tracing::info!("Reply to {}: {}", chat_id, response_text);

                let bot = TelegramBot::new(state.config());
                if let Err(e) = bot.send_message(chat_id, &response_text).await {
                    tracing::warn!("Failed to send reply to {chat_id}: {e}");
                }
            }
        }
        UpdateKind::CallbackQuery => {
            let data = update
                .get("callback_query")
                .and_then(|c| c.get("data"))
                .and_then(|d| d.as_str())
                .unwrap_or("");
            state
                .push_flow(FlowEvent {
                    ts: Utc::now(),
                    jail_id: state.jail_id().to_string(),
                    action: "callback_query".to_string(),
                    detail: data.to_string(),
                })
                .await;
        }
        UpdateKind::MyChatMember => {
            state
                .push_flow(FlowEvent {
                    ts: Utc::now(),
                    jail_id: state.jail_id().to_string(),
                    action: "my_chat_member".to_string(),
                    detail: "membership update".to_string(),
                })
                .await;
        }
        UpdateKind::Unknown => {
            tracing::debug!("Unknown Telegram update: {:?}", update);
        }
    }

    "ok"
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn classify_message() {
        let v = json!({"message": {"text": "hi"}});
        assert_eq!(classify_update(&v), UpdateKind::Message);
    }

    #[test]
    fn classify_callback() {
        let v = json!({"callback_query": {"data": "x"}});
        assert_eq!(classify_update(&v), UpdateKind::CallbackQuery);
    }

    #[test]
    fn classify_my_chat_member() {
        let v = json!({"my_chat_member": {"chat": {}}});
        assert_eq!(classify_update(&v), UpdateKind::MyChatMember);
    }

    #[test]
    fn classify_unknown() {
        let v = json!({"inline_query": {}});
        assert_eq!(classify_update(&v), UpdateKind::Unknown);
    }

    #[test]
    fn extract_text_present() {
        let v = json!({"message": {"text": "/start"}});
        assert_eq!(extract_message_text(&v), Some("/start".to_string()));
    }

    #[test]
    fn extract_text_missing() {
        let v = json!({"message": {}});
        assert_eq!(extract_message_text(&v), None);
    }
}
