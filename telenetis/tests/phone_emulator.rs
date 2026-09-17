//! Phone emulator scenarios (T8.2): two emulated phones against a hermetic,
//! in-process Telenetis (real TCP on 127.0.0.1:ephemeral — never live
//! `:9800`/`:9999`). This is the gate before real phones in the Telegram
//! chat: everything here must be green, or the phones will tap blind.
//!
//! Covered: fresh claim→done, stale handshake, minimize/restore reconnect,
//! kill (disk lives, cache dies), two-phone P2P signaling through the real
//! `/tracker` socket, unbound probe failing open (never 500).

use telenetis::config::Config;
use telenetis::emu::{sign_init_data, transport_encode, EmuPhone, Lifecycle};
use telenetis::security::initdata::DEFAULT_MAX_AGE_SECS;
use telenetis::state::{AppState, TicketRow};

const BOT_TOKEN: &str = "test";
const USER_A: &str = "{\"id\":279058397,\"first_name\":\"EmuA\",\"language_code\":\"en\"}";
const USER_B: &str = "{\"id\":279058398,\"first_name\":\"EmuB\",\"language_code\":\"uk\"}";

fn now_unix() -> i64 {
    chrono::Utc::now().timestamp()
}

fn phone_config(gsv_url: String) -> Config {
    Config {
        bot_token: BOT_TOKEN.to_string(),
        gsv_url,
        poolai_url: "http://127.0.0.1:9".to_string(),
        port: 9800,
        jail_id: "emu-jail".to_string(),
        godfather_channel_id: 0,
        webhook_url: None,
        webhook_secret: None,
        public_url: None,
        tunnel_enabled: false,
        ngrok_bin: None,
    }
}

/// Mock GSV: claim/done always succeed (the emulator checks OUR wire, not GSV).
fn mock_gsv_router() -> axum::Router {
    axum::Router::new()
        .route(
            "/api/tickets/claim",
            axum::routing::post(|| async {
                axum::Json(serde_json::json!({"ok": true, "mock": "claim"}))
            }),
        )
        .route(
            "/api/tickets/done",
            axum::routing::post(|| async {
                axum::Json(serde_json::json!({"ok": true, "mock": "done"}))
            }),
        )
}

/// Spawn the FULL Telenetis app (mirror of main.rs routes, incl `/tracker`)
/// on an ephemeral loopback port. Returns `(http_base, state)`.
async fn spawn_telenetis(gsv_url: String) -> (String, AppState) {
    let state = AppState::new(phone_config(gsv_url));
    let app = telenetis::ui::router(state.clone())
        .merge(telenetis::bot::webhook::router(state.clone()))
        .merge(telenetis::stream::ws::router(state.clone()))
        .merge(telenetis::stream::sse::router(state.clone()))
        .merge(telenetis::tracker::router(state.clone()));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind emulator server");
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .await
            .unwrap();
    });
    (format!("http://127.0.0.1:{port}"), state)
}

async fn spawn_mock_gsv() -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mock gsv");
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        axum::serve(listener, mock_gsv_router().into_make_service())
            .await
            .unwrap();
    });
    format!("http://127.0.0.1:{port}")
}

fn emu_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("emu_phone_{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn seed_open_ticket() -> TicketRow {
    TicketRow {
        id: "T-EMU-1".to_string(),
        title: "Emulator task".to_string(),
        body: "prove the phone path".to_string(),
        status: "open".to_string(),
        product: "gsv".to_string(),
        claimed_by: None,
        scenario: None,
    }
}

