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

    Some(envelope)
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
                    // ticket sync every 30s (every 6 ticks of 5s)
                    if tick.is_multiple_of(6) {
                        if let Ok(tval) = client.tickets().await {
                            let _ = crate::gsv::tickets::sync_tickets(&client, &state).await;
                            let _ = tval;
                        }
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
    async fn classify_unknown_kind() {
        assert_eq!(classify_envelope_kind("foobar"), "unknown");
        assert_eq!(classify_envelope_kind("sync"), "sync");
    }
}
