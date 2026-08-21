//! Godfather Telegram bind + MCP bus contracts (bands 167 · 169 · 174).
//!
//! Dry-run / `X-Telegram-Dry-Run: 1` never opens sockets. Missing channel or
//! token → `{ok:false}` without secrets. Band 169: in-memory bus queue, no
//! webhook, no Cloudflare. Band 174: `/ticket` ingest → board row; solo MCP
//! auto-claims. `gsv_telegram_create_ticket` stays unused (name is `gsv_telegram_ticket`).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use gsv::boxes::settings;
use gsv::boxes::telegram;
use gsv::boxes::tickets::{self, ClaimedBy};
use gsv::boxes::ui::{render_card, CARD_NAMES};
use gsv::mcp;
use gsv::server::router;
use gsv::AppState;
use serde_json::{json, Value};
use tokio::sync::broadcast;
use tower::ServiceExt;

fn kit_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn temp_data(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("gsv-telegram-http-{tag}-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn app_with_data(data: PathBuf) -> axum::Router {
    let (tx, _rx) = broadcast::channel(32);
    let state = AppState::new(Some(kit_root()), Some(data), tx);
    router(state)
}

async fn get_json(app: &axum::Router, path: &str) -> (StatusCode, Value) {
    get_json_headers(app, path, &[]).await
}

async fn get_json_headers(
    app: &axum::Router,
    path: &str,
    extra: &[(&str, &str)],
) -> (StatusCode, Value) {
    let mut builder = Request::builder().uri(path).method(Method::GET);
    for (k, v) in extra {
        builder = builder.header(*k, *v);
    }
    let res = app
        .clone()
        .oneshot(builder.body(Body::empty()).expect("request"))
        .await
        .expect("response");
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .expect("body");
    let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, json)
}

fn save_godfather(data: &Path, channel: &str, token: &str) {
    settings::save(
        data,
        &settings::SettingsFile {
            godfather: settings::Godfather {
                channel_id: channel.into(),
                allowed_user_ids: vec![],
                bot_token: token.into(),
                poll: false,
                role: String::new(),
            },
            ..Default::default()
        },
    )
    .expect("save");
}

fn assert_no_secret(json: &Value, secret: &str) {
    assert!(!settings::json_has_bot_token(json), "{json}");
    let raw = serde_json::to_string(json).expect("json");
    assert!(!raw.contains(secret), "{raw}");
    assert!(!raw.contains("bot_token"), "{raw}");
}

#[tokio::test]
async fn missing_channel_or_token_is_ok_false_without_secrets() {
    let _g = bus_guard().await;
    telegram::bus_reset();
    let data = temp_data("missing");
    let _ = std::fs::remove_file(data.join("gsv_settings.json"));
    let w = telegram::status(&data, true).await;
    assert_eq!(w["ok"], false, "{w}");
    assert_eq!(w["token_set"], false, "{w}");
    assert_eq!(w["polling"], false, "{w}");
    assert!(!w["error"].as_str().unwrap_or_default().is_empty());
    assert_no_secret(&w, "123:secret-token");

    save_godfather(&data, "", "123:secret-token");
    let no_ch = telegram::status(&data, true).await;
    assert_eq!(no_ch["ok"], false);
    assert_eq!(no_ch["token_set"], true);
    assert_no_secret(&no_ch, "123:secret-token");

    let data2 = temp_data("no-token");
    save_godfather(&data2, "-1001", "");
    let no_tok = telegram::status(&data2, true).await;
    assert_eq!(no_tok["ok"], false);
    assert_eq!(no_tok["token_set"], false);
    assert_no_secret(&no_tok, "123:secret-token");
}

#[tokio::test]
async fn dry_run_stub_returns_fake_bot_and_chat() {
    let data = temp_data("stub");
    save_godfather(&data, "-100123", "123:secret-token");
    let w = telegram::status(&data, true).await;
    assert_eq!(w["ok"], true, "{w}");
    assert_eq!(w["dry_run"], true);
    assert_eq!(w["channel_id"], "-100123");
    assert_eq!(w["token_set"], true);
    assert_eq!(w["polling"], false);
    assert_eq!(w["bot_username"], "gsv_godfather_bot");
    assert_eq!(w["chat_title"], "GSV Godfather (dry-run)");
    assert_eq!(w["member_count"], 3);
    assert_eq!(w["chat_kind"], "channel");
    assert_eq!(w["chat_role"], "host");
    assert_eq!(
        settings::load_result(&data)
            .expect("settings")
            .tickets
            .member_count,
        0,
        "dry-run must not persist stub member_count"
    );
    assert!(
        w["last_probe"].as_str().unwrap_or_default().len() > 8,
        "{w}"
    );
    assert_no_secret(&w, "123:secret-token");
}

#[tokio::test]
async fn get_telegram_dry_run_header_forces_stub() {
    let data = temp_data("http-dry");
    save_godfather(&data, "-1009", "123:http-secret");
    let app = app_with_data(data);
    let (status, json) =
        get_json_headers(&app, "/api/telegram", &[("x-telegram-dry-run", "1")]).await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert_eq!(json["ok"], true, "{json}");
    assert_eq!(json["dry_run"], true);
    assert_eq!(json["bot_username"], "gsv_godfather_bot");
    assert_eq!(json["channel_id"], "-1009");
    assert_no_secret(&json, "123:http-secret");
}

#[tokio::test]
async fn get_telegram_missing_is_ok_false() {
    let app = app_with_data(temp_data("http-empty"));
    let (status, json) = get_json(&app, "/api/telegram").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["ok"], false);
    assert_eq!(json["polling"], false);
    assert_no_secret(&json, "123:secret-token");
}

#[test]
fn poller_default_off_relay_or_flag_opts_in() {
    let mut file = settings::SettingsFile::default();
    assert!(!telegram::poller_wanted(&file));
    assert!(!telegram::boot_should_probe());
    file.godfather.poll = true;
    assert!(telegram::poller_wanted(&file));
    file.godfather.poll = false;
    file.workflows.enabled = vec!["telegram-relay".into()];
    assert!(telegram::poller_wanted(&file));
}

#[test]
fn error_mapper_strips_token() {
    let token = "999:leaked-bot-token";
    let err = telegram::map_probe_error("upstream 401 at bot999:leaked-bot-token/getMe", token);
    let raw = serde_json::to_string(&err).expect("json");
    assert_eq!(err["ok"], false);
    assert!(!raw.contains(token), "{raw}");
    assert!(!raw.contains("bot_token"), "{raw}");
}

#[test]
fn card_telegram_in_registry() {
    assert!(CARD_NAMES.contains(&"telegram"));
    assert_eq!(CARD_NAMES.len(), 42);
    let empty = render_card(
        "telegram",
        &json!({
            "ok": true,
            "token_set": false,
            "channel_id": "",
            "polling": false,
            "dry_run": true
        }),
    )
    .expect("empty");
    assert!(empty.contains("telegram — no data"), "{empty}");
    let err = render_card("telegram", &json!({ "ok": false, "error": "down" })).expect("err");
    assert!(err.contains("<span class='err'>down</span>"), "{err}");
}

