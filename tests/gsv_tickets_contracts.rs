//! Ticket board + MCP claim contracts (band 168).
//!
//! Sibling `docs/gsv/ticket_claims.jsonl` (not fingerprints). Missing JSONL is
//! empty-ok. Claim requires co-workflow `ticket-claim`. No Telegram create-ticket.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use gsv::boxes::settings;
use gsv::boxes::tickets::{self, ClaimedBy, Ticket};
use gsv::boxes::ui::{render_card, CARD_NAMES};
use gsv::mcp;
use gsv::server::router;
use gsv::AppState;
use serde_json::{json, Value};
use tokio::sync::broadcast;
use tower::ServiceExt;

fn nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

fn temp_kit(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "gsv-tickets-kit-{tag}-{}-{}",
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

fn enable_claim(data: &Path) {
    settings::save(
        data,
        &settings::SettingsFile {
            workflows: settings::Workflows {
                enabled: vec!["ticket-claim".into()],
            },
            ..Default::default()
        },
    )
    .expect("save workflows");
}

fn who() -> ClaimedBy {
    ClaimedBy {
        actor: "agent".into(),
        ide: "cursor".into(),
        model: "grok-4.6".into(),
        agent: "orchestrator".into(),
    }
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
fn jsonl_paths_are_sibling_under_docs_gsv() {
    let kit = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    assert!(tickets::tickets_path(&kit).ends_with(Path::new("docs/gsv/tickets.jsonl")));
    assert!(tickets::claims_path(&kit).ends_with(Path::new("docs/gsv/ticket_claims.jsonl")));
    assert_ne!(tickets::claims_path(&kit), tickets::tickets_path(&kit));
    let fp = kit.join("docs/gsv/fingerprints.jsonl");
    assert_ne!(tickets::claims_path(&kit), fp);
}

#[test]
fn missing_files_are_empty_ok() {
    let kit = temp_kit("missing");
    let w = tickets::list(&kit);
    assert_eq!(w["ok"], true);
    assert_eq!(w["tickets"], json!([]));
    assert!(!w.to_string().contains("bot_token"), "{w}");
}

#[test]
fn create_then_claim_rewrites_ticket_and_appends_claim() {
    let kit = temp_kit("round");
    enable_claim(&kit.join("data"));
    let created = tickets::create(
        &kit,
        "Join Galaxy board",
        "open tickets are the board",
        "gsv",
    )
    .expect("create");
    assert_eq!(created.status, "open");
    assert!(created.claimed_by.is_none());
    assert!(!created.id.is_empty());
    assert!(!created.title.contains("bot_token"));

    let claimed = tickets::claim(&kit, &kit.join("data"), &created.id, who()).expect("claim");
    assert_eq!(claimed.status, "in_progress");
    let by = claimed.claimed_by.expect("claimed_by");
    assert_eq!(by.actor, "agent");
    assert_eq!(by.ide, "cursor");
    assert_eq!(by.model, "grok-4.6");
    assert_eq!(by.agent, "orchestrator");

    let listed = tickets::list(&kit);
    let row = listed["tickets"]
        .as_array()
        .expect("arr")
        .iter()
        .find(|t| t["id"] == created.id)
        .expect("row");
    assert_eq!(row["status"], "in_progress");

    let claims_raw = std::fs::read_to_string(tickets::claims_path(&kit)).expect("claims");
    assert!(claims_raw.contains(&created.id), "{claims_raw}");
    assert!(claims_raw.contains("\"actor\":\"agent\""), "{claims_raw}");
    assert!(!claims_raw.contains("bot_token"), "{claims_raw}");
    let tickets_raw = std::fs::read_to_string(tickets::tickets_path(&kit)).expect("tickets");
    assert!(!tickets_raw.contains("bot_token"), "{tickets_raw}");
}

#[test]
fn claim_unknown_id_is_not_found() {
    let kit = temp_kit("unknown");
    enable_claim(&kit.join("data"));
    let err = tickets::claim(&kit, &kit.join("data"), "no-such", who()).expect_err("nf");
    assert!(err.to_string().contains("unknown"), "{err}");
}

#[test]
fn claim_without_workflow_is_forbidden() {
    let kit = temp_kit("gate");
    let t = tickets::create(&kit, "gated", "", "gsv").expect("create");
    let err = tickets::claim(&kit, &kit.join("data"), &t.id, who()).expect_err("403");
    assert!(err.to_string().contains("ticket-claim"), "{err}");
}

#[test]
fn seed_sample_has_no_secrets() {
    let kit = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let raw = std::fs::read_to_string(tickets::tickets_path(&kit)).expect("seed");
    assert!(!raw.is_empty(), "seed tickets.jsonl");
    assert!(!raw.contains("bot_token"), "{raw}");
    assert!(!raw.to_lowercase().contains("secret"), "{raw}");
    let mut open = 0usize;
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let t: Ticket = serde_json::from_str(line).expect("ticket line");
        if t.status == "open" {
            open += 1;
        }
    }
    assert!(open >= 1, "seed at least one open ticket");
}

#[tokio::test]
async fn get_tickets_missing_is_empty_ok() {
    let kit = temp_kit("http-empty");
    let app = app_kit(kit);
    let (status, json) = get_json(&app, "/api/tickets").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["ok"], true);
    assert_eq!(json["tickets"], json!([]));
    assert!(!json.to_string().contains("bot_token"), "{json}");
}

