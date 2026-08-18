//! GSV session token usage contracts (band 155).
//!
//! Automatic per-session counts from OmniRouter completions, MCP bot sessions,
//! and OmniRoute stats — persisted and mirrored on vision-sync.

use std::path::PathBuf;

use axum::body::Body;
use axum::http::{header, HeaderMap, HeaderValue, Method, Request, StatusCode};
use gsv::boxes::ui::{render_card, CARD_NAMES};
use gsv::boxes::usage::{self, TokenCounts, UsageEvent, UsageStore};
use gsv::boxes::vision;
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
    let dir = std::env::temp_dir().join(format!("gsv-usage-{tag}-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn app_with_data(data: PathBuf) -> axum::Router {
    let (tx, _rx) = broadcast::channel(64);
    let state = AppState::new(Some(kit_root()), Some(data), tx);
    router(state)
}

async fn get_json(app: &axum::Router, path: &str) -> (StatusCode, Value) {
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(path)
                .method(Method::GET)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .expect("body");
    let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, json)
}

fn event(session: &str, source: &str, prompt: u64, completion: u64) -> UsageEvent {
    usage::event_now(
        session,
        source,
        "omniroute",
        "grok-4",
        TokenCounts {
            prompt_tokens: prompt,
            completion_tokens: completion,
            cache_read_tokens: 0,
            reasoning_tokens: 0,
        },
    )
}

#[test]
fn parse_openai_usage() {
    let v = json!({
        "usage": { "prompt_tokens": 11, "completion_tokens": 22, "total_tokens": 33 }
    });
    let c = usage::parse_usage(&v).expect("usage");
    assert_eq!(c.prompt_tokens, 11);
    assert_eq!(c.completion_tokens, 22);
    assert_eq!(c.total(), 33);
}

#[test]
fn parse_omniroute_and_anthropic_aliases() {
    let or = json!({ "usage": { "input_tokens": 4, "output_tokens": 6 } });
    let c = usage::parse_usage(&or).expect("or");
    assert_eq!(c.prompt_tokens, 4);
    assert_eq!(c.completion_tokens, 6);
    assert_eq!(c.total(), 10);

    let gem = json!({
        "usageMetadata": { "promptTokenCount": 2, "candidatesTokenCount": 3 }
    });
    let c = usage::parse_usage(&gem).expect("gem");
    assert_eq!(c.prompt_tokens, 2);
    assert_eq!(c.completion_tokens, 3);
}

#[test]
fn parse_usage_skips_dry_run_and_empty() {
    assert!(usage::parse_usage(&json!({ "dry_run": true, "provider": "openai" })).is_none());
    assert!(usage::parse_usage(&json!({ "ok": true })).is_none());
    assert!(usage::parse_usage(&json!({ "usage": {} })).is_none());
}

#[test]
fn record_aggregates_per_session() {
    let mut store = UsageStore::default();
    usage::record(&mut store, event("mcp:abc-session", "mcp", 10, 20));
    usage::record(&mut store, event("mcp:abc-session", "mcp", 1, 2));
    usage::record(&mut store, event("process", "omni", 5, 5));
    assert_eq!(store.sessions.len(), 2);
    let mcp = store.sessions.get("mcp:abc-session").expect("mcp row");
    assert_eq!(mcp.requests, 2);
    assert_eq!(mcp.prompt_tokens, 11);
    assert_eq!(mcp.completion_tokens, 22);
    assert_eq!(mcp.total_tokens, 33);
    let process = usage::process_totals(&store);
    assert_eq!(process.requests, 3);
    assert_eq!(process.total_tokens, 43);
}

#[test]
fn record_skips_zero_and_caps_events() {
    let mut store = UsageStore::default();
    usage::record(&mut store, event("process", "omni", 0, 0));
    assert!(store.events.is_empty());
    for i in 0..(usage::EVENT_CAP + 8) {
        usage::record(&mut store, event("process", "omni", 1, i as u64));
    }
    assert_eq!(store.events.len(), usage::EVENT_CAP);
}

#[test]
fn persist_roundtrip() {
    let data = temp_data("roundtrip");
    let mut store = UsageStore::default();
    usage::record(&mut store, event("stdio", "mcp", 7, 8));
    usage::save(&data, &store).expect("save");
    let loaded = usage::load(&data);
    assert_eq!(loaded.sessions["stdio"].prompt_tokens, 7);
    assert_eq!(loaded.sessions["stdio"].completion_tokens, 8);
    assert!(usage::store_path(&data)
        .to_string_lossy()
        .replace('\\', "/")
        .ends_with("gsv_usage.json"));
}

#[test]
fn session_from_mcp_and_gsv_headers() {
    let mut headers = HeaderMap::new();
    headers.insert(
        mcp::MCP_SESSION_HEADER,
        HeaderValue::from_static("mcp-sess-0001"),
    );
    assert_eq!(usage::session_from_headers(&headers), "mcp:mcp-sess-0001");
    assert_eq!(usage::source_from_headers(&headers), "mcp");

    let mut headers = HeaderMap::new();
    headers.insert("x-gsv-session", HeaderValue::from_static("stdio"));
    headers.insert("x-gsv-source", HeaderValue::from_static("mcp"));
    assert_eq!(usage::session_from_headers(&headers), "stdio");
    assert_eq!(usage::source_from_headers(&headers), "mcp");

    let headers = HeaderMap::new();
    assert_eq!(usage::session_from_headers(&headers), "process");
    assert_eq!(usage::source_from_headers(&headers), "omni");
}