#[tokio::test]
async fn mcp_telegram_is_read_only_status() {
    let data = temp_data("mcp");
    save_godfather(&data, "-100mcp", "mcp-secret-token");
    let app = app_with_data(data);
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/mcp")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .header("x-telegram-dry-run", "1")
                .body(Body::from(
                    json!({
                        "jsonrpc": "2.0",
                        "id": 1,
                        "method": "tools/call",
                        "params": { "name": "gsv_telegram", "arguments": {} }
                    })
                    .to_string(),
                ))
                .expect("req"),
        )
        .await
        .expect("resp");
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .expect("body");
    let json: Value = serde_json::from_slice(&bytes).expect("json");
    assert_eq!(json["result"]["isError"], false, "{json}");
    let text = json["result"]["content"][0]["text"].as_str().unwrap_or("");
    assert!(text.contains("dry_run"), "{text}");
    assert!(text.contains("gsv_godfather_bot"), "{text}");
    assert!(!text.contains("bot_token"), "{text}");
    assert!(!text.contains("mcp-secret-token"), "{text}");
    assert!(mcp::tool_names().contains(&"gsv_telegram"));
    assert!(mcp::tool_names().contains(&"gsv_telegram_bus_send"));
    assert!(mcp::tool_names().contains(&"gsv_telegram_bus_poll"));
    assert!(mcp::tool_names().contains(&"gsv_telegram_ticket"));
    assert!(mcp::tool_names().contains(&"gsv_telegram_poll"));
    assert!(mcp::tool_names().contains(&"gsv_telegram_decode"));
    assert!(!mcp::tool_names().contains(&"gsv_telegram_create_ticket"));
    assert_eq!(mcp::tool_names().len(), 56);
}

async fn bus_guard() -> tokio::sync::MutexGuard<'static, ()> {
    static LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
    LOCK.lock().await
}

fn save_relay(data: &Path, channel: &str, token: &str, allowed: &[&str]) {
    settings::save(
        data,
        &settings::SettingsFile {
            godfather: settings::Godfather {
                channel_id: channel.into(),
                allowed_user_ids: allowed.iter().map(|s| (*s).to_string()).collect(),
                bot_token: token.into(),
                poll: false,
                role: String::new(),
            },
            workflows: settings::Workflows {
                enabled: vec!["telegram-relay".into()],
            },
            ..Default::default()
        },
    )
    .expect("save relay");
}

async fn post_json(
    app: &axum::Router,
    path: &str,
    body: Value,
    origin: Option<&str>,
    site: Option<&str>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .uri(path)
        .method(Method::POST)
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(o) = origin {
        builder = builder.header(header::ORIGIN, o);
    }
    if let Some(s) = site {
        builder = builder.header("sec-fetch-site", s);
    }
    let res = app
        .clone()
        .oneshot(builder.body(Body::from(body.to_string())).expect("request"))
        .await
        .expect("response");
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .expect("body");
    let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, json)
}

#[test]
fn parse_envelope_rejects_invalid_and_non_bus() {
    let bad = telegram::parse_envelope(&json!({ "nope": true }));
    assert!(bad.is_err(), "{bad:?}");
    let kind = telegram::parse_envelope(&json!({
        "v": 1,
        "kind": "ticket",
        "from": "cursor",
        "body": "hi"
    }));
    assert!(kind.is_err(), "{kind:?}");
    let sync = telegram::parse_envelope(&json!({
        "v": 1,
        "kind": "sync",
        "from": "solo",
        "ticket_id": "t-1",
        "body": "claimed t-1"
    }))
    .expect("sync");
    assert_eq!(sync.kind, "sync");
    let ok = telegram::parse_envelope(&json!({
        "v": 1,
        "kind": "bus",
        "from": "cursor",
        "to": "opencode",
        "ticket_id": "t-1",
        "body": "ping"
    }))
    .expect("ok");
    assert_eq!(ok.v, 1);
    assert_eq!(ok.kind, "bus");
    assert_eq!(ok.from, "cursor");
    assert_eq!(ok.to.as_deref(), Some("opencode"));
    assert_eq!(ok.ticket_id.as_deref(), Some("t-1"));
    assert_eq!(ok.body, "ping");
    let presence = telegram::parse_envelope(&json!({
        "v": 1,
        "kind": "presence",
        "from": "alice-gsv",
        "body": "alice-gsv heartbeat",
        "data": { "actor": "alice", "ide": "cursor", "hint": "heartbeat" }
    }))
    .expect("presence");
    assert_eq!(presence.kind, "presence");
    assert_eq!(
        telegram::classify_inbound(&serde_json::to_string(&presence).unwrap()),
        "presence"
    );
    let claim = telegram::parse_envelope(&json!({
        "v": 1,
        "kind": "claim",
        "from": "alice-gsv",
        "ticket_id": "t-1",
        "body": "alice-gsv claims t-1",
        "data": { "actor": "alice", "hint": "federated-claim" }
    }))
    .expect("claim");
    assert_eq!(claim.kind, "claim");
    assert_eq!(claim.ticket_id.as_deref(), Some("t-1"));
    assert_eq!(
        telegram::classify_inbound(&serde_json::to_string(&claim).unwrap()),
        "claim"
    );
    let claim_no_id = telegram::parse_envelope(&json!({
        "v": 1,
        "kind": "claim",
        "from": "alice-gsv",
        "body": "no id"
    }));
    assert!(claim_no_id.is_err(), "{claim_no_id:?}");
}

#[tokio::test]
async fn dry_run_queue_round_trips_two_messages() {
    let _g = bus_guard().await;
    telegram::bus_reset();
    let data = temp_data("bus-round");
    save_relay(&data, "-100bus", "123:bus-secret", &[]);
    let a = telegram::bus_send(
        &data,
        true,
        &json!({ "from": "cursor", "to": "opencode", "body": "one" }),
    )
    .await;
    assert_eq!(a["ok"], true, "{a}");
    assert_no_secret(&a, "123:bus-secret");
    telegram::bus_clear_rate_limit();
    let b = telegram::bus_send(
        &data,
        true,
        &json!({ "from": "opencode", "ticket_id": "t-9", "body": "two" }),
    )
    .await;
    assert_eq!(b["ok"], true, "{b}");
    let polled = telegram::bus_poll(&data, true, Some(8)).await;
    assert_eq!(polled["ok"], true, "{polled}");
    let msgs = polled["messages"].as_array().expect("messages");
    assert_eq!(msgs.len(), 2, "{polled}");
    assert_eq!(msgs[0]["body"], "one");
    assert_eq!(msgs[0]["from"], "cursor");
    assert_eq!(msgs[0]["kind"], "bus");
    assert_eq!(msgs[1]["body"], "two");
    assert_eq!(msgs[1]["ticket_id"], "t-9");
    assert_no_secret(&polled, "123:bus-secret");
    assert!(!serde_json::to_string(&polled)
        .unwrap()
        .contains("bot_token"));
}

#[tokio::test]
async fn missing_telegram_relay_is_error() {
    let _g = bus_guard().await;
    telegram::bus_reset();
    let data = temp_data("bus-off");
    save_godfather(&data, "-100off", "123:off-secret");
    let send = telegram::bus_send(&data, true, &json!({ "from": "cursor", "body": "x" })).await;
    assert_eq!(send["ok"], false, "{send}");
    assert!(
        send["error"]
            .as_str()
            .unwrap_or("")
            .contains("telegram-relay"),
        "{send}"
    );
    assert_no_secret(&send, "123:off-secret");
    let poll = telegram::bus_poll(&data, true, None).await;
    assert_eq!(poll["ok"], false, "{poll}");
    assert!(
        poll["error"]
            .as_str()
            .unwrap_or("")
            .contains("telegram-relay"),
        "{poll}"
    );
}

