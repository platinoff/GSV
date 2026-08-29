use crate::bot::telegram::TelegramBot;
use crate::gsv::client::GsvClient;
use crate::state::{BusEnvelope, FlowEvent, WorkerPresence, WorkerStatus};
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

/// GSV rank ladder ids in ascending level order (mirrors GSV `ranks.rs` LADDER).
const RANK_LADDER: &[&str] = &[
    "jun-nub",
    "intern-private",
    "trainee-soldier",
    "junior-corporal",
    "associate-sergeant",
    "middle-staff",
    "senior-nco",
    "lead-warrant",
    "staff-lieutenant",
    "senior-lt",
    "principal-captain",
    "architect-major",
    "distinguished-ltcol",
    "fellow-colonel",
    "general-fellow",
    "marshal-orchestrator",
];

/// Map a GSV `rank_id` string (e.g. `"marshal-orchestrator"`) to its numeric
/// level, or a best-effort fallback when the id is unknown.
fn rank_id_to_level(rank_id: &str) -> u8 {
    if let Some(pos) = RANK_LADDER.iter().position(|id| *id == rank_id) {
        return pos as u8;
    }
    rank_id
        .trim_end_matches(|c: char| c.is_ascii_digit())
        .parse::<u8>()
        .unwrap_or(0)
}