#[test]
fn parse_omniroute_stats_totals() {
    let v = json!({
        "totalRequests": 9,
        "totalPromptTokens": 100,
        "totalCompletionTokens": 40
    });
    let pull = usage::parse_omniroute_stats(&v);
    assert!(pull.ok);
    assert_eq!(pull.requests, 9);
    assert_eq!(pull.prompt_tokens, 100);
    assert_eq!(pull.completion_tokens, 40);
    assert_eq!(pull.total_tokens, 140);

    let empty = usage::parse_omniroute_stats(&json!({ "error": "auth" }));
    assert!(!empty.ok);
    assert_eq!(empty.requests, 0);
}

#[test]
fn omniroute_history_url_is_loopback() {
    assert_eq!(
        usage::omniroute_history_url(usage::DEFAULT_OMNIROUTE_URL),
        "http://127.0.0.1:20128/api/usage/history"
    );
}

#[test]
fn vision_sync_writes_usage_snapshot() {
    let src = kit_root();
    let data = temp_data("sync");
    let report = vision::sync(&src, &data).expect("sync");
    assert!(
        report
            .usage_target
            .replace('\\', "/")
            .ends_with("gsv_usage.json"),
        "{}",
        report.usage_target
    );
    assert!(usage::store_path(&data).is_file());
}

#[tokio::test]
async fn api_usage_ok_and_card() {
    let data = temp_data("api");
    let mut store = UsageStore::default();
    usage::record(&mut store, event("process", "omni", 3, 4));
    usage::save(&data, &store).expect("save");
    let app = app_with_data(data);
    let (status, json) = get_json(&app, "/api/usage").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["ok"], true);
    assert_eq!(json["process"]["prompt_tokens"], 3);
    assert_eq!(json["process"]["completion_tokens"], 4);
    assert!(json["omniroute"].is_object(), "{json}");
    assert!(json["path"]
        .as_str()
        .unwrap_or("")
        .contains("gsv_usage.json"));
}

#[tokio::test]
async fn dry_run_omni_chat_does_not_count() {
    let data = temp_data("dry");
    let app = app_with_data(data.clone());
    let body = json!({ "model": "gpt-5.2", "messages": [] });
    let res = app
        .oneshot(
            Request::post("/api/omni/v1/chat/completions")
                .header(header::CONTENT_TYPE, "application/json")
                .header("x-omni-dry-run", "1")
                .body(Body::from(body.to_string()))
                .expect("req"),
        )
        .await
        .expect("resp");
    assert_eq!(res.status(), StatusCode::OK);
    let app = app_with_data(data);
    let (_, json) = get_json(&app, "/api/usage").await;
    assert_eq!(json["process"]["requests"], 0);
    assert_eq!(json["process"]["total_tokens"], 0);
}

#[test]
fn render_usage_lists_sessions_and_omniroute() {
    assert!(CARD_NAMES.contains(&"usage"));
    let html = render_card(
        "usage",
        &json!({
            "ok": true,
            "process": {
                "requests": 2,
                "prompt_tokens": 10,
                "completion_tokens": 20,
                "total_tokens": 30
            },
            "sessions": [{
                "session": "mcp:abc",
                "source": "mcp",
                "requests": 1,
                "prompt_tokens": 10,
                "completion_tokens": 20,
                "total_tokens": 30
            }],
            "omniroute": {
                "ok": true,
                "base_url": "http://127.0.0.1:20128",
                "requests": 9,
                "prompt_tokens": 100,
                "completion_tokens": 40,
                "total_tokens": 140
            }
        }),
    )
    .expect("card");
    assert!(html.contains("mcp:abc"), "{html}");
    assert!(html.contains("omniroute"), "{html}");
    assert!(html.contains("30"), "{html}");
}

#[test]
fn render_usage_empty_and_error() {
    let empty = render_card("usage", &json!({ "ok": true, "sessions": [] })).expect("empty");
    assert!(empty.contains("usage — no data"), "{empty}");
    let err = render_card("usage", &json!({ "ok": false, "error": "down" })).expect("err");
    assert!(err.contains("<span class='err'>down</span>"), "{err}");
}

#[tokio::test]
async fn mcp_usage_tool_ok() {
    let data = temp_data("mcp-tool");
    let mut store = UsageStore::default();
    usage::record(&mut store, event("stdio", "mcp", 1, 1));
    usage::save(&data, &store).expect("save");
    let app = app_with_data(data);
    let res = app
        .oneshot(
            Request::post("/mcp")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "jsonrpc": "2.0",
                        "id": 1,
                        "method": "tools/call",
                        "params": { "name": "gsv_usage", "arguments": {} }
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
    let json: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    assert_eq!(json["result"]["isError"], false);
    let text = json["result"]["content"][0]["text"].as_str().unwrap_or("");
    assert!(
        text.contains("stdio") || text.contains("prompt_tokens"),
        "{text}"
    );
}

#[test]
fn data_files_allow_usage_snapshot() {
    assert!(gsv::security::DATA_FILES.contains(&"gsv_usage.json"));
}

#[test]
fn parse_sse_usage_from_openai_stream() {
    let body = "data: {\"choices\":[{\"delta\":{\"content\":\"a\"}}]}\n\
data: {\"usage\":{\"prompt_tokens\":4,\"completion_tokens\":5}}\n\
data: [DONE]\n";
    let c = usage::parse_sse_usage(body).expect("sse usage");
    assert_eq!(c.prompt_tokens, 4);
    assert_eq!(c.completion_tokens, 5);
}

#[test]
fn stream_body_requests_include_usage() {
    let mut body = json!({ "stream": true, "messages": [] });
    usage::ensure_stream_include_usage(&mut body);
    assert_eq!(body["stream_options"]["include_usage"], true);
}