#[tokio::test]
async fn allowlist_rejects_unknown_from() {
    let _g = bus_guard().await;
    telegram::bus_reset();
    let data = temp_data("bus-allow");
    save_relay(&data, "-100a", "123:allow-secret", &["42"]);
    let bad = telegram::bus_send(&data, true, &json!({ "from": "99", "body": "nope" })).await;
    assert_eq!(bad["ok"], false, "{bad}");
    assert_no_secret(&bad, "123:allow-secret");
    let ok = telegram::bus_send(&data, true, &json!({ "from": "42", "body": "yes" })).await;
    assert_eq!(ok["ok"], true, "{ok}");
}

#[tokio::test]
async fn body_over_cap_is_error() {
    let _g = bus_guard().await;
    telegram::bus_reset();
    let data = temp_data("bus-cap");
    save_relay(&data, "-100c", "123:cap-secret", &[]);
    let body = "x".repeat(telegram::BODY_CAP + 1);
    let send = telegram::bus_send(&data, true, &json!({ "from": "cursor", "body": body })).await;
    assert_eq!(send["ok"], false, "{send}");
    assert!(
        send["error"].as_str().unwrap_or("").contains("body"),
        "{send}"
    );
    assert_no_secret(&send, "123:cap-secret");
}

#[tokio::test]
async fn rate_limit_rejects_burst() {
    let _g = bus_guard().await;
    telegram::bus_reset();
    let data = temp_data("bus-rate");
    save_relay(&data, "-100r", "123:rate-secret", &[]);
    let first = telegram::bus_send(&data, true, &json!({ "from": "a", "body": "1" })).await;
    assert_eq!(first["ok"], true, "{first}");
    let second = telegram::bus_send(&data, true, &json!({ "from": "a", "body": "2" })).await;
    assert_eq!(second["ok"], false, "{second}");
    assert!(
        second["error"]
            .as_str()
            .unwrap_or("")
            .to_lowercase()
            .contains("rate"),
        "{second}"
    );
}

#[test]
fn card_telegram_shows_last_bus() {
    assert!(CARD_NAMES.contains(&"telegram"));
    assert_eq!(CARD_NAMES.len(), 42);
    let html = render_card(
        "telegram",
        &json!({
            "ok": true,
            "token_set": true,
            "channel_id": "-100",
            "polling": true,
            "dry_run": true,
            "last_bus_ts": "2026-08-19T00:00:00Z",
            "last_bus_error": "rate limited",
            "last_ticket_id": "t-174"
        }),
    )
    .expect("card");
    assert!(html.contains("polling"), "{html}");
    assert!(html.contains("2026-08-19T00:00:00Z"), "{html}");
    assert!(html.contains("rate limited"), "{html}");
    assert!(html.contains("t-174"), "{html}");
    assert!(html.contains("gsv_telegram_decode"), "{html}");
    assert!(!html.contains("bot_token"), "{html}");
}

#[tokio::test]
async fn http_bus_round_trip_and_csrf() {
    let _g = bus_guard().await;
    telegram::bus_reset();
    let data = temp_data("http-bus");
    save_relay(&data, "-100h", "123:http-bus-secret", &[]);
    let app = app_with_data(data);
    let (cross, cjson) = post_json(
        &app,
        "/api/telegram/bus",
        json!({ "from": "cursor", "body": "csrf" }),
        Some("https://example.com"),
        Some("cross-site"),
    )
    .await;
    assert_eq!(cross, StatusCode::FORBIDDEN, "{cjson}");
    let (status, sent) = post_json(
        &app,
        "/api/telegram/bus",
        json!({ "from": "cursor", "to": "opencode", "body": "hello-bus" }),
        Some("http://127.0.0.1:9999"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{sent}");
    assert_eq!(sent["ok"], true, "{sent}");
    assert_no_secret(&sent, "123:http-bus-secret");
    let (gstatus, got) = get_json(&app, "/api/telegram/bus?limit=4").await;
    assert_eq!(gstatus, StatusCode::OK, "{got}");
    assert_eq!(got["ok"], true, "{got}");
    assert_eq!(got["messages"][0]["body"], "hello-bus");
    assert_no_secret(&got, "123:http-bus-secret");
    let (sstatus, st) =
        get_json_headers(&app, "/api/telegram", &[("x-telegram-dry-run", "1")]).await;
    assert_eq!(sstatus, StatusCode::OK, "{st}");
    assert_eq!(st["polling"], true);
    assert!(!st["last_bus_ts"].as_str().unwrap_or("").is_empty(), "{st}");
    assert_no_secret(&st, "123:http-bus-secret");
}

#[tokio::test]
async fn mcp_bus_send_and_poll() {
    let _g = bus_guard().await;
    telegram::bus_reset();
    let data = temp_data("mcp-bus");
    save_relay(&data, "-100mcpbus", "mcp-bus-secret-token", &[]);
    let app = app_with_data(data);
    let send = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/mcp")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .header("x-telegram-dry-run", "1")
                .body(Body::from(
                    json!({
                        "jsonrpc": "2.0",
                        "id": 1,
                        "method": "tools/call",
                        "params": {
                            "name": "gsv_telegram_bus_send",
                            "arguments": { "from": "cursor", "body": "mcp-hi" }
                        }
                    })
                    .to_string(),
                ))
                .expect("req"),
        )
        .await
        .expect("resp");
    assert_eq!(send.status(), StatusCode::OK);
    let sb = axum::body::to_bytes(send.into_body(), usize::MAX)
        .await
        .expect("body");
    let sj: Value = serde_json::from_slice(&sb).expect("json");
    assert_eq!(sj["result"]["isError"], false, "{sj}");
    let stext = sj["result"]["content"][0]["text"].as_str().unwrap_or("");
    assert!(!stext.contains("bot_token"), "{stext}");
    assert!(!stext.contains("mcp-bus-secret-token"), "{stext}");
    let poll = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/mcp")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .header("x-telegram-dry-run", "1")
                .body(Body::from(
                    json!({
                        "jsonrpc": "2.0",
                        "id": 2,
                        "method": "tools/call",
                        "params": {
                            "name": "gsv_telegram_bus_poll",
                            "arguments": { "limit": 4 }
                        }
                    })
                    .to_string(),
                ))
                .expect("req"),
        )
        .await
        .expect("resp");
    let pb = axum::body::to_bytes(poll.into_body(), usize::MAX)
        .await
        .expect("body");
    let pj: Value = serde_json::from_slice(&pb).expect("json");
    assert_eq!(pj["result"]["isError"], false, "{pj}");
    let ptext = pj["result"]["content"][0]["text"].as_str().unwrap_or("");
    assert!(ptext.contains("mcp-hi"), "{ptext}");
    assert!(!ptext.contains("mcp-bus-secret-token"), "{ptext}");
}

fn nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

fn temp_kit(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "gsv-tg-ticket-{tag}-{}-{}",
        std::process::id(),
        nanos()
    ));
    let _ = std::fs::create_dir_all(dir.join("docs/gsv"));
    let _ = std::fs::create_dir_all(dir.join("data"));
    dir
}

