//! Godfather Telegram channel bind contracts (band 167).
//!
//! Dry-run / `X-Telegram-Dry-Run: 1` never opens sockets. Missing channel or
//! token → `{ok:false}` without secrets. No bus tools, no `tickets.jsonl`.

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
    assert_eq!(CARD_NAMES.len(), 39);
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
    assert!(!mcp::tool_names().contains(&"gsv_telegram_bus_send"));
    assert!(!mcp::tool_names().contains(&"gsv_telegram_bus_poll"));
}

#[test]
fn no_tickets_jsonl_in_band_167() {
    let path = kit_root().join("docs/gsv/tickets.jsonl");
    assert!(
        !path.exists(),
        "tickets.jsonl is band 168 — must not exist in 167"
    );
}