#[tokio::test]
async fn post_create_and_claim_round_trip() {
    let kit = temp_kit("http-claim");
    enable_claim(&kit.join("data"));
    let app = app_kit(kit);
    let (cstatus, created) = post_json(
        &app,
        "/api/tickets",
        json!({ "title": "MCP claim me", "body": "join copy", "product": "gsv" }),
        Some("http://127.0.0.1:9999"),
        None,
    )
    .await;
    assert_eq!(cstatus, StatusCode::OK, "{created}");
    assert_eq!(created["ok"], true);
    let id = created["ticket"]["id"].as_str().expect("id").to_string();
    assert_eq!(created["ticket"]["status"], "open");

    let (kstatus, claimed) = post_json(
        &app,
        "/api/tickets/claim",
        json!({ "id": id }),
        Some("http://127.0.0.1:9999"),
        None,
    )
    .await;
    assert_eq!(kstatus, StatusCode::OK, "{claimed}");
    assert_eq!(claimed["ok"], true);
    assert_eq!(claimed["ticket"]["status"], "in_progress");
    assert!(claimed["ticket"]["claimed_by"]["actor"].is_string());
    assert!(claimed["ticket"]["claimed_by"]["ide"].is_string());
    assert!(claimed["ticket"]["claimed_by"]["model"].is_string());
    assert!(claimed["ticket"]["claimed_by"]["agent"].is_string());
}

