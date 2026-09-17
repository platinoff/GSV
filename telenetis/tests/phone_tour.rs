//! Guided tour (T10.1): the emulator opens Telenetis and clicks through it.
//!
//! Run with `cargo test --test phone_tour -- --nocapture` and read the
//! `TOUR[…]` lines — every step prints what the phone would see. Hermetic:
//! in-process app + mock GSV on ephemeral loopback ports, never live.

use telenetis::config::Config;
use telenetis::emu::{transport_encode, EmuPhone};
use telenetis::state::{AppState, TicketRow};

const BOT_TOKEN: &str = "test";
const USER_A: &str = "{\"id\":279058397,\"first_name\":\"TourA\",\"language_code\":\"en\"}";

fn now_unix() -> i64 {
    chrono::Utc::now().timestamp()
}

fn tour(msg: &str) {
    eprintln!("TOUR {msg}");
}

fn phone_config(gsv_url: String) -> Config {
    Config {
        bot_token: BOT_TOKEN.to_string(),
        gsv_url,
        poolai_url: "http://127.0.0.1:9".to_string(),
        port: 9800,
        jail_id: "tour-jail".to_string(),
        godfather_channel_id: 0,
        webhook_url: None,
        webhook_secret: None,
        public_url: None,
        tunnel_enabled: false,
        ngrok_bin: None,
    }
}

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

async fn spawn_telenetis(gsv_url: String) -> (String, AppState) {
    let state = AppState::new(phone_config(gsv_url));
    let app = telenetis::ui::router(state.clone())
        .merge(telenetis::bot::webhook::router(state.clone()))
        .merge(telenetis::stream::ws::router(state.clone()))
        .merge(telenetis::stream::sse::router(state.clone()))
        .merge(telenetis::tracker::router(state.clone()));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind tour server");
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .await
            .unwrap();
    });
    (format!("http://127.0.0.1:{port}"), state)
}

fn row(id: &str, title: &str, status: &str) -> TicketRow {
    TicketRow {
        id: id.to_string(),
        title: title.to_string(),
        body: "tour".to_string(),
        status: status.to_string(),
        product: "gsv".to_string(),
        claimed_by: None,
        scenario: None,
    }
}