#[tokio::test]
async fn fresh_phone_claims_and_finishes() {
    let gsv = spawn_mock_gsv().await;
    let (base, state) = spawn_telenetis(gsv).await;
    state.set_tickets(vec![seed_open_ticket()]).await;
    let http = reqwest::Client::new();
    let phone = EmuPhone::new("emu-a", emu_dir("claim"), USER_A).unwrap();

    // Snapshot hydrates like the Mini App boot.
    let snap: serde_json::Value = http
        .get(format!("{base}/api/snapshot?lang=en"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(snap["tickets"][0]["id"], "T-EMU-1");
    assert_eq!(snap["next_hint"]["kind"], "claim");

    // Fresh handshake → claim reaches the (mock) board.
    let auth = now_unix();
    let init = transport_encode(&phone.init_data(BOT_TOKEN, auth));
    let claim: serde_json::Value = http
        .post(format!(
            "{base}/api/board/claim?initData={init}&authDate={auth}"
        ))
        .json(&serde_json::json!({"id": "T-EMU-1"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(claim["ok"], true);
    let _ = std::fs::remove_dir_all(emu_dir("claim"));

    // …and done closes it.
    let done: serde_json::Value = http
        .post(format!(
            "{base}/api/board/done?initData={init}&authDate={auth}"
        ))
        .json(&serde_json::json!({"id": "T-EMU-1"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(done["ok"], true);
    let _ = std::fs::remove_dir_all(emu_dir("claim"));
}

#[tokio::test]
async fn stale_session_is_rejected_with_marker() {
    let gsv = spawn_mock_gsv().await;
    let (base, state) = spawn_telenetis(gsv).await;
    state.set_tickets(vec![seed_open_ticket()]).await;
    let http = reqwest::Client::new();
    let phone = EmuPhone::new("emu-a", emu_dir("stale"), USER_A).unwrap();

    // A handshake older than the backend window → 403 + stale marker
    // (the Mini App shows "reopen" instead of failing silently).
    let auth = now_unix() - DEFAULT_MAX_AGE_SECS as i64 - 60;
    let init = transport_encode(&phone.init_data(BOT_TOKEN, auth));
    let resp = http
        .post(format!(
            "{base}/api/board/claim?initData={init}&authDate={auth}"
        ))
        .json(&serde_json::json!({"id": "T-EMU-1"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), reqwest::StatusCode::FORBIDDEN);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["ok"], false);
    assert!(body["error"].as_str().unwrap().contains("stale"));
    let _ = std::fs::remove_dir_all(emu_dir("stale"));
}

#[tokio::test]
async fn minimize_restore_reconnects_and_rehydrates() {
    let gsv = spawn_mock_gsv().await;
    let (base, state) = spawn_telenetis(gsv).await;
    state.set_tickets(vec![seed_open_ticket()]).await;
    let http = reqwest::Client::new();
    let mut phone = EmuPhone::new("emu-a", emu_dir("bg"), USER_A).unwrap();
    assert!(phone.timers_live());

    // Live socket while foregrounded…
    let ws_url = base.replacen("http://", "ws://", 1) + "/ws";
    let (mut sock, _) = tokio_tungstenite::connect_async(&ws_url).await.unwrap();

    // …minimize: timers frozen, socket dropped like the WebView does…
    phone.minimize();
    assert!(!phone.timers_live());
    sock.close(None).await.unwrap();

    // …restore: reconnect works and the snapshot rehydrates the board.
    phone.restore();
    assert!(phone.timers_live());
    let (_sock2, _) = tokio_tungstenite::connect_async(&ws_url).await.unwrap();
    let snap: serde_json::Value = http
        .get(format!("{base}/api/snapshot?lang=en"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(snap["tickets"][0]["id"], "T-EMU-1");

    // Bytes stay servable for Range resume after return.
    let full = http
        .get(format!("{base}/static/app.js"))
        .send()
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap();
    assert!(!full.is_empty());
    let _ = std::fs::remove_dir_all(emu_dir("bg"));
}

#[tokio::test]
async fn killed_phone_keeps_disk_loses_cache() {
    // "Picked once — always there": the Download-equivalent (disk) survives
    // a killed Telegram; only the RAM/IDB-equivalent (cache) is lost.
    let dir = emu_dir("kill");
    let mut phone = EmuPhone::new("emu-a", dir.clone(), USER_A).unwrap();
    phone
        .storage
        .save_to_disk("qwen.gguf", b"gguf-fixture-bytes")
        .unwrap();
    phone
        .storage
        .cache_put("qwen.gguf", b"gguf-fixture-bytes".to_vec());
    phone.kill();
    assert_eq!(phone.lifecycle, Lifecycle::Killed);
    assert!(phone.storage.cache_get("qwen.gguf").is_none());

    // A fresh boot on the same device sees the model file on storage.
    let rebooted = EmuPhone::new("emu-a", dir.clone(), USER_A).unwrap();
    assert_eq!(
        rebooted.storage.read_disk("qwen.gguf").unwrap(),
        b"gguf-fixture-bytes"
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn two_phones_signal_through_tracker() {
    use futures::SinkExt;
    use telenetis::tracker::bytes_to_latin1;

    let gsv = spawn_mock_gsv().await;
    let (base, _state) = spawn_telenetis(gsv).await;
    let ws_url = base.replacen("http://", "ws://", 1) + "/tracker";

    let info_raw: Vec<u8> = (0u8..20).collect();
    let info_bin = bytes_to_latin1(&info_raw);
    let peer_a = "-WWEMU00000000000001";
    let peer_b = "-WWEMU00000000000002";
    let offer_id = bytes_to_latin1(&[0xabu8; 20]);

    // Phone B joins first (registers, no offers).
    let (mut b, _) = tokio_tungstenite::connect_async(&ws_url).await.unwrap();
    b.send(tokio_tungstenite::tungstenite::Message::Text(
        serde_json::json!({
            "action": "announce",
            "info_hash": info_bin,
            "peer_id": peer_b,
            "numwant": 5,
            "offers": [],
        })
        .to_string()
        .into(),
    ))
    .await
    .unwrap();
    let _stats_b = read_text(&mut b).await; // stats reply

    // Phone A announces with an offer → B must receive the relay.
    let (mut a, _) = tokio_tungstenite::connect_async(&ws_url).await.unwrap();
    a.send(tokio_tungstenite::tungstenite::Message::Text(
        serde_json::json!({
            "action": "announce",
            "info_hash": info_bin,
            "peer_id": peer_a,
            "numwant": 5,
            "offers": [{"offer": {"type": "offer", "sdp": "v=0 fake"}, "offer_id": offer_id}],
        })
        .to_string()
        .into(),
    ))
    .await
    .unwrap();
    let _stats_a = read_text(&mut a).await; // stats reply
    let relay = read_text(&mut b).await; // offer relay
    let relay: serde_json::Value = serde_json::from_str(&relay).unwrap();
    assert_eq!(relay["peer_id"], peer_a);
    assert_eq!(relay["offer"]["sdp"], "v=0 fake");

    // B answers → A must receive the answer relay (and B gets no reply).
    b.send(tokio_tungstenite::tungstenite::Message::Text(
        serde_json::json!({
            "action": "announce",
            "info_hash": info_bin,
            "peer_id": peer_b,
            "to_peer_id": peer_a,
            "answer": {"type": "answer", "sdp": "v=0 fake-answer"},
            "offer_id": offer_id,
        })
        .to_string()
        .into(),
    ))
    .await
    .unwrap();
    let answer = read_text(&mut a).await;
    let answer: serde_json::Value = serde_json::from_str(&answer).unwrap();
    assert_eq!(answer["peer_id"], peer_b);
    assert_eq!(answer["answer"]["sdp"], "v=0 fake-answer");
}

async fn read_text<S>(sock: &mut S) -> String
where
    S: futures::StreamExt<
            Item = Result<
                tokio_tungstenite::tungstenite::Message,
                tokio_tungstenite::tungstenite::Error,
            >,
        > + Unpin,
{
    let msg = tokio::time::timeout(std::time::Duration::from_secs(5), sock.next())
        .await
        .expect("tracker reply timeout")
        .expect("socket closed")
        .unwrap();
    msg.into_text().unwrap().to_string()
}

#[tokio::test]
async fn unbound_probe_fails_open_never_500() {
    let gsv = spawn_mock_gsv().await;
    let (base, _state) = spawn_telenetis(gsv).await;
    let http = reqwest::Client::new();
    let _phone = EmuPhone::new("emu-a", emu_dir("probe"), USER_B).unwrap();

    // No bound peer and dead upstreams: structured failure, never a 500.
    let auth = now_unix();
    let init = transport_encode(&sign_init_data(BOT_TOKEN, USER_B, auth));
    let resp = http
        .post(format!(
            "{base}/api/edge/webgpu?initData={init}&authDate={auth}"
        ))
        .json(&serde_json::json!({
            "user": "279058398",
            "probe": {"supported": true, "adapter": {}, "limits": {}},
        }))
        .send()
        .await
        .unwrap();
    assert_ne!(resp.status(), reqwest::StatusCode::INTERNAL_SERVER_ERROR);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["ok"], false);
    let _ = std::fs::remove_dir_all(emu_dir("probe"));
}