fn app_kit(kit: PathBuf) -> axum::Router {
    let data = kit.join("data");
    let (tx, _rx) = broadcast::channel(32);
    let state = AppState::new(Some(kit), Some(data), tx);
    router(state)
}

fn save_solo_relay(data: &Path, channel: &str, token: &str, allowed: &[&str]) {
    settings::save(
        data,
        &settings::SettingsFile {
            godfather: settings::Godfather {
                channel_id: channel.into(),
                allowed_user_ids: allowed.iter().map(|s| (*s).to_string()).collect(),
                bot_token: token.into(),
                poll: false,
                role: String::new(),
            },
            workflows: settings::Workflows {
                enabled: vec!["telegram-relay".into(), "ticket-claim".into()],
            },
            tickets: settings::TicketsSettings {
                mode: "solo".into(),
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .expect("save solo relay");
}

fn solo_who() -> ClaimedBy {
    ClaimedBy {
        actor: "agent".into(),
        ide: "cursor".into(),
        model: "grok-4.6".into(),
        agent: "orchestrator".into(),
    }
}

#[tokio::test]
async fn slash_ticket_solo_bot_claims() {
    let _g = bus_guard().await;
    telegram::bus_reset();
    let kit = temp_kit("solo");
    let data = kit.join("data");
    save_solo_relay(&data, "-100solo", "123:solo-secret", &[]);
    let store = Mutex::new(HashMap::new());
    tickets::heartbeat(&store, &solo_who());
    let v = telegram::ticket_from_message(
        &kit,
        &data,
        true,
        &json!({ "from": "42", "body": "/ticket Fix live copy" }),
        Some(&store),
    );
    assert_eq!(v["ok"], true, "{v}");
    assert_eq!(v["dry_run"], true, "{v}");
    assert_eq!(v["ticket"]["status"], "in_progress", "{v}");
    assert_eq!(v["ticket"]["title"], "Fix live copy", "{v}");
    assert_eq!(v["ticket"]["workflow"], "telegram", "{v}");
    assert_eq!(v["ticket"]["claimed_by"]["actor"], "agent", "{v}");
    assert_eq!(v["envelope"]["kind"], "ticket", "{v}");
    assert_no_secret(&v, "123:solo-secret");
    let claims = std::fs::read_to_string(tickets::claims_path(&kit)).expect("claims");
    assert!(claims.contains("\"kind\":\"telegram\""), "{claims}");
    assert!(claims.contains("\"kind\":\"claimed\""), "{claims}");
}

#[tokio::test]
async fn telegram_ticket_stays_open_without_presence() {
    let _g = bus_guard().await;
    telegram::bus_reset();
    let kit = temp_kit("open");
    let data = kit.join("data");
    save_solo_relay(&data, "-100open", "123:open-secret", &[]);
    let store = Mutex::new(HashMap::new());
    let v = telegram::ticket_from_message(
        &kit,
        &data,
        true,
        &json!({ "from": "42", "body": "/ticket Nobody home" }),
        Some(&store),
    );
    assert_eq!(v["ok"], true, "{v}");
    assert_eq!(v["ticket"]["status"], "open", "{v}");
    assert!(v["ticket"]["claimed_by"].is_null(), "{v}");
    assert_no_secret(&v, "123:open-secret");
}

#[tokio::test]
async fn telegram_ticket_requires_relay_and_claim() {
    let _g = bus_guard().await;
    telegram::bus_reset();
    let kit = temp_kit("gate");
    let data = kit.join("data");
    save_godfather(&data, "-100g", "123:gate-secret");
    let v = telegram::ticket_from_message(
        &kit,
        &data,
        true,
        &json!({ "from": "42", "body": "/ticket gated" }),
        None,
    );
    assert_eq!(v["ok"], false, "{v}");
    assert!(
        v["error"].as_str().unwrap_or("").contains("telegram-relay"),
        "{v}"
    );
    save_relay(&data, "-100g", "123:gate-secret", &[]);
    let v2 = telegram::ticket_from_message(
        &kit,
        &data,
        true,
        &json!({ "from": "42", "body": "/ticket gated" }),
        None,
    );
    assert_eq!(v2["ok"], false, "{v2}");
    assert!(
        v2["error"].as_str().unwrap_or("").contains("ticket-claim"),
        "{v2}"
    );
    assert_no_secret(&v2, "123:gate-secret");
}

#[tokio::test]
async fn telegram_ticket_allowlist_and_bus_json() {
    let _g = bus_guard().await;
    telegram::bus_reset();
    let kit = temp_kit("allow");
    let data = kit.join("data");
    save_solo_relay(&data, "-100a", "123:alw-secret", &["42"]);
    let bad = telegram::ticket_from_message(
        &kit,
        &data,
        true,
        &json!({ "from": "99", "body": "/ticket nope" }),
        None,
    );
    assert_eq!(bad["ok"], false, "{bad}");
    let bus = telegram::ticket_from_message(
        &kit,
        &data,
        true,
        &json!({
            "from": "42",
            "body": r#"{"v":1,"kind":"bus","from":"42","body":"x"}"#
        }),
        None,
    );
    assert_eq!(bus["ok"], false, "{bus}");
    assert!(
        bus["error"].as_str().unwrap_or("").contains("not a ticket"),
        "{bus}"
    );
    assert_no_secret(&bus, "123:alw-secret");
}

#[tokio::test]
async fn http_telegram_ticket_csrf_and_solo() {
    let _g = bus_guard().await;
    telegram::bus_reset();
    let kit = temp_kit("http-tix");
    save_solo_relay(&kit.join("data"), "-100ht", "123:http-tix-secret", &[]);
    let app = app_kit(kit);
    let (cross, cjson) = post_json(
        &app,
        "/api/telegram/ticket",
        json!({ "from": "42", "body": "/ticket csrf" }),
        Some("https://example.com"),
        Some("cross-site"),
    )
    .await;
    assert_eq!(cross, StatusCode::FORBIDDEN, "{cjson}");
    let (_, pres) = post_json(
        &app,
        "/api/tickets/presence",
        json!({ "actor": "agent", "ide": "cursor", "agent": "orchestrator" }),
        Some("http://127.0.0.1:9999"),
        None,
    )
    .await;
    assert_eq!(pres["ok"], true, "{pres}");
    let (status, sent) = post_json(
        &app,
        "/api/telegram/ticket",
        json!({ "from": "42", "body": "/ticket HTTP solo" }),
        Some("http://127.0.0.1:9999"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{sent}");
    assert_eq!(sent["ok"], true, "{sent}");
    assert_eq!(sent["ticket"]["status"], "in_progress", "{sent}");
    assert_no_secret(&sent, "123:http-tix-secret");
}

#[tokio::test]
async fn mcp_telegram_ticket_solo() {
    let _g = bus_guard().await;
    telegram::bus_reset();
    let kit = temp_kit("mcp-tix");
    save_solo_relay(&kit.join("data"), "-100mcp", "mcp-tix-secret-token", &[]);
    let app = app_kit(kit);
    let _ = post_json(
        &app,
        "/api/tickets/presence",
        json!({ "actor": "agent", "ide": "cursor", "agent": "orchestrator" }),
        Some("http://127.0.0.1:9999"),
        None,
    )
    .await;
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/mcp")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .header("x-telegram-dry-run", "1")
                .body(Body::from(
                    json!({
                        "jsonrpc": "2.0",
                        "id": 7,
                        "method": "tools/call",
                        "params": {
                            "name": "gsv_telegram_ticket",
                            "arguments": { "from": "42", "body": "/ticket MCP solo" }
                        }
                    })
                    .to_string(),
                ))
                .expect("req"),
        )
        .await
        .expect("resp");
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .expect("body");
    let json: Value = serde_json::from_slice(&bytes).expect("json");
    assert_eq!(json["result"]["isError"], false, "{json}");
    let text = json["result"]["content"][0]["text"].as_str().unwrap_or("");
    assert!(
        text.contains("MCP solo") || text.contains("in_progress"),
        "{text}"
    );
    assert!(!text.contains("bot_token"), "{text}");
    assert!(!text.contains("mcp-tix-secret-token"), "{text}");
}

#[tokio::test]
async fn enqueue_sync_is_kind_sync_no_token() {
    let _g = bus_guard().await;
    telegram::bus_reset();
    let env = telegram::enqueue_sync("solo", "t-mds", "claimed").expect("sync");
    assert_eq!(env.kind, "sync");
    assert_eq!(env.ticket_id.as_deref(), Some("t-mds"));
    assert_eq!(env.body, "solo claimed t-mds");
    let raw = serde_json::to_string(&env).expect("json");
    assert!(!raw.contains("bot_token"), "{raw}");
}

#[tokio::test]
async fn hook_phrase_ingests_catalog_band() {
    let _g = bus_guard().await;
    telegram::bus_reset();
    let kit = temp_kit("hook");
    let data = kit.join("data");
    save_solo_relay(&data, "-100hook", "123:hook-secret", &[]);
    std::fs::write(
        tickets::scenarios_path(&kit),
        r#"{
          "scenarios": [{
            "id": "memory-disk-speed",
            "title": "MDS",
            "body": "band",
            "workflow": "ticket-claim",
            "product": "gsv",
            "tickets": [
              {"title": "MDS: scaffold", "body": "a"},
              {"title": "MDS: memory", "body": "b"}
            ]
          }]
        }"#,
    )
    .expect("scenarios");
    let v = telegram::ingest_channel_body(
        &kit,
        &data,
        true,
        &json!({
            "from": "42",
            "body": "run mcp bot hook up scenario memory-disk-speed"
        }),
        None,
    )
    .await;
    assert_eq!(v["ok"], true, "{v}");
    assert_eq!(v["source"], "scenario", "{v}");
    assert_eq!(v["tickets"].as_array().expect("t").len(), 2, "{v}");
    assert_no_secret(&v, "123:hook-secret");
}

#[tokio::test]
async fn poll_once_classifies_ticket_bus_hook_and_skip() {
    let _g = bus_guard().await;
    telegram::bus_reset();
    let kit = temp_kit("poll");
    let data = kit.join("data");
    save_solo_relay(&data, "-100poll", "123:poll-secret", &[]);
    telegram::push_inbound_stub(1, "hello channel", "-100poll", "", "42");
    telegram::push_inbound_stub(
        2,
        r#"{"v":1,"kind":"bus","from":"cursor","body":"ping"}"#,
        "-100poll",
        "",
        "42",
    );
    telegram::push_inbound_stub(3, "/ticket Poll ingest", "-100poll", "", "42");
    telegram::push_inbound_stub(4, "solo claimed Session: S0 disk", "-100poll", "", "42");
    telegram::push_inbound_stub(
        5,
        "solo claimed Session: S0 disk\n{\"v\":1,\"kind\":\"sync\",\"from\":\"solo\",\"ticket_id\":\"t-1\",\"body\":\"solo claimed Session: S0 disk\",\"data\":{\"hint\":\"work-ticket\",\"next\":\"PH-S2459\"}}",
        "-100poll",
        "",
        "42",
    );
    let v = telegram::poll_once(&kit, &data, true, None).await;
    assert_eq!(v["ok"], true, "{v}");
    assert_eq!(v["dry_run"], true, "{v}");
    assert_eq!(v["bus"], 2, "{v}");
    assert_eq!(v["ticket"], 1, "{v}");
    assert_eq!(v["skip"], 2, "{v}");
    assert_eq!(v["poll_alive"], false, "{v}");
    assert!(v["update_offset"].as_i64().unwrap_or(0) >= 5, "{v}");
    assert_no_secret(&v, "123:poll-secret");
    let bus = telegram::bus_poll(&data, true, Some(8)).await;
    assert_eq!(bus["ok"], true, "{bus}");
    let msgs = bus["messages"].as_array().expect("msgs");
    assert!(
        msgs.iter()
            .any(|m| m["kind"] == "bus" && m["body"] == "ping"),
        "{bus}"
    );
    assert!(
        msgs.iter().any(|m| m["kind"] == "sync"
            && m["data"]["hint"] == "work-ticket"
            && m["data"]["next"] == "PH-S2459"),
        "{bus}"
    );
}

#[tokio::test]
async fn poll_once_ingests_federated_presence() {
    let _g = bus_guard().await;
    telegram::bus_reset();
    let kit = temp_kit("poll-presence");
    let data = kit.join("data");
    save_solo_relay(&data, "-100fp", "123:fp-secret", &[]);
    let store = tickets::new_presence_store();
    telegram::push_inbound_stub(
        31,
        r#"{"v":1,"kind":"presence","from":"alice-gsv","body":"alice-gsv heartbeat","data":{"actor":"alice","ide":"opencode","agent":"bot","jail_id":"alice-gsv","rank_id":"jun-nub","rank_title":"Jun-nub","hint":"heartbeat"}}"#,
        "-100fp",
        "",
        "99",
    );
    let v = telegram::poll_once(&kit, &data, true, Some(&store)).await;
    assert_eq!(v["ok"], true, "{v}");
    assert_eq!(v["presence"], 1, "{v}");
    assert_eq!(tickets::federation_now(&store).len(), 1);
    assert_eq!(tickets::online_local(&store).len(), 0);
    let fed = &tickets::federation_now(&store)[0];
    assert_eq!(fed.jail_id, "alice-gsv");
    assert_eq!(fed.actor, "alice");
    assert_eq!(fed.rank_title, "Jun-nub");
    assert_no_secret(&v, "123:fp-secret");
}

#[tokio::test]
async fn poll_once_ingests_federated_claim() {
    let _g = bus_guard().await;
    telegram::bus_reset();
    let kit = temp_kit("poll-claim");
    let data = kit.join("data");
    save_solo_relay(&data, "-100fc", "123:fc-secret", &[]);
    let t = tickets::create(&kit, "Federated claim row", "body", "gsv").expect("create");
    telegram::push_inbound_stub(
        41,
        &format!(
            r#"{{"v":1,"kind":"claim","from":"alice-gsv","ticket_id":"{}","body":"alice-gsv claims","data":{{"actor":"alice","ide":"opencode","agent":"bot","jail_id":"alice-gsv","hint":"federated-claim"}}}}"#,
            t.id
        ),
        "-100fc",
        "",
        "99",
    );
    let v = telegram::poll_once(&kit, &data, true, None).await;
    assert_eq!(v["ok"], true, "{v}");
    assert_eq!(v["claim"], 1, "{v}");
    let listed = tickets::list(&kit);
    let row = listed["tickets"]
        .as_array()
        .expect("arr")
        .iter()
        .find(|x| x["id"] == t.id)
        .expect("row");
    assert_eq!(row["status"], "in_progress");
    assert_eq!(row["claimed_jail"], "alice-gsv");
    assert_eq!(row["claimed_by"]["actor"], "alice");
    assert_no_secret(&v, "123:fc-secret");
}

#[test]
fn federated_claim_echo_and_guest_skip() {
    telegram::bus_reset();
    let kit = temp_kit("claim-echo");
    let data = kit.join("data");
    save_solo_relay(&data, "-100echo", "123:echo-secret", &[]);
    settings::save(
        &data,
        &settings::SettingsFile {
            jail: settings::JailSettings {
                id: "alice-gsv".into(),
            },
            godfather: settings::Godfather {
                channel_id: "-100echo".into(),
                bot_token: "123:echo-secret".into(),
                ..Default::default()
            },
            workflows: settings::Workflows {
                enabled: vec!["telegram-relay".into(), "ticket-claim".into()],
            },
            ..Default::default()
        },
    )
    .expect("save echo jail");
    let t = tickets::create(&kit, "Echo skip", "body", "gsv").expect("create");
    let env = telegram::parse_envelope(&json!({
        "v": 1,
        "kind": "claim",
        "from": "alice-gsv",
        "ticket_id": t.id,
        "body": "alice-gsv claims",
        "data": { "jail_id": "alice-gsv", "actor": "alice" }
    }))
    .expect("env");
    assert!(!telegram::apply_claim_envelope(&kit, &data, &env));
    let still = tickets::list(&kit);
    let row = still["tickets"]
        .as_array()
        .expect("arr")
        .iter()
        .find(|x| x["id"] == t.id)
        .expect("row");
    assert_eq!(row["status"], "open");

    let kit_g = temp_kit("claim-guest");
    let data_g = kit_g.join("data");
    settings::save(
        &data_g,
        &settings::SettingsFile {
            godfather: settings::Godfather {
                role: "guest".into(),
                channel_id: "-100g".into(),
                bot_token: "123:guest-claim".into(),
                ..Default::default()
            },
            workflows: settings::Workflows {
                enabled: vec!["telegram-relay".into(), "ticket-claim".into()],
            },
            ..Default::default()
        },
    )
    .expect("save guest");
    let tg = tickets::create(&kit_g, "Guest board", "body", "gsv").expect("create");
    let env_g = telegram::parse_envelope(&json!({
        "v": 1,
        "kind": "claim",
        "from": "alice-gsv",
        "ticket_id": tg.id,
        "body": "alice-gsv claims",
        "data": { "actor": "alice", "jail_id": "alice-gsv" }
    }))
    .expect("envg");
    assert!(!telegram::apply_claim_envelope(&kit_g, &data_g, &env_g));
    let who = ClaimedBy {
        actor: "guest".into(),
        ide: "cursor".into(),
        model: "m".into(),
        agent: "bot".into(),
    };
    let file = settings::load_result(&data_g).expect("load");
    assert!(!telegram::maybe_federate_claim(&file, &who, &tg.id));
}

#[test]
fn federated_done_envelope_parse_and_classify() {
    let done = telegram::parse_envelope(&json!({
        "v": 1,
        "kind": "done",
        "from": "alice-gsv",
        "ticket_id": "t-7",
        "body": "alice-gsv done t-7",
        "data": { "actor": "alice", "hint": "band closed" }
    }))
    .expect("done");
    assert_eq!(done.kind, "done");
    assert_eq!(done.ticket_id.as_deref(), Some("t-7"));
    assert_eq!(
        telegram::classify_inbound(&serde_json::to_string(&done).unwrap()),
        "done"
    );
    let no_id = telegram::parse_envelope(&json!({
        "v": 1,
        "kind": "done",
        "from": "alice-gsv",
        "body": "no id"
    }));
    assert!(no_id.is_err(), "{no_id:?}");
}

#[tokio::test]
async fn poll_once_ingests_federated_done() {
    let _g = bus_guard().await;
    telegram::bus_reset();
    let kit = temp_kit("poll-done");
    let data = kit.join("data");
    save_solo_relay(&data, "-100fc", "123:fc-secret", &[]);
    let t = tickets::create(&kit, "Federated done row", "body", "gsv").expect("create");
    let who = ClaimedBy {
        actor: "host".into(),
        ide: "cursor".into(),
        model: String::new(),
        agent: "orchestrator".into(),
    };
    tickets::claim_with(&kit, &data, &t.id, who, None).expect("claim");
    telegram::push_inbound_stub(
        42,
        &format!(
            r#"{{"v":1,"kind":"done","from":"alice-gsv","ticket_id":"{}","body":"alice-gsv done","data":{{"actor":"alice","ide":"opencode","agent":"bot","jail_id":"alice-gsv","hint":"work finished"}}}}"#,
            t.id
        ),
        "-100fc",
        "",
        "99",
    );
    let v = telegram::poll_once(&kit, &data, true, None).await;
    assert_eq!(v["ok"], true, "{v}");
    assert_eq!(v["done"], 1, "{v}");
    let listed = tickets::list(&kit);
    let row = listed["tickets"]
        .as_array()
        .expect("arr")
        .iter()
        .find(|x| x["id"] == t.id)
        .expect("row");
    assert_eq!(row["status"], "done");
    assert_no_secret(&v, "123:fc-secret");
}

#[test]
fn federated_done_echo_guest_and_noop_skip() {
    telegram::bus_reset();
    let kit = temp_kit("done-echo");
    let data = kit.join("data");
    save_solo_relay(&data, "-100echo", "123:echo-secret", &[]);
    settings::save(
        &data,
        &settings::SettingsFile {
            jail: settings::JailSettings {
                id: "alice-gsv".into(),
            },
            godfather: settings::Godfather {
                channel_id: "-100echo".into(),
                bot_token: "123:echo-secret".into(),
                ..Default::default()
            },
            workflows: settings::Workflows {
                enabled: vec!["telegram-relay".into(), "ticket-claim".into()],
            },
            ..Default::default()
        },
    )
    .expect("save echo jail");
    let t = tickets::create(&kit, "Done echo skip", "body", "gsv").expect("create");
    let env = telegram::parse_envelope(&json!({
        "v": 1,
        "kind": "done",
        "from": "alice-gsv",
        "ticket_id": t.id,
        "body": "alice-gsv done",
        "data": { "jail_id": "alice-gsv", "actor": "alice" }
    }))
    .expect("env");
    assert!(!telegram::apply_done_envelope(&kit, &data, &env));
    let still = tickets::list(&kit);
    let row = still["tickets"]
        .as_array()
        .expect("arr")
        .iter()
        .find(|x| x["id"] == t.id)
        .expect("row");
    assert_eq!(row["status"], "open");

    let kit_g = temp_kit("done-guest");
    let data_g = kit_g.join("data");
    settings::save(
        &data_g,
        &settings::SettingsFile {
            godfather: settings::Godfather {
                role: "guest".into(),
                channel_id: "-100g".into(),
                bot_token: "123:guest-done".into(),
                ..Default::default()
            },
            workflows: settings::Workflows {
                enabled: vec!["telegram-relay".into(), "ticket-claim".into()],
            },
            ..Default::default()
        },
    )
    .expect("save guest");
    let tg = tickets::create(&kit_g, "Guest done board", "body", "gsv").expect("create");
    let env_g = telegram::parse_envelope(&json!({
        "v": 1,
        "kind": "done",
        "from": "alice-gsv",
        "ticket_id": tg.id,
        "body": "alice-gsv done",
        "data": { "actor": "alice", "jail_id": "alice-gsv" }
    }))
    .expect("envg");
    assert!(!telegram::apply_done_envelope(&kit_g, &data_g, &env_g));

    let noop = telegram::parse_envelope(&json!({
        "v": 1,
        "kind": "done",
        "from": "bob-gsv",
        "ticket_id": t.id,
        "body": "bob closes an open row",
        "data": { "actor": "bob", "jail_id": "bob-gsv" }
    }))
    .expect("noop env");
    assert!(!telegram::apply_done_envelope(&kit, &data, &noop));

    let who = ClaimedBy {
        actor: "guest".into(),
        ide: "cursor".into(),
        model: "m".into(),
        agent: "bot".into(),
    };
    let file = settings::load_result(&data_g).expect("load");
    assert!(!telegram::maybe_federate_done(&file, &who, &tg.id, ""));
}

#[tokio::test]
async fn bus_send_kind_done_dry_run() {
    let _g = bus_guard().await;
    telegram::bus_reset();
    let data = temp_data("bus-done");
    save_relay(&data, "-100done", "123:done-secret", &[]);
    let missing = telegram::bus_send(
        &data,
        true,
        &json!({ "from": "cursor", "kind": "done", "body": "x" }),
    )
    .await;
    assert_eq!(missing["ok"], false, "{missing}");
    telegram::bus_clear_rate_limit();
    let sent = telegram::bus_send(
        &data,
        true,
        &json!({ "from": "cursor", "kind": "done", "ticket_id": "t-5" }),
    )
    .await;
    assert_eq!(sent["ok"], true, "{sent}");
    assert_eq!(sent["envelope"]["kind"], "done");
    assert_eq!(sent["envelope"]["ticket_id"], "t-5");
    assert_eq!(sent["envelope"]["body"], "cursor done t-5");
    assert_eq!(sent["envelope"]["data"]["hint"], "federated-done");
    assert_no_secret(&sent, "123:done-secret");
}

#[test]
fn federated_reclaim_envelope_parse_and_classify() {
    let reclaim = telegram::parse_envelope(&json!({
        "v": 1,
        "kind": "reclaim",
        "from": "alice-gsv",
        "ticket_id": "t-7",
        "body": "alice-gsv reclaims t-7",
        "data": { "actor": "alice", "hint": "federated-reclaim" }
    }))
    .expect("reclaim");
    assert_eq!(reclaim.kind, "reclaim");
    assert_eq!(reclaim.ticket_id.as_deref(), Some("t-7"));
    assert_eq!(
        telegram::classify_inbound(&serde_json::to_string(&reclaim).unwrap()),
        "reclaim"
    );
    let no_id = telegram::parse_envelope(&json!({
        "v": 1,
        "kind": "reclaim",
        "from": "alice-gsv",
        "body": "no id"
    }));
    assert!(no_id.is_err(), "{no_id:?}");
}

#[tokio::test]
async fn poll_once_ingests_federated_reclaim() {
    let _g = bus_guard().await;
    telegram::bus_reset();
    let kit = temp_kit("poll-reclaim");
    let data = kit.join("data");
    save_solo_relay(&data, "-100fr", "123:fr-secret", &[]);
    let t = tickets::create(&kit, "Federated reclaim row", "body", "gsv").expect("create");
    let who = ClaimedBy {
        actor: "host".into(),
        ide: "cursor".into(),
        model: String::new(),
        agent: "orchestrator".into(),
    };
    tickets::claim_with(&kit, &data, &t.id, who, None).expect("claim");
    telegram::push_inbound_stub(
        43,
        &format!(
            r#"{{"v":1,"kind":"reclaim","from":"alice-gsv","ticket_id":"{}","body":"alice-gsv reclaims","data":{{"actor":"alice","ide":"opencode","agent":"bot","jail_id":"alice-gsv","hint":"federated-reclaim"}}}}"#,
            t.id
        ),
        "-100fr",
        "",
        "99",
    );
    let v = telegram::poll_once(&kit, &data, true, None).await;
    assert_eq!(v["ok"], true, "{v}");
    assert_eq!(v["reclaim"], 1, "{v}");
    let listed = tickets::list(&kit);
    let row = listed["tickets"]
        .as_array()
        .expect("arr")
        .iter()
        .find(|x| x["id"] == t.id)
        .expect("row");
    assert_eq!(row["status"], "open");
    assert_eq!(row["claimed_by"], Value::Null);
    assert_no_secret(&v, "123:fr-secret");
}

#[test]
fn federated_reclaim_echo_guest_and_noop_skip() {
    telegram::bus_reset();
    let kit = temp_kit("reclaim-echo");
    let data = kit.join("data");
    save_solo_relay(&data, "-100echo", "123:echo-secret", &[]);
    settings::save(
        &data,
        &settings::SettingsFile {
            jail: settings::JailSettings {
                id: "alice-gsv".into(),
            },
            godfather: settings::Godfather {
                channel_id: "-100echo".into(),
                bot_token: "123:echo-secret".into(),
                ..Default::default()
            },
            workflows: settings::Workflows {
                enabled: vec!["telegram-relay".into(), "ticket-claim".into()],
            },
            ..Default::default()
        },
    )
    .expect("save echo jail");
    let t = tickets::create(&kit, "Reclaim echo skip", "body", "gsv").expect("create");
    let who = ClaimedBy {
        actor: "host".into(),
        ide: "cursor".into(),
        model: String::new(),
        agent: "orchestrator".into(),
    };
    tickets::claim_with(&kit, &data, &t.id, who, None).expect("claim");
    let env = telegram::parse_envelope(&json!({
        "v": 1,
        "kind": "reclaim",
        "from": "alice-gsv",
        "ticket_id": t.id,
        "body": "alice-gsv reclaims",
        "data": { "jail_id": "alice-gsv", "actor": "alice" }
    }))
    .expect("env");
    assert!(!telegram::apply_reclaim_envelope(&kit, &data, &env));
    let still = tickets::list(&kit);
    let row = still["tickets"]
        .as_array()
        .expect("arr")
        .iter()
        .find(|x| x["id"] == t.id)
        .expect("row");
    assert_eq!(row["status"], "in_progress");

    let kit_g = temp_kit("reclaim-guest");
    let data_g = kit_g.join("data");
    settings::save(
        &data_g,
        &settings::SettingsFile {
            godfather: settings::Godfather {
                role: "guest".into(),
                channel_id: "-100g".into(),
                bot_token: "123:guest-reclaim".into(),
                ..Default::default()
            },
            workflows: settings::Workflows {
                enabled: vec!["telegram-relay".into(), "ticket-claim".into()],
            },
            ..Default::default()
        },
    )
    .expect("save guest");
    let tg = tickets::create(&kit_g, "Guest reclaim board", "body", "gsv").expect("create");
    let env_g = telegram::parse_envelope(&json!({
        "v": 1,
        "kind": "reclaim",
        "from": "alice-gsv",
        "ticket_id": tg.id,
        "body": "alice-gsv reclaims",
        "data": { "actor": "alice", "jail_id": "alice-gsv" }
    }))
    .expect("envg");
    assert!(!telegram::apply_reclaim_envelope(&kit_g, &data_g, &env_g));

    let noop = telegram::parse_envelope(&json!({
        "v": 1,
        "kind": "reclaim",
        "from": "bob-gsv",
        "ticket_id": tg.id,
        "body": "bob releases an open row",
        "data": { "actor": "bob", "jail_id": "bob-gsv" }
    }))
    .expect("noop env");
    assert!(!telegram::apply_reclaim_envelope(&kit, &data, &noop));

    let who = ClaimedBy {
        actor: "guest".into(),
        ide: "cursor".into(),
        model: "m".into(),
        agent: "bot".into(),
    };
    let file = settings::load_result(&data_g).expect("load");
    assert!(!telegram::maybe_federate_reclaim(&file, &who, &tg.id));
}

#[tokio::test]
async fn bus_send_kind_reclaim_dry_run() {
    let _g = bus_guard().await;
    telegram::bus_reset();
    let data = temp_data("bus-reclaim");
    save_relay(&data, "-100reclaim", "123:reclaim-secret", &[]);
    let missing = telegram::bus_send(
        &data,
        true,
        &json!({ "from": "cursor", "kind": "reclaim", "body": "x" }),
    )
    .await;
    assert_eq!(missing["ok"], false, "{missing}");
    telegram::bus_clear_rate_limit();
    let sent = telegram::bus_send(
        &data,
        true,
        &json!({ "from": "cursor", "kind": "reclaim", "ticket_id": "t-6" }),
    )
    .await;
    assert_eq!(sent["ok"], true, "{sent}");
    assert_eq!(sent["envelope"]["kind"], "reclaim");
    assert_eq!(sent["envelope"]["ticket_id"], "t-6");
    assert_eq!(sent["envelope"]["body"], "cursor reclaims t-6");
    assert_eq!(sent["envelope"]["data"]["hint"], "federated-reclaim");
    assert_no_secret(&sent, "123:reclaim-secret");
}

#[test]
fn guest_does_not_federate_presence() {
    telegram::bus_reset();
    let file = settings::SettingsFile {
        godfather: settings::Godfather {
            role: "guest".into(),
            channel_id: "-100g".into(),
            bot_token: "123:guest".into(),
            ..Default::default()
        },
        workflows: settings::Workflows {
            enabled: vec!["telegram-relay".into()],
        },
        ..Default::default()
    };
    let who = ClaimedBy {
        actor: "guest".into(),
        ide: "cursor".into(),
        model: "m".into(),
        agent: "bot".into(),
    };
    assert!(!telegram::maybe_federate_presence(
        &file, &who, "jun-nub", "Jun-nub"
    ));
}

#[tokio::test]
async fn http_telegram_poll_csrf_and_dry_run() {
    let _g = bus_guard().await;
    telegram::bus_reset();
    let kit = temp_kit("http-poll");
    save_solo_relay(&kit.join("data"), "-100hp", "123:http-poll-secret", &[]);
    telegram::push_inbound_stub(11, "/ticket HTTP poll", "-100hp", "", "42");
    let app = app_kit(kit);
    let (cross, cjson) = post_json(
        &app,
        "/api/telegram/poll",
        json!({}),
        Some("https://example.com"),
        Some("cross-site"),
    )
    .await;
    assert_eq!(cross, StatusCode::FORBIDDEN, "{cjson}");
    let (status, sent) = post_json(
        &app,
        "/api/telegram/poll",
        json!({}),
        Some("http://127.0.0.1:9999"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{sent}");
    assert_eq!(sent["ok"], true, "{sent}");
    assert_eq!(sent["ticket"], 1, "{sent}");
    assert_no_secret(&sent, "123:http-poll-secret");
}

#[tokio::test]
async fn mcp_telegram_poll_dry_run() {
    let _g = bus_guard().await;
    telegram::bus_reset();
    let kit = temp_kit("mcp-poll");
    save_solo_relay(&kit.join("data"), "-100mp", "mcp-poll-secret-token", &[]);
    telegram::push_inbound_stub(21, "/ticket MCP poll", "-100mp", "", "42");
    let app = app_kit(kit);
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/mcp")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .header("x-telegram-dry-run", "1")
                .body(Body::from(
                    json!({
                        "jsonrpc": "2.0",
                        "id": 8,
                        "method": "tools/call",
                        "params": { "name": "gsv_telegram_poll", "arguments": {} }
                    })
                    .to_string(),
                ))
                .expect("req"),
        )
        .await
        .expect("resp");
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .expect("body");
    let json: Value = serde_json::from_slice(&bytes).expect("json");
    assert_eq!(json["result"]["isError"], false, "{json}");
    let text = json["result"]["content"][0]["text"].as_str().unwrap_or("");
    assert!(
        text.contains("\"ticket\":1") || text.contains("MCP poll"),
        "{text}"
    );
    assert!(!text.contains("bot_token"), "{text}");
    assert!(!text.contains("mcp-poll-secret-token"), "{text}");
}

#[tokio::test]
async fn http_telegram_decode_csrf_and_hint() {
    let app = app_with_data(temp_data("decode"));
    let text = "solo claimed Session: S0 disk\n{\"v\":1,\"kind\":\"sync\",\"from\":\"solo\",\"ticket_id\":\"t-1\",\"body\":\"solo claimed Session: S0 disk\",\"data\":{\"hint\":\"work-ticket\",\"next\":\"PH-S2459\"}}";
    let (cross, cjson) = post_json(
        &app,
        "/api/telegram/decode",
        json!({ "text": text }),
        Some("https://example.com"),
        Some("cross-site"),
    )
    .await;
    assert_eq!(cross, StatusCode::FORBIDDEN, "{cjson}");
    let (status, sent) = post_json(
        &app,
        "/api/telegram/decode",
        json!({ "text": text }),
        Some("http://127.0.0.1:9999"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{sent}");
    assert_eq!(sent["ok"], true, "{sent}");
    assert_eq!(sent["hint"], "work-ticket", "{sent}");
    assert_eq!(sent["next"], "PH-S2459", "{sent}");
    assert!(!sent.to_string().contains("bot_token"), "{sent}");
}

#[tokio::test]
async fn mcp_telegram_decode_returns_data() {
    let app = app_with_data(temp_data("mcp-decode"));
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/mcp")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "jsonrpc": "2.0",
                        "id": 9,
                        "method": "tools/call",
                        "params": {
                            "name": "gsv_telegram_decode",
                            "arguments": {
                                "text": "{\"v\":1,\"kind\":\"sync\",\"from\":\"solo\",\"ticket_id\":\"t-1\",\"body\":\"solo done Session: close\",\"data\":{\"hint\":\"claim-next\",\"next\":\"PH-S2459\",\"crate\":\"0.181.0\"}}"
                            }
                        }
                    })
                    .to_string(),
                ))
                .expect("req"),
        )
        .await
        .expect("resp");
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .expect("body");
    let json: Value = serde_json::from_slice(&bytes).expect("json");
    assert_eq!(json["result"]["isError"], false, "{json}");
    let text = json["result"]["content"][0]["text"].as_str().unwrap_or("");
    assert!(text.contains("claim-next"), "{text}");
    assert!(text.contains("PH-S2459"), "{text}");
    assert!(!text.contains("bot_token"), "{text}");
}
