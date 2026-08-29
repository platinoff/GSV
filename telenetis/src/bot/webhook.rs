use crate::bot::commands::{handle_command, Command};
use crate::bot::mini_app::mini_app_url;
use crate::bot::telegram::TelegramBot;
use crate::state::{AppState, FlowEvent};
use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use chrono::Utc;
use serde_json::Value;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/webhook", post(handle_webhook))
        .with_state(state)
}

/// Constant-time comparison of two byte strings (avoids leaking prefix length
/// details through timing when the webhook secret is checked).
fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// True only when the incoming webhook carries the configured secret in
/// `X-Telegram-Bot-Api-Secret-Token`. When no secret is configured the check
/// is a no-op (secret-less webhook setup is still supported, but not advised).
fn webhook_secret_ok(headers: &axum::http::HeaderMap, configured: Option<&str>) -> bool {
    match configured {
        None => true,
        Some(secret) => headers
            .get("X-Telegram-Bot-Api-Secret-Token")
            .and_then(|v| v.to_str().ok())
            .map(|s| ct_eq(s.as_bytes(), secret.as_bytes()))
            .unwrap_or(false),
    }
}

async fn handle_webhook(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(update): Json<Value>,
) -> (StatusCode, &'static str) {
    if !webhook_secret_ok(&headers, state.config().webhook_secret.as_deref()) {
        tracing::warn!("webhook rejected (missing or mismatched secret token)");
        return (StatusCode::FORBIDDEN, "forbidden");
    }
    process_update(&state, &update).await;
    (StatusCode::OK, "ok")
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

/// Telegram assigns positive ids to private chats; channels and groups are
/// negative (`-100…` for channels/supergroups). `web_app` buttons are only
/// accepted in private chats.
pub fn is_private_chat(chat_id: i64) -> bool {
    chat_id > 0
}

/// Cold-start warm-up for the Mini App (plan P3). When the owner taps the
/// bot's `Start` web-app button, `process_update` syncs the board from GSV so
/// the snapshot the WebView fetches is already fresh, and records a
/// `cold_start` flow event so the live log shows the warm-up. Best-effort: a
/// GSV outage must not block the reply, so the sync failure is swallowed and
/// only reflected in the flow detail. Returns the number of tickets now on
/// the board.
pub async fn warm_start(state: &AppState) -> usize {
    let client = crate::gsv::client::GsvClient::new(state.config());
    let synced = crate::gsv::tickets::sync_tickets(&client, state)
        .await
        .is_ok();
    let count = state.tickets().await.len();
    state
        .push_flow(FlowEvent {
            ts: Utc::now(),
            jail_id: state.jail_id().to_string(),
            action: "cold_start".to_string(),
            detail: format!("prefetch synced={synced} tickets={count}"),
        })
        .await;
    count
}

/// Shared update handler for both the webhook route and the long-poll loop.
pub async fn process_update(state: &AppState, update: &Value) {
    let kind = classify_update(update);
    tracing::info!("Telegram update kind={:?}", kind);

    match kind {
        UpdateKind::Message => {
            if let Some(text) = extract_message_text(update) {
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

                let cmd = Command::from_text(&text);
                let response_text = handle_command(&cmd, state).await;

                if matches!(cmd, Command::Start) {
                    // Cold-start prefetch (plan P3): the first interaction after
                    // the Mini App button opens syncs the board from GSV so the
                    // WebView snapshot is already fresh, and records the warm-up
                    // in the live log. Best-effort — an unreachable GSV must not
                    // block the reply.
                    let warmed = warm_start(state).await;
                    tracing::info!("cold start: prefetched {} tickets", warmed);
                }

                state
                    .push_flow(FlowEvent {
                        ts: Utc::now(),
                        jail_id: state.jail_id().to_string(),
                        action: "telegram_message".to_string(),
                        detail: format!("chat={} from={} cmd={}", chat_id, from, text),
                    })
                    .await;

                tracing::info!("Reply to {}: {}", chat_id, response_text);

                let bot = TelegramBot::new(state.config());
                match &cmd {
                    Command::Start | Command::Help | Command::App => {
                        let base = match state.tunnel_url().await {
                            Some(url) => url,
                            None => match &state.config().public_url {
                                Some(pub_url) => pub_url.trim_end_matches('/').to_string(),
                                None => format!("http://127.0.0.1:{}", state.config().port),
                            },
                        };
                        let url = mini_app_url(&base);
                        let res = if !url.starts_with("https://") {
                            Err(crate::error::TelenetisError::Tunnel(
                                "Mini App requires HTTPS tunnel URL (run /tunnel)".to_string(),
                            ))
                        } else if is_private_chat(chat_id) {
                            // Private chats support embedded `web_app` buttons.
                            bot.send_mini_app(chat_id, &response_text, &url).await
                        } else {
                            // Channels/groups reject `web_app` buttons
                            // (BUTTON_TYPE_INVALID); a direct-link Mini App URL
                            // still opens the app embedded there.
                            let username = bot.get_me().await.unwrap_or_default();
                            let app_link = format!("https://t.me/{username}?startapp=telenetis");
                            bot.send_url_button(
                                chat_id,
                                &response_text,
                                "Open Telenetis",
                                &app_link,
                            )
                            .await
                        };
                        if let Err(e) = res {
                            let fallback_text = format!("{}\n\n⚠️ Mini App requires HTTPS tunnel URL.\nURL: {}\nError: {e}\n\nTip: Run /tunnel to start ngrok.", response_text, url);
                            if let Err(err) = bot.send_message(chat_id, &fallback_text).await {
                                tracing::warn!("Failed to send fallback reply to {chat_id}: {err}");
                            }
                        }
                    }
                    _ => {
                        if let Err(e) = bot.send_message(chat_id, &response_text).await {
                            tracing::warn!("Failed to send reply to {chat_id}: {e}");
                        }
                    }
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
}

/// Long-poll Telegram for updates when no public webhook URL is configured.
/// Runs until the process exits; tolerates transient API errors. A `409
/// Conflict` means an active webhook still owns the bot, so we re-remove it
/// (Telegram needs a moment) before resuming polling.
pub async fn run_polling(state: AppState) {
    if state.config().bot_token.is_empty() {
        return;
    }
    let bot = TelegramBot::new(state.config());
    if let Err(e) = bot.delete_webhook().await {
        tracing::warn!("Failed to delete webhook before polling: {e}");
    }
    let mut offset: i64 = 0;
    loop {
        let updates = match bot.get_updates(offset, 30).await {
            Ok(v) => v,
            Err(e) if e.to_string().contains("409") => {
                tracing::warn!(
                    "getUpdates 409 Conflict — a webhook owns the bot; removing it and retrying (detail: {e})"
                );
                if let Err(de) = bot.delete_webhook().await {
                    tracing::warn!("deleteWebhook failed: {de}");
                }
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                continue;
            }
            Err(e) => {
                tracing::warn!("getUpdates failed: {e} — retrying in 3s");
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                continue;
            }
        };
        let Some(list) = updates.get("result").and_then(|r| r.as_array()) else {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            continue;
        };
        for upd in list {
            if let Some(id) = upd.get("update_id").and_then(|v| v.as_i64()) {
                offset = id + 1;
            }
            process_update(&state, upd).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn test_config() -> crate::config::Config {
        crate::config::Config {
            bot_token: "test".to_string(),
            gsv_url: "http://127.0.0.1:9999".to_string(),
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

    #[test]
    fn private_chat_detection() {
        assert!(is_private_chat(123_456));
        assert!(!is_private_chat(-1_003_872_035_653));
        assert!(!is_private_chat(-42));
    }

    #[tokio::test]
    async fn warm_start_pushes_cold_start_flow() {
        let state = crate::state::AppState::new(test_config());
        let count = warm_start(&state).await;
        assert_eq!(count, 0);
        let flows = state.recent_flows(5).await;
        assert_eq!(flows.len(), 1);
        assert_eq!(flows[0].action, "cold_start");
        assert!(flows[0].detail.contains("prefetch"));
    }

    #[tokio::test]
    async fn warm_start_reflects_seeded_tickets() {
        let state = crate::state::AppState::new(test_config());
        state
            .set_tickets(vec![crate::state::TicketRow {
                id: "T-1".to_string(),
                title: "Task".to_string(),
                body: String::new(),
                status: "open".to_string(),
                product: "gsv".to_string(),
                claimed_by: None,
                scenario: None,
            }])
            .await;
        let count = warm_start(&state).await;
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn warm_start_tolerates_unreachable_gsv() {
        let mut cfg = test_config();
        cfg.gsv_url = "http://127.0.0.1:9".to_string();
        let state = crate::state::AppState::new(cfg);
        // An unreachable GSV must not panic or block the reply — the warm-up
        // still records the attempt and returns the (empty) board length.
        let count = warm_start(&state).await;
        assert_eq!(count, 0);
        let flows = state.recent_flows(5).await;
        assert_eq!(flows.len(), 1);
        assert_eq!(flows[0].action, "cold_start");
    }

    // ---- band 222: webhook secret-token auth ----

    #[test]
    fn ct_eq_compares_bytes() {
        assert!(ct_eq(b"secret", b"secret"));
        assert!(!ct_eq(b"secret", b"secrey"));
        assert!(!ct_eq(b"secret", b"secret2"));
        assert!(!ct_eq(b"", b"x"));
        assert!(ct_eq(b"", b""));
    }

    #[test]
    fn webhook_secret_ok_passes_when_matching() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("X-Telegram-Bot-Api-Secret-Token", "s3cret".parse().unwrap());
        assert!(webhook_secret_ok(&headers, Some("s3cret")));
    }

    #[test]
    fn webhook_secret_ok_rejects_wrong_or_missing() {
        let mut headers = axum::http::HeaderMap::new();
        // Missing header with a configured secret → rejected.
        assert!(!webhook_secret_ok(&headers, Some("s3cret")));
        headers.insert("X-Telegram-Bot-Api-Secret-Token", "wrong".parse().unwrap());
        assert!(!webhook_secret_ok(&headers, Some("s3cret")));
        // No secret configured → any request is accepted (no-op gate).
        assert!(webhook_secret_ok(&headers, None));
    }

    #[tokio::test]
    async fn webhook_rejects_forged_update_when_secret_configured() {
        use axum::body::Body;
        use axum::http::Request;
        use tower::ServiceExt;

        let mut cfg = test_config();
        cfg.webhook_secret = Some("s3cret".to_string());
        let app = router(crate::state::AppState::new(cfg));
        // Callback-query update: handled locally (flow push), never makes an
        // outbound Telegram call, so the test stays hermetic and fast.
        let body = json!({
            "update_id": 1,
            "callback_query": {"id": "42", "data": "x"}
        });

        // Missing secret header → 403, update not processed.
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/webhook")
                    .method("POST")
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);

        // With the correct secret header → 200.
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/webhook")
                    .method("POST")
                    .header("content-type", "application/json")
                    .header("X-Telegram-Bot-Api-Secret-Token", "s3cret")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
