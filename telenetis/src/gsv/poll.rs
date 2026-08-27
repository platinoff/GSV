use crate::bot::telegram::TelegramBot;
use crate::gsv::client::GsvClient;
use crate::state::{BusEnvelope, FlowEvent};
use chrono::Utc;

fn classify_envelope_kind(kind: &str) -> &'static str {
    match kind {
        "presence" => "presence",
        "claim" => "claim",
        "done" => "done",
        "reclaim" => "reclaim",
        "sync" => "sync",
        "bus" => "bus",
        _ => "unknown",
    }
}

fn format_bus_for_telegram(env: &BusEnvelope) -> String {
    let icon = match env.kind.as_str() {
        "presence" => "🟢",
        "claim" => "📌",
        "done" => "✅",
        "reclaim" => "🔄",
        "sync" => "🔁",
        "bus" => "💬",
        _ => "📨",
    };
    let data_hint = env
        .data
        .as_ref()
        .and_then(|d| d.get("hint"))
        .and_then(|h| h.as_str())
        .map(|h| format!(" ({h})"))
        .unwrap_or_default();
    format!(
        "{icon} *{}* `{}` — {}{data_hint}",
        env.kind, env.from, env.body
    )
}

pub async fn handle_bus_value(
    value: &serde_json::Value,
    state: &crate::state::AppState,
) -> Option<BusEnvelope> {
    let kind = value.get("kind")?.as_str()?.to_string();
    let body = value.get("body")?.as_str().unwrap_or("").to_string();
    let from = value.get("from")?.as_str().unwrap_or("unknown").to_string();
    let body_kind = classify_envelope_kind(&kind);

    let envelope = BusEnvelope {
        v: value.get("v").and_then(|v| v.as_u64()).unwrap_or(1) as u8,
        kind: kind.clone(),
        body: body.clone(),
        from: from.clone(),
        ts: Utc::now(),
        data: value.get("data").cloned(),
    };

    state.push_bus(envelope.clone()).await;

    let detail = format!("[{}] {}: {}", body_kind, from, body);
    state
        .push_flow(FlowEvent {
            ts: Utc::now(),
            jail_id: from.clone(),
            action: body_kind.to_string(),
            detail,
        })
        .await;

    let channel_id = state.config().godfather_channel_id;
    if channel_id != 0 && from != state.jail_id() {
        let text = format_bus_for_telegram(&envelope);
        let bot = TelegramBot::new(state.config());
        if let Err(e) = bot.send_message(channel_id, &text).await {
            tracing::warn!("Failed to forward bus to Telegram channel: {e}");
        }
    }

    Some(envelope)
}

pub async fn post_bus_envelope(
    state: &crate::state::AppState,
    kind: &str,
    body: &str,
    ticket_id: Option<&str>,
) -> Result<(), crate::error::TelenetisError> {
    let client = GsvClient::new(state.config());
    let payload = serde_json::json!({
        "v": 1,
        "kind": kind,
        "body": body,
        "from": state.jail_id(),
        "ticket_id": ticket_id,
    });
    let url = format!("{}/api/telegram/bus", client.base_url());
    let resp = reqwest::Client::new()
        .post(&url)
        .json(&payload)
        .send()
        .await?;
    let status = resp.status();
    if !status.is_success() {
        return Err(crate::error::TelenetisError::Gsv(format!(
            "HTTP {status} posting bus envelope"
        )));
    }
    Ok(())
}

pub fn spawn_poll_loop(client: GsvClient, state: crate::state::AppState) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
        let mut tick: u32 = 0;
        loop {
            interval.tick().await;
            tick = tick.wrapping_add(1);
            match client.bus_poll(8).await {
                Ok(value) => {
                    let envelopes =
                        if let Some(arr) = value.get("envelopes").and_then(|v| v.as_array()) {
                            arr.clone()
                        } else if let Some(arr) = value.as_array() {
                            arr.clone()
                        } else {
                            vec![]
                        };
                    for env in envelopes {
                        handle_bus_value(&env, &state).await;
                    }
                    if tick.is_multiple_of(6) {
                        let _ = crate::gsv::tickets::sync_tickets(&client, &state).await;
                    }
                }
                Err(e) => {
                    tracing::debug!("poll bus error: {:?}", e);
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::state::AppState;

    fn test_config() -> Config {
        Config {
            bot_token: "test".to_string(),
            gsv_url: "http://127.0.0.1:9999".to_string(),
            port: 9800,
            jail_id: "test-jail".to_string(),
            godfather_channel_id: 0,
            webhook_url: None,
        }
    }

    #[tokio::test]
    async fn handle_bus_value_pushes_flow() {
        let state = AppState::new(test_config());
        let value = serde_json::json!({
            "v": 1,
            "kind": "presence",
            "body": "hello from jail-02",
            "from": "jail-02",
            "data": {"hint": "test"}
        });
        let env = handle_bus_value(&value, &state).await;
        assert!(env.is_some());
        let flows = state.recent_flows(5).await;
        assert_eq!(flows.len(), 1);
        assert_eq!(flows[0].action, "presence");
        let queue = state.bus_queue().await;
        assert_eq!(queue.len(), 1);
    }

    #[tokio::test]
    async fn handle_bus_value_no_telegram_when_channel_zero() {
        let state = AppState::new(test_config());
        let value = serde_json::json!({
            "v": 1,
            "kind": "bus",
            "body": "test",
            "from": "other-jail"
        });
        let env = handle_bus_value(&value, &state).await;
        assert!(env.is_some());
    }

    #[tokio::test]
    async fn handle_bus_value_skips_own_jail_forwarding() {
        let cfg = Config {
            godfather_channel_id: 12345,
            ..test_config()
        };
        let state = AppState::new(cfg);
        let value = serde_json::json!({
            "v": 1,
            "kind": "bus",
            "body": "own message",
            "from": "test-jail"
        });
        let env = handle_bus_value(&value, &state).await;
        assert!(env.is_some());
    }

    #[test]
    fn classify_unknown_kind() {
        assert_eq!(classify_envelope_kind("foobar"), "unknown");
        assert_eq!(classify_envelope_kind("sync"), "sync");
    }

    #[test]
    fn format_bus_for_telegram_icons() {
        let env = BusEnvelope {
            v: 1,
            kind: "claim".to_string(),
            body: "claimed T-1".to_string(),
            from: "jail-02".to_string(),
            ts: Utc::now(),
            data: None,
        };
        let text = format_bus_for_telegram(&env);
        assert!(text.contains("📌"));
        assert!(text.contains("claim"));
        assert!(text.contains("jail-02"));
    }

    #[test]
    fn format_bus_for_telegram_with_hint() {
        let env = BusEnvelope {
            v: 1,
            kind: "sync".to_string(),
            body: "syncing".to_string(),
            from: "jail-03".to_string(),
            ts: Utc::now(),
            data: Some(serde_json::json!({"hint": "memory-disk-speed"})),
        };
        let text = format_bus_for_telegram(&env);
        assert!(text.contains("memory-disk-speed"));
    }
}
