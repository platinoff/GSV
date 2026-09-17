//! Minimal WebSocket tracker for phone-to-phone torrent P2P (T7).
//!
//! Browsers cannot do UDP DHT, so torrent bytes between two phones need a
//! signaling relay. This module speaks the WebTorrent WS-tracker announce
//! dialect (JSON text frames, `action: "announce"`). An announce carrying
//! `offers` registers the peer, gets swarm stats back, and relays one offer
//! per selected swarm mate (cap [`MAX_OFFERS_RELAYED`]). An announce carrying
//! `answer` plus `to_peer_id` is forwarded verbatim to that peer with no
//! reply. An announce with `event: "stopped"` drops the peer.
//!
//! Binary ids (`info_hash`/`offer_id`) ride JSON as latin1 binary strings
//! (one char per byte) — the helpers below convert to/from bytes losslessly
//! (serde_json alone would corrupt ≥0x80 chars via UTF-8).
//!
//! Sockets stay thin: every routing decision lives in [`SwarmRegistry`]
//! (pure, unit-tested); each connection owns an mpsc inbox the WS task
//! drains, and unregisters its peers on drop.

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{State, WebSocketUpgrade};
use axum::response::IntoResponse;
use futures::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::collections::HashMap;
use tokio::sync::mpsc;

/// Announce reply interval (seconds) served to WebTorrent clients.
pub const TRACKER_INTERVAL_SECS: u64 = 120;

/// Max offers relayed per announce (reference caps at 5–10; we take 5).
pub const MAX_OFFERS_RELAYED: usize = 5;

/// Encode bytes as a latin1 binary string (one char per byte) for JSON wire.
pub fn bytes_to_latin1(bytes: &[u8]) -> String {
    bytes.iter().map(|b| *b as char).collect()
}

/// Decode a latin1 binary string back to bytes. `None` when any char is
/// outside latin1 range (not round-trippable — reject the frame).
pub fn latin1_to_bytes(s: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(s.len());
    for c in s.chars() {
        let v = c as u32;
        if v > 0xFF {
            return None;
        }
        out.push(v as u8);
    }
    Some(out)
}

/// Lowercase hex of a latin1 binary string. `None` when not latin1.
pub fn latin1_hex(s: &str) -> Option<String> {
    latin1_to_bytes(s).map(|b| b.iter().map(|x| format!("{x:02x}")).collect())
}

/// Inbox of one connected peer (frames the socket task must deliver).
pub type PeerInbox = mpsc::UnboundedSender<String>;

/// One swarm: peer_id → inbox. Keyed by info-hash hex in the registry.
#[derive(Debug, Default)]
struct Swarm {
    peers: HashMap<String, PeerInbox>,
}

/// Pure routing state: swarms plus per-connection membership. No sockets —
/// the WS task owns inboxes and calls these methods under a lock.
#[derive(Debug, Default)]
pub struct SwarmRegistry {
    swarms: HashMap<String, Swarm>,
}

impl SwarmRegistry {
    pub fn new() -> Self {
        Self {
            swarms: HashMap::new(),
        }
    }

    /// Register (or refresh) a peer; returns `(complete, incomplete)` stats.
    /// `complete` = swarm size (we do not track seeding — honest count of
    /// signaling peers), `incomplete` = 0.
    pub fn announce(&mut self, info_hex: &str, peer_id: &str, inbox: PeerInbox) -> (usize, usize) {
        let swarm = self.swarms.entry(info_hex.to_string()).or_default();
        swarm.peers.insert(peer_id.to_string(), inbox);
        (swarm.peers.len(), 0)
    }

    /// Drop one peer; prunes empty swarms so the map cannot grow unbounded.
    pub fn drop_peer(&mut self, info_hex: &str, peer_id: &str) {
        if let Some(swarm) = self.swarms.get_mut(info_hex) {
            swarm.peers.remove(peer_id);
            if swarm.peers.is_empty() {
                self.swarms.remove(info_hex);
            }
        }
    }