#[tokio::test]
async fn post_claim_unknown_is_404() {
    let kit = temp_kit("http-404");
    enable_claim(&kit.join("data"));
    let app = app_kit(kit);
    let (status, json) = post_json(
        &app,
        "/api/tickets/claim",
        json!({ "id": "missing-id" }),
        Some("http://127.0.0.1:9999"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(json["ok"], false);
}

#[tokio::test]
async fn post_claim_without_workflow_is_403() {
    let kit = temp_kit("http-403");
    let app = app_kit(kit.clone());
    let (cstatus, created) = post_json(
        &app,
        "/api/tickets",
        json!({ "title": "no claim wf" }),
        Some("http://127.0.0.1:9999"),
        None,
    )
    .await;
    assert_eq!(cstatus, StatusCode::OK, "{created}");
    let id = created["ticket"]["id"].as_str().expect("id");
    let (status, json) = post_json(
        &app,
        "/api/tickets/claim",
        json!({ "id": id }),
        Some("http://127.0.0.1:9999"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(json["ok"], false);
}

#[tokio::test]
async fn post_cross_site_is_forbidden() {
    let kit = temp_kit("csrf");
    let app = app_kit(kit);
    let (status, json) = post_json(
        &app,
        "/api/tickets",
        json!({ "title": "csrf" }),
        Some("https://example.com"),
        Some("cross-site"),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(json["ok"], false);
    let (cstatus, cjson) = post_json(
        &app,
        "/api/tickets/claim",
        json!({ "id": "x" }),
        Some("https://example.com"),
        Some("cross-site"),
    )
    .await;
    assert_eq!(cstatus, StatusCode::FORBIDDEN);
    assert_eq!(cjson["ok"], false);
}

#[test]
fn card_tickets_in_registry() {
    assert!(CARD_NAMES.contains(&"tickets"));
    assert_eq!(CARD_NAMES.len(), 40);
    let empty = render_card("tickets", &json!({ "ok": true, "tickets": [] })).expect("empty");
    assert!(empty.contains("tickets — no data"), "{empty}");
    assert!(empty.contains("open tickets are the board"), "{empty}");
    let err = render_card("tickets", &json!({ "ok": false, "error": "down" })).expect("err");
    assert!(err.contains("<span class='err'>down</span>"), "{err}");
    let board = render_card(
        "tickets",
        &json!({
            "ok": true,
            "mode": "squad",
            "online": [{"actor":"a"}],
            "scenarios": [{"id":"join-board","title":"Join","workflow":"ticket-claim"}],
            "tickets": [
                {"id":"t-open","title":"Join","status":"open"},
                {"id":"t-wip","title":"WIP","status":"in_progress"},
                {"id":"t-done","title":"Done","status":"done"},
                {"id":"t-err","title":"Err","status":"blocked"}
            ]
        }),
    )
    .expect("board");
    assert!(board.contains("open"), "{board}");
    assert!(board.contains("in_progress"), "{board}");
    assert!(board.contains("done"), "{board}");
    assert!(board.contains("blocked"), "{board}");
    assert!(board.contains("data-action='tickets-create'"), "{board}");
    assert!(board.contains("data-action='tickets-claim'"), "{board}");
    assert!(board.contains("data-action='tickets-done'"), "{board}");
    assert!(board.contains("data-action='tickets-error'"), "{board}");
    assert!(
        board.contains("data-action='tickets-from-scenario'"),
        "{board}"
    );
    assert!(board.contains("data-action='tickets-presence'"), "{board}");
    assert!(board.contains("squad"), "{board}");
}

#[tokio::test]
async fn mcp_list_and_claim() {
    let kit = temp_kit("mcp");
    enable_claim(&kit.join("data"));
    let created = tickets::create(&kit, "mcp ticket", "", "gsv").expect("create");
    let app = app_kit(kit);

    let list_res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/mcp")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "jsonrpc": "2.0",
                        "id": 1,
                        "method": "tools/call",
                        "params": { "name": "gsv_tickets", "arguments": {} }
                    })
                    .to_string(),
                ))
                .expect("req"),
        )
        .await
        .expect("res");
    let list_bytes = axum::body::to_bytes(list_res.into_body(), usize::MAX)
        .await
        .expect("body");
    let list_json: Value = serde_json::from_slice(&list_bytes).unwrap_or(Value::Null);
    let list_text = list_json["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or("");
    assert_eq!(list_json["result"]["isError"], false, "{list_json}");
    assert!(list_text.contains(&created.id), "{list_text}");

    let claim_res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/mcp")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "jsonrpc": "2.0",
                        "id": 2,
                        "method": "tools/call",
                        "params": { "name": "gsv_tickets_claim", "arguments": { "id": created.id } }
                    })
                    .to_string(),
                ))
                .expect("req"),
        )
        .await
        .expect("res");
    let claim_bytes = axum::body::to_bytes(claim_res.into_body(), usize::MAX)
        .await
        .expect("body");
    let claim_json: Value = serde_json::from_slice(&claim_bytes).unwrap_or(Value::Null);
    assert_eq!(claim_json["result"]["isError"], false, "{claim_json}");
    let claim_text = claim_json["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or("");
    assert!(claim_text.contains("in_progress"), "{claim_text}");
}

#[tokio::test]
async fn mcp_claim_unknown_and_gate_are_tool_errors() {
    let kit = temp_kit("mcp-err");
    let created = tickets::create(&kit, "gated mcp", "", "gsv").expect("create");
    let app = app_kit(kit);

    let unknown = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/mcp")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "jsonrpc": "2.0",
                        "id": 1,
                        "method": "tools/call",
                        "params": { "name": "gsv_tickets_claim", "arguments": { "id": "nope" } }
                    })
                    .to_string(),
                ))
                .expect("req"),
        )
        .await
        .expect("res");
    let ub = axum::body::to_bytes(unknown.into_body(), usize::MAX)
        .await
        .expect("body");
    let uj: Value = serde_json::from_slice(&ub).unwrap_or(Value::Null);
    assert_eq!(uj["result"]["isError"], true, "{uj}");

    let gated = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/mcp")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "jsonrpc": "2.0",
                        "id": 2,
                        "method": "tools/call",
                        "params": { "name": "gsv_tickets_claim", "arguments": { "id": created.id } }
                    })
                    .to_string(),
                ))
                .expect("req"),
        )
        .await
        .expect("res");
    let gb = axum::body::to_bytes(gated.into_body(), usize::MAX)
        .await
        .expect("body");
    let gj: Value = serde_json::from_slice(&gb).unwrap_or(Value::Null);
    assert_eq!(gj["result"]["isError"], true, "{gj}");
}

