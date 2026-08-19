//! Godfather Telegram bind + MCP bus contracts (bands 167 · 169).
//!
//! Dry-run / `X-Telegram-Dry-Run: 1` never opens sockets. Missing channel or
//! token → `{ok:false}` without secrets. Band 169: in-memory bus queue, no
//! webhook, no Cloudflare. Telegram create-ticket stays out.

use std::path::{Path, PathBuf};

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use gsv::boxes::settings;
use gsv::boxes::telegram;
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
    let data = temp_data("missing");
    let w = telegram::status(&data, true).await;
    assert_eq!(w["ok"], false);
    assert_eq!(w["token_set"], false);
    assert_eq!(w["polling"], false);
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
    assert_eq!(CARD_NAMES.len(), 40);
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
    assert!(!mcp::tool_names().contains(&"gsv_telegram_create_ticket"));
    assert_eq!(mcp::tool_names().len(), 42);
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
    assert_eq!(CARD_NAMES.len(), 40);
    let html = render_card(
        "telegram",
        &json!({
            "ok": true,
            "token_set": true,
            "channel_id": "-100",
            "polling": true,
            "dry_run": true,
            "last_bus_ts": "2026-08-19T00:00:00Z",
            "last_bus_error": "rate limited"
        }),
    )
    .expect("card");
    assert!(html.contains("polling"), "{html}");
    assert!(html.contains("2026-08-19T00:00:00Z"), "{html}");
    assert!(html.contains("rate limited"), "{html}");
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