    /// Drop every (info, peer) pair a dead connection owned.
    pub fn drop_owned(&mut self, owned: &[(String, String)]) {
        for (info_hex, peer_id) in owned {
            self.drop_peer(info_hex, peer_id);
        }
    }

    /// Swarm mates for offer relay, sorted for determinism, sender excluded.
    pub fn relay_targets(&self, info_hex: &str, exclude_peer: &str, n: usize) -> Vec<String> {
        let Some(swarm) = self.swarms.get(info_hex) else {
            return Vec::new();
        };
        let mut mates: Vec<String> = swarm
            .peers
            .keys()
            .filter(|p| p.as_str() != exclude_peer)
            .cloned()
            .collect();
        mates.sort();
        mates.truncate(n);
        mates
    }

    /// Inbox of a registered peer, if still connected.
    pub fn find_inbox(&self, info_hex: &str, peer_id: &str) -> Option<PeerInbox> {
        self.swarms.get(info_hex)?.peers.get(peer_id).cloned()
    }

    /// Swarm size (0 when unknown) — used by stats replies in tests.
    pub fn swarm_size(&self, info_hex: &str) -> usize {
        self.swarms
            .get(info_hex)
            .map(|s| s.peers.len())
            .unwrap_or(0)
    }

    /// Read-only snapshot for `GET /api/edge/tracker/status` (T9.1): one row
    /// per live swarm — info-hash hex plus peer count. Peer ids stay inside:
    /// they are internal names, never chat surfaces.
    pub fn status_snapshot(&self) -> Value {
        let mut swarms: Vec<Value> = self
            .swarms
            .iter()
            .map(|(info_hash, swarm)| json!({"info_hash": info_hash, "peers": swarm.peers.len()}))
            .collect();
        swarms.sort_by(|a, b| a["info_hash"].as_str().cmp(&b["info_hash"].as_str()));
        json!({"swarms": swarms})
    }
}

/// Frames the socket task must emit: replies to the sender over its own
/// socket, relays resolved through the registry (peer may be gone by then).
#[derive(Debug, Default, PartialEq, Eq)]
pub struct TrackerOut {
    pub to_sender: Vec<String>,
    pub relays: Vec<(String, String, String)>,
}

/// Validate a peer id (ASCII-ish, non-empty, bounded — reference uses 20).
fn valid_peer_id(v: Option<&Value>) -> Option<String> {
    let s = v?.as_str()?;
    let s = s.trim();
    if s.is_empty() || s.len() > 128 {
        return None;
    }
    Some(s.to_string())
}