#[test]
fn unregistered_product_is_rejected() {
    let kit = temp_kit("unreg");
    let err = tickets::create(&kit, "nope", "", "nonesuch").expect_err("unreg");
    assert!(err.to_string().contains("unregistered"), "{err}");
}

#[test]
fn seed_scenarios_have_no_secrets() {
    let kit = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let list = tickets::load_scenarios(&kit);
    assert!(list.len() >= 3, "{}", list.len());
    for sc in &list {
        assert!(!sc.title.contains("bot_token"), "{}", sc.title);
        assert!(!sc.body.contains("bot_token"), "{}", sc.body);
    }
}

fn enable_squad(data: &Path) {
    settings::save(
        data,
        &settings::SettingsFile {
            workflows: settings::Workflows {
                enabled: vec!["ticket-claim".into(), "ticket-squad".into(), "drain".into()],
            },
            tickets: settings::TicketsSettings {
                mode: "squad".into(),
            },
            ..Default::default()
        },
    )
    .expect("save squad");
}

fn write_scenarios(kit: &Path) {
    let raw = r#"{
      "scenarios": [
        {"id":"join-board","title":"Join board","body":"join","workflow":"ticket-claim","product":"gsv"},
        {"id":"squad-dev","title":"Squad dev","body":"squad","workflow":"ticket-squad","product":"gsv"}
      ]
    }"#;
    std::fs::write(tickets::scenarios_path(kit), raw).expect("scenarios");
}

fn who_named(actor: &str) -> ClaimedBy {
    ClaimedBy {
        actor: actor.into(),
        ide: "cursor".into(),
        model: "grok-4.6".into(),
        agent: "worker".into(),
    }
}

#[test]
fn scenario_create_requires_workflow() {
    let kit = temp_kit("sc-off");
    write_scenarios(&kit);
    let err =
        tickets::create_from_scenario(&kit, &kit.join("data"), "join-board", "").expect_err("off");
    assert!(err.to_string().contains("ticket-claim"), "{err}");
}

#[test]
fn scenario_create_then_done_and_error_events() {
    let kit = temp_kit("sc-ok");
    enable_claim(&kit.join("data"));
    write_scenarios(&kit);
    let created =
        tickets::create_from_scenario(&kit, &kit.join("data"), "join-board", "").expect("sc");
    assert_eq!(created.workflow, "ticket-claim");
    assert_eq!(created.status, "open");
    let claimed = tickets::claim(&kit, &kit.join("data"), &created.id, who()).expect("claim");
    assert_eq!(claimed.status, "in_progress");
    let finished = tickets::done(
        &kit,
        &kit.join("data"),
        &created.id,
        who(),
        "all good",
        None,
    )
    .expect("done");
    assert_eq!(finished.status, "done");
    let claims_raw = std::fs::read_to_string(tickets::claims_path(&kit)).expect("claims");
    assert!(claims_raw.contains("\"kind\":\"claimed\""), "{claims_raw}");
    assert!(claims_raw.contains("\"kind\":\"done\""), "{claims_raw}");
    assert!(claims_raw.contains("all good"), "{claims_raw}");

    let other = tickets::create(&kit, "will fail", "", "gsv").expect("other");
    tickets::claim(&kit, &kit.join("data"), &other.id, who()).expect("c2");
    let blocked = tickets::error_ticket(&kit, &kit.join("data"), &other.id, who(), "boom", None)
        .expect("err");
    assert_eq!(blocked.status, "blocked");
    let claims_raw = std::fs::read_to_string(tickets::claims_path(&kit)).expect("claims2");
    assert!(claims_raw.contains("\"kind\":\"error\""), "{claims_raw}");
    assert!(claims_raw.contains("boom"), "{claims_raw}");
}