/// Extract a presence worker from a GSV bus envelope `data` object. Returns
/// `None` when the envelope does not carry enough to identify a worker.
fn presence_from_envelope(value: &serde_json::Value) -> Option<WorkerPresence> {
    let data = value.get("data")?;
    let jail_id = data
        .get("jail_id")
        .and_then(|v| v.as_str())
        .unwrap_or(value.get("from")?.as_str()?);
    let rank_id = data.get("rank_id").and_then(|v| v.as_str()).unwrap_or("");
    Some(WorkerPresence {
        jail_id: jail_id.to_string(),
        actor: data
            .get("actor")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string(),
        ide: data
            .get("ide")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string(),
        model: data
            .get("model")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string(),
        agent: data
            .get("agent")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string(),
        rank: rank_id_to_level(rank_id),
        status: WorkerStatus::Ready,
        last_heartbeat: Utc::now(),
        timezone: "UTC".to_string(),
    })
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

    if kind == "presence" {
        if let Some(worker) = presence_from_envelope(value) {
            state.update_presence(worker).await;
        }
    }

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
    let resp = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
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

/// Extract pending bus envelopes from a GSV `GET /api/telegram/bus` response.
///
/// GSV returns `{"ok":true, "dry_run":…, "messages":[ …envelopes… ]}` on every
/// path (dry-run stub queue and live `getUpdates`). Earlier builds wrongly read
/// `envelopes` (or a top-level array) and so always got an empty result against
/// real GSV — the bus→Telegram forward / presence / flows pipeline never fired.
/// The `messages` key is the canonical shape; a bare top-level array is kept as
/// a legacy robustness fallback but is not the contract.
pub fn extract_messages(value: &serde_json::Value) -> Vec<serde_json::Value> {
    if let Some(arr) = value.get("messages").and_then(|v| v.as_array()) {
        return arr.clone();
    }
    if let Some(arr) = value.as_array() {
        return arr.clone();
    }
    vec![]
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
                    for env in extract_messages(&value) {
                        handle_bus_value(&env, &state).await;
                    }
                    if tick.is_multiple_of(6) {
                        let _ = crate::gsv::tickets::sync_tickets(&client, &state).await;
                    }
                }
                Err(e) => {
                    // A GSV outage is the single most common reason the bridge
                    // goes quiet — it must be visible at the default log level,
                    // not only under debug tracing.
                    tracing::warn!("poll bus error: {:?}", e);
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
            webhook_secret: None,
            public_url: None,
            tunnel_enabled: false,
            ngrok_bin: None,
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

    // ---- band 222: bus wire-contract fix ----
    // GSV `GET /api/telegram/bus` returns {ok, dry_run, messages:[envelopes]}
    // in both the dry-run stub queue and the live getUpdates path. The poll
    // loop MUST read `messages`, not `envelopes`. These tests pin the contract.

    #[test]
    fn extract_messages_reads_gsv_messages_key() {
        let value = serde_json::json!({
            "ok": true,
            "dry_run": false,
            "messages": [
                {"v":1,"kind":"presence","body":"jail-02","from":"jail-02"},
                {"v":1,"kind":"done","body":"done T-1","from":"jail-03"}
            ]
        });
        let msgs = extract_messages(&value);
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0]["kind"], "presence");
        assert_eq!(msgs[1]["kind"], "done");
    }

    #[test]
    fn extract_messages_ignores_legacy_envelopes_key() {
        // The pre-fix shape would have silently decoded nothing — the object
        // carries `messages` (canonical), never a bare `envelopes` array. A
        // stray `envelopes` key must NOT be read.
        let value = serde_json::json!({
            "ok": true,
            "messages": [{"v":1,"kind":"sync","from":"jail-01"}],
            "envelopes": [{"v":1,"kind":"ignored","from":"legacy"}]
        });
        let msgs = extract_messages(&value);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0]["kind"], "sync");
    }

    #[test]
    fn extract_messages_empty_object_yields_none() {
        assert!(extract_messages(&serde_json::json!({})).is_empty());
    }

    #[test]
    fn extract_messages_tolerates_bare_top_level_array() {
        // Legacy robustness fallback (pre-object shape) must still work.
        let value = serde_json::json!([{"v":1,"kind":"bus","from":"jail-09"}]);
        let msgs = extract_messages(&value);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0]["kind"], "bus");
    }

    #[tokio::test]
    async fn handle_bus_value_parses_gsv_messages_envelope() {
        // Feed the exact envelope shape GSV puts on the wire into the handler
        // and confirm it produces a flow — i.e. the bridge actually fires.
        let state = AppState::new(test_config());
        let value = serde_json::json!({
            "v": 1,
            "kind": "presence",
            "body": "jail-02 heartbeat",
            "from": "jail-02",
            "data": {"jail_id": "jail-02", "actor": "alice", "rank_id": "jun-nub"}
        });
        let env = handle_bus_value(&value, &state).await;
        assert!(env.is_some());
        let flows = state.recent_flows(5).await;
        assert_eq!(flows.len(), 1);
        assert_eq!(flows[0].action, "presence");
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

    #[test]
    fn rank_id_to_level_known_ladder() {
        assert_eq!(rank_id_to_level("jun-nub"), 0);
        assert_eq!(rank_id_to_level("marshal-orchestrator"), 15);
        assert_eq!(rank_id_to_level("lead-warrant"), 7);
    }

    #[test]
    fn rank_id_to_level_fallback() {
        assert_eq!(rank_id_to_level(""), 0);
        assert_eq!(rank_id_to_level("custom-rank"), 0);
    }

    #[test]
    fn presence_from_envelope_parses_worker() {
        let value = serde_json::json!({
            "kind": "presence",
            "from": "jail-02",
            "body": "jail-02 heartbeat",
            "data": {
                "actor": "alice",
                "ide": "cursor",
                "agent": "orchestrator",
                "jail_id": "jail-02",
                "rank_id": "lead-warrant",
                "rank_title": "Lead Warrant"
            }
        });
        let worker = presence_from_envelope(&value).expect("worker");
        assert_eq!(worker.jail_id, "jail-02");
        assert_eq!(worker.actor, "alice");
        assert_eq!(worker.ide, "cursor");
        assert_eq!(worker.rank, 7);
        assert_eq!(worker.status, WorkerStatus::Ready);
    }

    #[tokio::test]
    async fn presence_envelope_updates_map() {
        let state = AppState::new(test_config());
        let value = serde_json::json!({
            "kind": "presence",
            "from": "jail-02",
            "body": "heartbeat",
            "data": {"jail_id": "jail-02", "actor": "alice", "rank_id": "jun-nub"}
        });
        let env = handle_bus_value(&value, &state).await;
        assert!(env.is_some());
        let map = state.presence_map().await;
        assert!(map.contains_key("jail-02"));
    }
}