/// Decode an info-hash binary string to lowercase hex (exactly 20 bytes).
fn info_hex(v: Option<&Value>) -> Option<String> {
    let s = v?.as_str()?;
    let bytes = latin1_to_bytes(s)?;
    if bytes.len() != 20 {
        return None;
    }
    Some(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

/// Route one parsed client frame. Pure — the socket task delivers `out`.
/// `owned` accumulates this connection's (info, peer) pairs for drop.
/// `self_inbox` registers the sender for future relays.
pub fn handle_frame(
    reg: &mut SwarmRegistry,
    owned: &mut Vec<(String, String)>,
    raw: &str,
    self_inbox: &PeerInbox,
) -> TrackerOut {
    let mut out = TrackerOut::default();
    let Ok(body) = serde_json::from_str::<Value>(raw) else {
        return out;
    };
    if body.get("action").and_then(Value::as_str) != Some("announce") {
        return out;
    }
    let Some(info_hex) = info_hex(body.get("info_hash")) else {
        return out;
    };
    let Some(peer_id) = valid_peer_id(body.get("peer_id")) else {
        return out;
    };
    let info_verbatim = body
        .get("info_hash")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();

    // Answer path: forward verbatim to the offering peer, no reply.
    if let (Some(answer), Some(to_peer)) = (
        body.get("answer"),
        body.get("to_peer_id").and_then(Value::as_str),
    ) {
        if answer.is_object() && !to_peer.trim().is_empty() {
            out.relays
                .push((info_hex, to_peer.trim().to_string(), raw.to_string()));
        }
        return out;
    }

    // Stopped: drop, no reply.
    if body.get("event").and_then(Value::as_str) == Some("stopped") {
        reg.drop_peer(&info_hex, &peer_id);
        owned.retain(|(i, p)| i != &info_hex || p != &peer_id);
        return out;
    }

    // (Re-)announce: register + stats reply.
    let (complete, incomplete) = reg.announce(&info_hex, &peer_id, self_inbox.clone());
    let owned_pair = (info_hex.clone(), peer_id.clone());
    if !owned.contains(&owned_pair) {
        owned.push(owned_pair);
    }
    out.to_sender.push(
        json!({
            "action": "announce",
            "info_hash": info_verbatim,
            "interval": TRACKER_INTERVAL_SECS,
            "complete": complete,
            "incomplete": incomplete,
        })
        .to_string(),
    );

    // Offer relay: pair i-th valid offer with i-th mate (cap 5).
    let offers: Vec<&Value> = body
        .get("offers")
        .and_then(Value::as_array)
        .map(|a| a.iter().collect())
        .unwrap_or_default();
    if offers.is_empty() {
        return out;
    }
    let mates = reg.relay_targets(&info_hex, &peer_id, MAX_OFFERS_RELAYED);
    for (offer, mate) in offers.into_iter().take(MAX_OFFERS_RELAYED).zip(mates) {
        let (Some(offer_obj), Some(offer_id)) = (
            offer.get("offer"),
            offer.get("offer_id").and_then(Value::as_str),
        ) else {
            continue;
        };
        if !offer_obj.is_object() || latin1_to_bytes(offer_id).is_none() {
            continue;
        }
        out.relays.push((
            info_hex.clone(),
            mate,
            json!({
                "action": "announce",
                "info_hash": info_verbatim,
                "peer_id": peer_id,
                "offer": offer_obj,
                "offer_id": offer_id,
            })
            .to_string(),
        ));
    }
    out
}

pub fn router(state: crate::state::AppState) -> axum::Router {
    axum::Router::new()
        .route("/tracker", axum::routing::get(tracker_handler))
        .route(
            "/api/edge/tracker/status",
            axum::routing::get(tracker_status),
        )
        .with_state(state)
}

/// Read-only swarm overview (T9.1): lets the owner watch a two-phone P2P
/// session form without touching the signaling flow. No auth like
/// `/api/status` — counts only, no peer ids.
async fn tracker_status(State(state): State<crate::state::AppState>) -> axum::Json<Value> {
    let snapshot = state.tracker_state().read().await.status_snapshot();
    axum::Json(snapshot)
}

async fn tracker_handler(
    ws: WebSocketUpgrade,
    State(state): State<crate::state::AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

/// Serve one tracker socket. Own replies and relay deliveries both flow
/// through the connection inbox; on exit the connection's peers are dropped.
async fn handle_socket(socket: WebSocket, state: crate::state::AppState) {
    let (mut sender, mut receiver) = socket.split();
    let (inbox_tx, mut inbox_rx) = mpsc::unbounded_channel::<String>();

    let mut send_task = tokio::spawn(async move {
        while let Some(frame) = inbox_rx.recv().await {
            if sender.send(Message::Text(frame.into())).await.is_err() {
                break;
            }
        }
    });

    let mut owned: Vec<(String, String)> = Vec::new();
    let recv_state = state.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            let text = match msg {
                Ok(Message::Text(t)) => t.to_string(),
                Ok(Message::Close(_)) | Err(_) => break,
                _ => continue,
            };
            let out = {
                let mut reg = recv_state.tracker_state().write().await;
                handle_frame(&mut reg, &mut owned, &text, &inbox_tx)
            };
            for frame in out.to_sender {
                if inbox_tx.send(frame).is_err() {
                    break;
                }
            }
            for (info_hex, peer_id, frame) in out.relays {
                let inbox = {
                    recv_state
                        .tracker_state()
                        .read()
                        .await
                        .find_inbox(&info_hex, &peer_id)
                };
                if let Some(tx) = inbox {
                    let _ = tx.send(frame);
                }
            }
        }
        recv_state.tracker_state().write().await.drop_owned(&owned);
    });

    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const HASH_HEX: &str = "0123456789012345678901234567890123456789";

    fn latin1_of_hex(hex: &str) -> String {
        let bytes: Vec<u8> = (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
            .collect();
        bytes_to_latin1(&bytes)
    }

    fn inbox() -> (PeerInbox, mpsc::UnboundedReceiver<String>) {
        mpsc::unbounded_channel()
    }

    fn announce_frame(peer: &str, offers: Value) -> String {
        json!({
            "action": "announce",
            "info_hash": latin1_of_hex(HASH_HEX),
            "peer_id": peer,
            "uploaded": 0,
            "downloaded": 0,
            "left": 100,
            "numwant": 5,
            "offers": offers,
        })
        .to_string()
    }

    fn offer_obj(n: u8) -> Value {
        json!({
            "offer": {"type": "offer", "sdp": format!("v=0 offer{n}")},
            "offer_id": latin1_of_hex(&format!("{:040x}", n as u64 + 0xaa)),
        })
    }

    #[test]
    fn latin1_round_trips_high_bytes() {
        let bytes: Vec<u8> = (0u8..=255u8).collect();
        let s = bytes_to_latin1(&bytes);
        assert_eq!(latin1_to_bytes(&s).unwrap(), bytes);
        assert_eq!(s.chars().count(), 256);
    }

    #[test]
    fn latin1_rejects_non_latin1_chars() {
        assert!(latin1_to_bytes("ok").is_some());
        assert!(latin1_to_bytes("ok€").is_none());
    }

    #[test]
    fn announce_registers_and_replies_stats() {
        let mut reg = SwarmRegistry::new();
        let (tx, _rx) = inbox();
        let mut owned = Vec::new();
        let out = handle_frame(
            &mut reg,
            &mut owned,
            &announce_frame("-WW0001-aaaaaaaaaa", json!([])),
            &tx,
        );
        assert_eq!(out.relays.len(), 0);
        assert_eq!(out.to_sender.len(), 1);
        let reply: Value = serde_json::from_str(&out.to_sender[0]).unwrap();
        assert_eq!(reply["action"], "announce");
        assert_eq!(reply["complete"], 1);
        assert_eq!(reply["incomplete"], 0);
        assert_eq!(reply["interval"], TRACKER_INTERVAL_SECS);
        assert_eq!(owned.len(), 1);
        assert_eq!(reg.swarm_size(HASH_HEX), 1);
    }

    #[test]
    fn offer_relays_to_swarm_mate_not_sender() {
        let mut reg = SwarmRegistry::new();
        let (tx_a, _rx_a) = inbox();
        let (tx_b, _rx_b) = inbox();
        let mut owned = Vec::new();
        handle_frame(
            &mut reg,
            &mut owned,
            &announce_frame("-WW0001-aaaaaaaaaa", json!([])),
            &tx_a,
        );
        let out = handle_frame(
            &mut reg,
            &mut owned,
            &announce_frame("-WW0002-bbbbbbbbbb", json!([offer_obj(1)])),
            &tx_b,
        );
        assert_eq!(out.to_sender.len(), 1);
        assert_eq!(out.relays.len(), 1);
        let (info, mate, frame) = &out.relays[0];
        assert_eq!(info, HASH_HEX);
        assert_eq!(mate, "-WW0001-aaaaaaaaaa");
        let relay: Value = serde_json::from_str(frame).unwrap();
        assert_eq!(relay["peer_id"], "-WW0002-bbbbbbbbbb");
        assert_eq!(relay["offer"]["sdp"], "v=0 offer1");
        assert!(relay.get("uploaded").is_none());
    }

    #[test]
    fn offer_cap_is_five_per_announce() {
        let mut reg = SwarmRegistry::new();
        let mut owned = Vec::new();
        // Six mates waiting.
        for i in 0..6 {
            let (tx, _rx) = inbox();
            handle_frame(
                &mut reg,
                &mut owned,
                &announce_frame(&format!("-WW000{i}-aaaaaaaaaa"), json!([])),
                &tx,
            );
        }
        let (tx, _rx) = inbox();
        let offers: Vec<Value> = (0..7).map(offer_obj).collect();
        let out = handle_frame(
            &mut reg,
            &mut owned,
            &announce_frame("-WW0009-zzzzzzzzzz", json!(offers)),
            &tx,
        );
        assert_eq!(out.relays.len(), MAX_OFFERS_RELAYED);
    }

    #[test]
    fn answer_forwards_verbatim_without_sender_reply() {
        let mut reg = SwarmRegistry::new();
        let (tx_a, _rx_a) = inbox();
        let (tx_b, _rx_b) = inbox();
        let mut owned = Vec::new();
        handle_frame(
            &mut reg,
            &mut owned,
            &announce_frame("-WW0001-aaaaaaaaaa", json!([])),
            &tx_a,
        );
        handle_frame(
            &mut reg,
            &mut owned,
            &announce_frame("-WW0002-bbbbbbbbbb", json!([])),
            &tx_b,
        );
        let answer = json!({
            "action": "announce",
            "info_hash": latin1_of_hex(HASH_HEX),
            "peer_id": "-WW0001-aaaaaaaaaa",
            "to_peer_id": "-WW0002-bbbbbbbbbb",
            "answer": {"type": "answer", "sdp": "v=0 answer"},
            "offer_id": latin1_of_hex(&format!("{:040x}", 0xabu64)),
        })
        .to_string();
        let out = handle_frame(&mut reg, &mut owned, &answer, &tx_a);
        assert!(out.to_sender.is_empty());
        assert_eq!(out.relays.len(), 1);
        assert_eq!(out.relays[0].1, "-WW0002-bbbbbbbbbb");
        // Verbatim: the exact frame the answerer sent goes out.
        assert_eq!(out.relays[0].2, answer);
    }

    #[test]
    fn stopped_drops_peer_and_prunes_swarm() {
        let mut reg = SwarmRegistry::new();
        let (tx, _rx) = inbox();
        let mut owned = Vec::new();
        handle_frame(
            &mut reg,
            &mut owned,
            &announce_frame("-WW0001-aaaaaaaaaa", json!([])),
            &tx,
        );
        assert_eq!(reg.swarm_size(HASH_HEX), 1);
        let stop = json!({
            "action": "announce",
            "info_hash": latin1_of_hex(HASH_HEX),
            "peer_id": "-WW0001-aaaaaaaaaa",
            "event": "stopped",
            "numwant": 0,
        })
        .to_string();
        let out = handle_frame(&mut reg, &mut owned, &stop, &tx);
        assert!(out.to_sender.is_empty() && out.relays.is_empty());
        assert_eq!(reg.swarm_size(HASH_HEX), 0);
        assert!(owned.is_empty());
    }

    #[test]
    fn malformed_frames_are_ignored() {
        let mut reg = SwarmRegistry::new();
        let (tx, _rx) = inbox();
        let mut owned = Vec::new();
        // Not JSON.
        assert!(handle_frame(&mut reg, &mut owned, "nope", &tx)
            .to_sender
            .is_empty());
        // Wrong action.
        let out = handle_frame(&mut reg, &mut owned, r#"{"action":"scrape"}"#, &tx);
        assert!(out.to_sender.is_empty() && out.relays.is_empty());
        // Short hash.
        let out = handle_frame(
            &mut reg,
            &mut owned,
            r#"{"action":"announce","info_hash":"abc","peer_id":"-WW0001-aaaaaaaaaa"}"#,
            &tx,
        );
        assert!(out.to_sender.is_empty() && out.relays.is_empty());
        // Missing peer.
        let out = handle_frame(
            &mut reg,
            &mut owned,
            &format!(
                "{{\"action\":\"announce\",\"info_hash\":{}}}",
                json!(latin1_of_hex(HASH_HEX))
            ),
            &tx,
        );
        assert!(out.to_sender.is_empty() && out.relays.is_empty());
        assert!(owned.is_empty());
    }

    #[test]
    fn relay_delivery_reaches_mate_inbox() {
        // End-to-end of the pure layer: relay target resolves to a live inbox.
        let mut reg = SwarmRegistry::new();
        let (tx_a, mut rx_a) = inbox();
        let (tx_b, _rx_b) = inbox();
        let mut owned = Vec::new();
        handle_frame(
            &mut reg,
            &mut owned,
            &announce_frame("-WW0001-aaaaaaaaaa", json!([])),
            &tx_a,
        );
        let out = handle_frame(
            &mut reg,
            &mut owned,
            &announce_frame("-WW0002-bbbbbbbbbb", json!([offer_obj(3)])),
            &tx_b,
        );
        assert_eq!(out.relays.len(), 1);
        let (info, mate, frame) = &out.relays[0];
        reg.find_inbox(info, mate)
            .unwrap()
            .send(frame.clone())
            .unwrap();
        let got = rx_a.try_recv().unwrap();
        let relay: Value = serde_json::from_str(&got).unwrap();
        assert_eq!(relay["offer"]["sdp"], "v=0 offer3");
    }

    #[test]
    fn reannounce_refreshes_without_dup_owned() {
        let mut reg = SwarmRegistry::new();
        let (tx, _rx) = inbox();
        let mut owned = Vec::new();
        for _ in 0..3 {
            handle_frame(
                &mut reg,
                &mut owned,
                &announce_frame("-WW0001-aaaaaaaaaa", json!([])),
                &tx,
            );
        }
        assert_eq!(owned.len(), 1);
        assert_eq!(reg.swarm_size(HASH_HEX), 1);
    }

    #[test]
    fn status_snapshot_lists_swarms_without_peer_ids() {
        // T9.1: the status surface counts, never identities.
        let mut reg = SwarmRegistry::new();
        let (tx_a, _rx_a) = inbox();
        let (tx_b, _rx_b) = inbox();
        let mut owned = Vec::new();
        handle_frame(
            &mut reg,
            &mut owned,
            &announce_frame("-WW0001-aaaaaaaaaa", json!([])),
            &tx_a,
        );
        handle_frame(
            &mut reg,
            &mut owned,
            &announce_frame("-WW0002-bbbbbbbbbb", json!([])),
            &tx_b,
        );
        let snap = reg.status_snapshot();
        assert_eq!(snap["swarms"].as_array().unwrap().len(), 1);
        assert_eq!(snap["swarms"][0]["info_hash"], HASH_HEX);
        assert_eq!(snap["swarms"][0]["peers"], 2);
        let flat = snap.to_string();
        assert!(!flat.contains("-WW0001-aaaaaaaaaa"), "{flat}");
        assert!(!flat.contains("-WW0002-bbbbbbbbbb"), "{flat}");
    }

    #[tokio::test]
    async fn tracker_status_route_serves_empty_swarms() {
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use tower::ServiceExt;

        let state = crate::state::AppState::new(crate::config::Config {
            bot_token: "test".to_string(),
            gsv_url: "http://127.0.0.1:9999".to_string(),
            poolai_url: "http://127.0.0.1:8091".to_string(),
            port: 9800,
            jail_id: "test-jail".to_string(),
            godfather_channel_id: 0,
            webhook_url: None,
            webhook_secret: None,
            public_url: None,
            tunnel_enabled: false,
            ngrok_bin: None,
        });
        let resp = super::router(state)
            .oneshot(
                Request::builder()
                    .uri("/api/edge/tracker/status")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["swarms"].as_array().unwrap().len(), 0);
    }
}