#[test]
fn solo_picks_one_mcp_squad_picks_by_seed() {
    let kit = temp_kit("mode");
    enable_squad(&kit.join("data"));
    let store = tickets::new_presence_store();
    tickets::heartbeat(&store, &who_named("alpha"));
    tickets::heartbeat(&store, &who_named("beta"));
    let online = tickets::online_now(&store);
    assert_eq!(online.len(), 2);
    assert_eq!(online[0].actor, "alpha");
    assert_eq!(online[1].actor, "beta");

    let open = tickets::create(&kit, "squad me", "", "gsv").expect("create");
    let assigned = tickets::try_dispatch(&kit, &kit.join("data"), &open.id, &store, 1)
        .expect("dispatch")
        .expect("someone");
    assert_eq!(assigned.status, "in_progress");
    let by = assigned.claimed_by.expect("who");
    assert_eq!(by.actor, "beta");

    let kit2 = temp_kit("solo");
    enable_claim(&kit2.join("data"));
    let store2 = tickets::new_presence_store();
    tickets::heartbeat(&store2, &who_named("zeta"));
    tickets::heartbeat(&store2, &who_named("alpha"));
    let open2 = tickets::create(&kit2, "solo me", "", "gsv").expect("c2");
    let assigned2 = tickets::try_dispatch(&kit2, &kit2.join("data"), &open2.id, &store2, 99)
        .expect("d2")
        .expect("one");
    assert_eq!(assigned2.claimed_by.expect("who").actor, "alpha");
}

#[tokio::test]
async fn http_presence_done_and_mcp_create() {
    let kit = temp_kit("http-170");
    enable_claim(&kit.join("data"));
    write_scenarios(&kit);
    let app = app_kit(kit);

    let (pstatus, pjson) = post_json(
        &app,
        "/api/tickets/presence",
        json!({ "actor": "cursor-bot" }),
        Some("http://127.0.0.1:9999"),
        None,
    )
    .await;
    assert_eq!(pstatus, StatusCode::OK, "{pjson}");
    assert_eq!(pjson["ok"], true);
    assert!(!pjson["online"].as_array().expect("arr").is_empty());

    let (cstatus, created) = post_json(
        &app,
        "/api/tickets",
        json!({ "scenario_id": "join-board" }),
        Some("http://127.0.0.1:9999"),
        None,
    )
    .await;
    assert_eq!(cstatus, StatusCode::OK, "{created}");
    let id = created["ticket"]["id"].as_str().expect("id").to_string();
    let st = created["ticket"]["status"].as_str().unwrap_or("");
    assert!(st == "open" || st == "in_progress", "{created}");

    if st == "open" {
        let (kstatus, claimed) = post_json(
            &app,
            "/api/tickets/claim",
            json!({ "id": id }),
            Some("http://127.0.0.1:9999"),
            None,
        )
        .await;
        assert_eq!(kstatus, StatusCode::OK, "{claimed}");
    }

    let (dstatus, done) = post_json(
        &app,
        "/api/tickets/done",
        json!({ "id": id, "note": "shipped" }),
        Some("http://127.0.0.1:9999"),
        None,
    )
    .await;
    assert_eq!(dstatus, StatusCode::OK, "{done}");
    assert_eq!(done["ticket"]["status"], "done");

    let mcp_res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/mcp")
                .method(Method::POST)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "jsonrpc": "2.0",
                        "id": 3,
                        "method": "tools/call",
                        "params": {
                            "name": "gsv_tickets_create",
                            "arguments": { "title": "mcp create", "product": "gsv" }
                        }
                    })
                    .to_string(),
                ))
                .expect("req"),
        )
        .await
        .expect("res");
    let bytes = axum::body::to_bytes(mcp_res.into_body(), usize::MAX)
        .await
        .expect("body");
    let mcp_json: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    assert_eq!(mcp_json["result"]["isError"], false, "{mcp_json}");
}

#[test]
fn mcp_tools_include_tickets_not_bus() {
    assert!(mcp::tool_names().contains(&"gsv_tickets"));
    assert!(mcp::tool_names().contains(&"gsv_tickets_claim"));
    assert!(mcp::tool_names().contains(&"gsv_tickets_create"));
    assert!(mcp::tool_names().contains(&"gsv_tickets_done"));
    assert!(mcp::tool_names().contains(&"gsv_tickets_error"));
    assert!(mcp::tool_names().contains(&"gsv_tickets_presence"));
    assert!(mcp::tool_names().contains(&"gsv_telegram_bus_send"));
    assert!(!mcp::tool_names().contains(&"gsv_telegram_create_ticket"));
    assert_eq!(mcp::tool_names().len(), 46);
}