#[tokio::test]
async fn phone_tours_telenetis() {
    // STEP 1 — boot: mock GSV + full app on ephemeral ports.
    let gsv_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let gsv_port = gsv_listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        axum::serve(gsv_listener, mock_gsv_router().into_make_service())
            .await
            .unwrap();
    });
    let (base, state) = spawn_telenetis(format!("http://127.0.0.1:{gsv_port}")).await;
    let http = reqwest::Client::new();
    let health: serde_json::Value = http
        .get(format!("{base}/health"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    tour(&format!(
        "[boot] health={} version={}",
        health["status"], health["version"]
    ));

    let dir = std::env::temp_dir().join("emu_phone_tour");
    let _ = std::fs::remove_dir_all(&dir);
    let phone = EmuPhone::new("tour-a", dir.clone(), USER_A).unwrap();

    // STEP 2 — empty board snapshot (what a fresh Mini App boot fetches).
    let snap: serde_json::Value = http
        .get(format!("{base}/api/snapshot?lang=en"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    tour(&format!(
        "[snapshot-empty] tickets={} next_hint={} i18n={}",
        snap["tickets"].as_array().unwrap().len(),
        snap["next_hint"],
        snap["i18n"]["lang"]
    ));
    assert!(snap["next_hint"].is_null());

    // STEP 3 — two tickets land (via GSV sync in prod); hint points at open.
    state
        .set_tickets(vec![
            row("T-TOUR-1", "First", "open"),
            row("T-TOUR-2", "Second", "in_progress"),
        ])
        .await;
    let snap: serde_json::Value = http
        .get(format!("{base}/api/snapshot?lang=en"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    tour(&format!(
        "[snapshot-seeded] tickets={} next_hint={}/{}",
        snap["tickets"].as_array().unwrap().len(),
        snap["next_hint"]["kind"],
        snap["next_hint"]["ticket_id"]
    ));
    assert_eq!(snap["next_hint"]["ticket_id"], "T-TOUR-1");

    // STEP 4 — open every Mini App page like taps would.
    for page in ["/app", "/board", "/probe", "/tensor", "/flows", "/roles"] {
        let resp = http.get(format!("{base}{page}")).send().await.unwrap();
        let status = resp.status();
        let len = resp.bytes().await.unwrap().len();
        tour(&format!("[open {page}] status={status} bytes={len}"));
        assert_eq!(status, reqwest::StatusCode::OK);
    }

    // STEP 5 — claim with a fresh handshake, then finish it.
    let auth = now_unix();
    let init = transport_encode(&phone.init_data(BOT_TOKEN, auth));
    let claim: serde_json::Value = http
        .post(format!(
            "{base}/api/board/claim?initData={init}&authDate={auth}"
        ))
        .json(&serde_json::json!({"id": "T-TOUR-1"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    tour(&format!("[claim T-TOUR-1] ok={}", claim["ok"]));
    assert_eq!(claim["ok"], true);
    let done: serde_json::Value = http
        .post(format!(
            "{base}/api/board/done?initData={init}&authDate={auth}"
        ))
        .json(&serde_json::json!({"id": "T-TOUR-1"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    tour(&format!("[done T-TOUR-1] ok={}", done["ok"]));
    assert_eq!(done["ok"], true);

    // STEP 6 — stale handshake: the reopen path, not a silent death.
    let stale_auth = auth - 100_000;
    let stale_init = transport_encode(&phone.init_data(BOT_TOKEN, stale_auth));
    let resp = http
        .post(format!(
            "{base}/api/board/claim?initData={stale_init}&authDate={stale_auth}"
        ))
        .json(&serde_json::json!({"id": "T-TOUR-1"}))
        .send()
        .await
        .unwrap();
    let body: serde_json::Value = resp.json().await.unwrap();
    tour(&format!(
        "[stale] ok={} error={}",
        body["ok"], body["error"]
    ));
    assert_eq!(body["ok"], false);

    // STEP 7 — probe without a bound peer fails open (structured, no 500).
    let probe: serde_json::Value = http
        .post(format!(
            "{base}/api/edge/webgpu?initData={init}&authDate={auth}"
        ))
        .json(&serde_json::json!({
            "user": "279058397",
            "probe": {"supported": true, "adapter": {}, "limits": {}},
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    tour(&format!("[probe-unbound] ok={}", probe["ok"]));
    assert_eq!(probe["ok"], false);

    // STEP 8 — two phones meet on the tracker; the owner watches the swarm.
    use futures::{SinkExt, StreamExt};
    let ws_url = base.replacen("http://", "ws://", 1) + "/tracker";
    let info_bin = telenetis::tracker::bytes_to_latin1(&(0u8..20).collect::<Vec<u8>>());
    let (mut sock_b, _) = tokio_tungstenite::connect_async(&ws_url).await.unwrap();
    sock_b
        .send(tokio_tungstenite::tungstenite::Message::Text(
            serde_json::json!({
                "action": "announce", "info_hash": info_bin,
                "peer_id": "-WWTOUR000000000001", "numwant": 5, "offers": [],
            })
            .to_string()
            .into(),
        ))
        .await
        .unwrap();
    let (mut sock_a, _) = tokio_tungstenite::connect_async(&ws_url).await.unwrap();
    sock_a
        .send(tokio_tungstenite::tungstenite::Message::Text(
            serde_json::json!({
                "action": "announce", "info_hash": info_bin,
                "peer_id": "-WWTOUR000000000002", "numwant": 5, "offers": [],
            })
            .to_string()
            .into(),
        ))
        .await
        .unwrap();
    // Drain both stats replies: this synchronizes with the server (no read =
    // a race where /status may not see the second announce yet — the tour
    // caught exactly that).
    for sock in [&mut sock_a, &mut sock_b] {
        let msg = tokio::time::timeout(std::time::Duration::from_secs(5), sock.next())
            .await
            .expect("tracker stats timeout")
            .expect("socket closed")
            .unwrap();
        let text = msg.into_text().unwrap().to_string();
        let stats: serde_json::Value = serde_json::from_str(&text).unwrap();
        tour(&format!(
            "[tracker-stats] complete={} incomplete={}",
            stats["complete"], stats["incomplete"]
        ));
    }
    let status: serde_json::Value = http
        .get(format!("{base}/api/edge/tracker/status"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    tour(&format!(
        "[tracker] swarms={} peers={}",
        status["swarms"].as_array().unwrap().len(),
        status["swarms"][0]["peers"]
    ));
    assert_eq!(status["swarms"][0]["peers"], 2);

    tour("[done] tour green — phones go next, blind spots are documented");
    let _ = std::fs::remove_dir_all(dir);
}
