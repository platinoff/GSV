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

#[test]
fn done_remote_closes_wip_row_without_ranks() {
    let kit = temp_kit("done-remote");
    let data = kit.join("data");
    enable_claim(&data);
    let t = tickets::create(&kit, "Remote close", "body", "gsv").expect("create");
    tickets::claim_with(&kit, &data, &t.id, who(), None).expect("claim");

    let remote = ClaimedBy {
        actor: "alice".into(),
        ide: "opencode".into(),
        model: String::new(),
        agent: "bot".into(),
    };
    let closed = tickets::done_remote(&kit, &data, &t.id, remote, "band closed").expect("close");
    assert_eq!(closed.status, "done");

    let listed = tickets::list(&kit);
    let row = listed["tickets"]
        .as_array()
        .expect("arr")
        .iter()
        .find(|x| x["id"] == t.id)
        .expect("row");
    assert_eq!(row["status"], "done");
    assert!(!kit.join("data/gsv_ranks.json").exists(), "ranks moved");

    assert!(tickets::done_remote(&kit, &data, &t.id, who(), "again").is_err());
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
    assert_eq!(json["next"]["hint"], "idle");
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
    assert_eq!(CARD_NAMES.len(), 42);
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
    assert!(board.contains("data-action='tickets-walk'"), "{board}");
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
        for step in &sc.tickets {
            assert!(!step.title.contains("bot_token"), "{}", step.title);
            assert!(!step.body.contains("bot_token"), "{}", step.body);
        }
    }
    assert!(
        list.iter()
            .any(|s| s.id == "memory-disk-speed" && s.tickets.len() >= 6),
        "mds scenario band missing"
    );
    assert!(
        list.iter()
            .any(|s| s.id == "abrakadabra-session" && s.tickets.len() >= 6),
        "abrakadabra session scenario band missing"
    );
    assert!(
        list.iter().any(|s| s.id == "federated-claim"),
        "federated-claim scenario missing"
    );
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
                ..Default::default()
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
    assert!(mcp::tool_names().contains(&"gsv_tickets_reclaim"));
    assert!(mcp::tool_names().contains(&"gsv_tickets_walk"));
    assert!(mcp::tool_names().contains(&"gsv_tickets_hook"));
    assert!(mcp::tool_names().contains(&"gsv_tickets_bench"));
    assert!(mcp::tool_names().contains(&"gsv_tickets_next"));
    assert!(mcp::tool_names().contains(&"gsv_ranks"));
    assert!(mcp::tool_names().contains(&"gsv_mds"));
    assert!(mcp::tool_names().contains(&"gsv_telegram_bus_send"));
    assert!(mcp::tool_names().contains(&"gsv_telegram_ticket"));
    assert!(mcp::tool_names().contains(&"gsv_telegram_poll"));
    assert!(mcp::tool_names().contains(&"gsv_telegram_decode"));
    assert!(!mcp::tool_names().contains(&"gsv_telegram_create_ticket"));
    assert_eq!(mcp::tool_names().len(), 56);
}

fn write_stale_wip(kit: &Path, id: &str, actor: &str, lease_until: u64) {
    let row = json!({
        "id": id,
        "ts": "2026-08-19T00:00:00Z",
        "title": "stale lease",
        "body": "",
        "status": "in_progress",
        "product": "gsv",
        "claimed_by": {
            "actor": actor,
            "ide": "cursor",
            "model": "grok-4.6",
            "agent": "worker"
        },
        "lease_until": lease_until
    });
    std::fs::write(tickets::tickets_path(kit), format!("{row}\n")).expect("stale jsonl");
}

#[test]
fn claim_sets_default_lease() {
    let kit = temp_kit("lease-set");
    enable_claim(&kit.join("data"));
    let created = tickets::create(&kit, "lease me", "", "gsv").expect("create");
    let claimed = tickets::claim(&kit, &kit.join("data"), &created.id, who()).expect("claim");
    let until = claimed.lease_until.expect("lease_until");
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    assert!(
        until >= now + tickets::DEFAULT_LEASE_SECS - 2,
        "until={until} now={now}"
    );
    assert!(
        until <= now + tickets::DEFAULT_LEASE_SECS + 2,
        "until={until} now={now}"
    );
}

#[test]
fn stale_in_progress_reclaims_to_open_on_list() {
    let kit = temp_kit("lease-expire");
    enable_claim(&kit.join("data"));
    write_stale_wip(&kit, "t-stale", "agent", 1);
    let store = tickets::new_presence_store();
    let listed = tickets::wire_list(&kit, &kit.join("data"), &store);
    let row = listed["tickets"]
        .as_array()
        .expect("arr")
        .iter()
        .find(|t| t["id"] == "t-stale")
        .expect("row");
    assert_eq!(row["status"], "open");
    assert!(row["claimed_by"].is_null() || row.get("claimed_by").is_none());
    assert!(row.get("lease_until").is_none() || row["lease_until"].is_null());
    let claims_raw = std::fs::read_to_string(tickets::claims_path(&kit)).expect("claims");
    assert!(
        claims_raw.contains("\"kind\":\"reclaimed\""),
        "{claims_raw}"
    );
}

#[test]
fn unexpired_lease_stays_in_progress() {
    let kit = temp_kit("lease-fresh");
    enable_claim(&kit.join("data"));
    let far = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
        + 10_000;
    write_stale_wip(&kit, "t-fresh", "agent", far);
    let store = tickets::new_presence_store();
    let listed = tickets::wire_list(&kit, &kit.join("data"), &store);
    let row = listed["tickets"]
        .as_array()
        .expect("arr")
        .iter()
        .find(|t| t["id"] == "t-fresh")
        .expect("row");
    assert_eq!(row["status"], "in_progress");
}

#[test]
fn presence_renews_holder_lease() {
    let kit = temp_kit("lease-renew");
    enable_claim(&kit.join("data"));
    write_stale_wip(&kit, "t-hold", "agent", 50);
    let store = tickets::new_presence_store();
    let listed = tickets::wire_presence(
        &kit,
        &kit.join("data"),
        &store,
        &json!({ "actor": "agent", "ide": "cursor", "agent": "worker" }),
    );
    assert_eq!(listed["ok"], true);
    let tickets = tickets::list(&kit);
    let row = tickets["tickets"]
        .as_array()
        .expect("arr")
        .iter()
        .find(|t| t["id"] == "t-hold")
        .expect("row");
    assert_eq!(row["status"], "in_progress");
    let until = row["lease_until"].as_u64().expect("lease");
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    assert!(until >= now + tickets::DEFAULT_LEASE_SECS - 2, "{until}");
}

#[tokio::test]
async fn http_and_mcp_reclaim_stale() {
    let kit = temp_kit("lease-http");
    enable_claim(&kit.join("data"));
    write_stale_wip(&kit, "t-http", "agent", 1);
    let app = app_kit(kit);

    let (status, json) = post_json(
        &app,
        "/api/tickets/reclaim",
        json!({ "id": "t-http" }),
        Some("http://127.0.0.1:9999"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert_eq!(json["ok"], true);
    assert_eq!(json["ticket"]["status"], "open");

    let (cstatus, csrf) = post_json(
        &app,
        "/api/tickets/reclaim",
        json!({ "id": "t-http" }),
        Some("https://example.com"),
        Some("cross-site"),
    )
    .await;
    assert_eq!(cstatus, StatusCode::FORBIDDEN);
    assert_eq!(csrf["ok"], false);

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
                        "id": 9,
                        "method": "tools/call",
                        "params": { "name": "gsv_tickets_reclaim", "arguments": {} }
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
fn card_shows_lease_and_reclaim() {
    let board = render_card(
        "tickets",
        &json!({
            "ok": true,
            "lease_secs": 300,
            "tickets": [
                {"id":"t-wip","title":"WIP","status":"in_progress","lease_until": 9_999_999_999u64}
            ]
        }),
    )
    .expect("board");
    assert!(board.contains("lease"), "{board}");
    assert!(board.contains("data-action='tickets-reclaim'"), "{board}");
}

#[test]
fn galaxy_glue_defines_reclaim_ticket() {
    assert!(
        gsv::server::INDEX_HTML.contains("async function reclaimTicket"),
        "tickets-reclaim button must have glue"
    );
}

#[test]
fn galaxy_glue_walk_refreshes_telegram_and_vision_sync_is_data_action() {
    let html = gsv::server::INDEX_HTML;
    assert!(
        html.contains("async function syncVision"),
        "vision remirror must be one glue function"
    );
    assert!(
        html.contains("data-action=\"vision-sync\""),
        "vision-sync button must use data-action"
    );
    assert!(
        !html.contains("onclick=\"resyncVision()\""),
        "vision-sync must not duplicate onclick"
    );
    assert!(
        html.contains("await getText(\"telegram\")"),
        "walk/hook/bench must refresh the Telegram MCP signal"
    );
    assert!(
        html.contains("async function nextTicket"),
        "tickets-next button must have glue"
    );
    assert!(
        html.contains("data-action='tickets-next'")
            || html.contains("data-action=\"tickets-next\"")
            || html.contains("tickets-next"),
        "click router must dispatch tickets-next"
    );
}

fn enable_claim_relay(data: &Path) {
    settings::save(
        data,
        &settings::SettingsFile {
            workflows: settings::Workflows {
                enabled: vec!["ticket-claim".into(), "telegram-relay".into()],
            },
            ..Default::default()
        },
    )
    .expect("save claim+relay");
}

fn write_mds_band(kit: &Path) {
    let raw = r#"{
      "scenarios": [
        {
          "id": "memory-disk-speed",
          "title": "Light Rust MDS app",
          "body": "band",
          "workflow": "ticket-claim",
          "product": "gsv",
          "tickets": [
            {"title": "MDS: scaffold", "body": "bin"},
            {"title": "MDS: memory", "body": "probe"},
            {"title": "MDS: disk", "body": "probe"}
          ]
        }
      ]
    }"#;
    std::fs::write(tickets::scenarios_path(kit), raw).expect("mds scenarios");
}

#[test]
fn scenario_band_creates_all_rows_open() {
    let kit = temp_kit("mds-band");
    enable_claim(&kit.join("data"));
    write_mds_band(&kit);
    let band = tickets::create_band_from_scenario(&kit, &kit.join("data"), "memory-disk-speed", "")
        .expect("band");
    assert_eq!(band.len(), 3, "{}", band.len());
    assert!(band.iter().all(|t| t.status == "open"));
    assert!(band.iter().all(|t| t.scenario == "memory-disk-speed"));
    let listed = tickets::list(&kit);
    let n = listed["tickets"].as_array().expect("arr").len();
    assert_eq!(n, 3, "{listed}");
}

#[test]
fn solo_walk_claims_and_dones_band() {
    let kit = temp_kit("mds-walk");
    enable_claim_relay(&kit.join("data"));
    write_mds_band(&kit);
    let _ = tickets::create_band_from_scenario(&kit, &kit.join("data"), "memory-disk-speed", "")
        .expect("band");
    let store = tickets::new_presence_store();
    let report = tickets::solo_walk(
        &kit,
        &kit.join("data"),
        Some(&store),
        who(),
        "memory-disk-speed",
    )
    .expect("walk");
    assert!(report.ok);
    assert_eq!(report.walked.len(), 6, "3 tickets × claimed+done");
    assert!(report.walked.iter().any(|s| s.phase == "claimed"));
    assert!(report.walked.iter().any(|s| s.phase == "done"));
    let listed = tickets::list(&kit);
    for t in listed["tickets"].as_array().expect("arr") {
        assert_eq!(t["status"], "done", "{t}");
    }
}

#[tokio::test]
async fn http_walk_enqueues_telegram_sync() {
    let kit = temp_kit("http-walk");
    enable_claim_relay(&kit.join("data"));
    write_mds_band(&kit);
    let app = app_kit(kit);
    gsv::boxes::telegram::bus_reset();
    let (status, json) = post_json(
        &app,
        "/api/tickets/walk",
        json!({ "scenario_id": "memory-disk-speed", "from": "solo-bot" }),
        Some("http://127.0.0.1:9999"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert_eq!(json["ok"], true, "{json}");
    assert_eq!(json["walked"].as_array().expect("w").len(), 6, "{json}");
    assert_eq!(json["telegram"], 7, "6 steps + bench {json}");
    assert!(
        json["bench"]
            .as_str()
            .unwrap_or("")
            .starts_with("bench gsv_dev "),
        "{json}"
    );
    let (pstatus, pjson) = get_json(&app, "/api/telegram/bus?limit=32").await;
    assert_eq!(pstatus, StatusCode::OK, "{pjson}");
    let msgs = pjson["messages"].as_array().cloned().unwrap_or_default();
    assert!(
        msgs.iter().any(|m| m["body"]
            .as_str()
            .unwrap_or("")
            .starts_with("solo claimed ")),
        "{pjson}"
    );
    assert!(
        msgs.iter()
            .any(|m| m["body"].as_str().unwrap_or("").starts_with("solo done ")),
        "{pjson}"
    );
    assert!(
        msgs.iter()
            .any(|m| m["body"].as_str().unwrap_or("").contains("bench gsv_dev")),
        "{pjson}"
    );
}

fn who_b() -> ClaimedBy {
    ClaimedBy {
        actor: "opencode".into(),
        ide: "opencode".into(),
        model: "grok-4.6".into(),
        agent: "worker".into(),
    }
}

#[test]
fn squad_walk_assigns_among_two_online() {
    let kit = temp_kit("squad-walk");
    enable_squad(&kit.join("data"));
    write_mds_band(&kit);
    let _ = tickets::create_band_from_scenario(&kit, &kit.join("data"), "memory-disk-speed", "")
        .expect("band");
    let store = tickets::new_presence_store();
    let _ = tickets::heartbeat(&store, &who());
    let _ = tickets::heartbeat(&store, &who_b());
    let report = tickets::solo_walk(
        &kit,
        &kit.join("data"),
        Some(&store),
        who(),
        "memory-disk-speed",
    )
    .expect("walk");
    assert!(
        report.walked.iter().any(|s| s.phase == "assigned"),
        "{:?}",
        report.walked
    );
    assert!(report.walked.iter().any(|s| s.kind == "squad"));
    let actors: Vec<&str> = report
        .walked
        .iter()
        .filter(|s| s.phase == "assigned")
        .map(|s| s.actor.as_str())
        .collect();
    assert!(
        actors.contains(&"agent") && actors.contains(&"opencode"),
        "{actors:?}"
    );
}

fn write_roadmap_and_plan(kit: &Path) {
    let _ = std::fs::create_dir_all(kit.join("docs/gsv"));
    let _ = std::fs::create_dir_all(kit.join("docs/superpowers/plans"));
    std::fs::write(
        tickets::roadmap_path(kit),
        r#"## Спринти (band 177) — hook up

| Sprint | Фокус | Acceptance |
| **PH-S2409** | Scope | owner pick — **[ ]** |
| **PH-S2410** | Parse | phrase grammar — **[ ]** |
| **PH-S2411** | Roadmap | PH-S* rows — **[ ]** |
"#,
    )
    .expect("roadmap");
    std::fs::write(
        tickets::plans_dir(kit).join("hook-demo.md"),
        "- [x] already done\n- [ ] Plan: place tickets\n- [ ] Plan: Telegram sync\n",
    )
    .expect("plan");
}

#[test]
fn hook_up_catalog_is_idempotent() {
    let kit = temp_kit("hook-catalog");
    enable_claim(&kit.join("data"));
    write_mds_band(&kit);
    let first =
        tickets::hook_up(&kit, &kit.join("data"), "scenario", "memory-disk-speed").expect("hook");
    assert_eq!(first.tickets.len(), 3);
    assert_eq!(first.scenario, "memory-disk-speed");
    let second =
        tickets::hook_up(&kit, &kit.join("data"), "scenario", "memory-disk-speed").expect("again");
    assert_eq!(second.tickets.len(), 3);
    assert!(second.skipped >= 3, "{}", second.skipped);
}

#[test]
fn hook_up_band_from_roadmap() {
    let kit = temp_kit("hook-band");
    enable_claim(&kit.join("data"));
    write_roadmap_and_plan(&kit);
    let report = tickets::hook_up(&kit, &kit.join("data"), "band", "177").expect("band");
    assert_eq!(report.source, "band");
    assert_eq!(report.scenario, "roadmap-band-177");
    assert_eq!(report.tickets.len(), 3);
    assert!(report.tickets[0].title.starts_with("PH-S2409"));
}

#[test]
fn hook_up_plan_open_checkboxes_only() {
    let kit = temp_kit("hook-plan");
    enable_claim(&kit.join("data"));
    write_roadmap_and_plan(&kit);
    let report = tickets::hook_up(&kit, &kit.join("data"), "plan", "hook-demo").expect("plan");
    assert_eq!(report.tickets.len(), 2, "{:?}", report.tickets);
    assert!(report
        .tickets
        .iter()
        .any(|t| t.title.contains("place tickets")));
}

#[test]
fn parse_owner_phrase() {
    let p = tickets::parse_hook_phrase("run mcp bot hook up scenario band 177 walk").expect("p");
    assert_eq!(p.source, "band");
    assert_eq!(p.id, "177");
    assert!(p.walk);
}

#[tokio::test]
async fn http_hook_phrase_places_catalog_and_syncs() {
    let kit = temp_kit("http-hook");
    enable_claim_relay(&kit.join("data"));
    write_mds_band(&kit);
    let app = app_kit(kit);
    gsv::boxes::telegram::bus_reset();
    let (status, json) = post_json(
        &app,
        "/api/tickets/hook",
        json!({
            "phrase": "run mcp bot hook up scenario memory-disk-speed",
            "from": "solo-bot"
        }),
        Some("http://127.0.0.1:9999"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert_eq!(json["ok"], true, "{json}");
    assert_eq!(json["tickets"].as_array().expect("t").len(), 3, "{json}");
    assert_eq!(json["source"], "scenario");
    let (pstatus, pjson) = get_json(&app, "/api/telegram/bus?limit=32").await;
    assert_eq!(pstatus, StatusCode::OK, "{pjson}");
    let msgs = pjson["messages"].as_array().cloned().unwrap_or_default();
    assert!(
        msgs.iter().any(|m| m["body"]
            .as_str()
            .unwrap_or("")
            .starts_with("hook scenario")),
        "{pjson}"
    );
}

#[tokio::test]
async fn http_tickets_bench_get_empty_ok_and_post_runs() {
    let kit = temp_kit("http-bench");
    enable_claim(&kit.join("data"));
    std::fs::write(
        tickets::scenarios_path(&kit),
        r#"{
          "scenarios": [{
            "id": "abrakadabra-session",
            "title": "session",
            "body": "bench",
            "workflow": "ticket-claim",
            "product": "gsv",
            "tickets": [
              {"title": "Session: S0", "body": "a"},
              {"title": "Session: close", "body": "c"}
            ]
          }]
        }"#,
    )
    .expect("session catalog");
    let app = app_kit(kit);
    let (gstatus, gjson) = get_json(&app, "/api/tickets/bench").await;
    assert_eq!(gstatus, StatusCode::OK, "{gjson}");
    assert_eq!(gjson["ok"], true, "{gjson}");
    assert_eq!(gjson["recorded"], false, "{gjson}");
    let (pstatus, pjson) = post_json(
        &app,
        "/api/tickets/bench",
        json!({ "run": true }),
        Some("http://127.0.0.1:9999"),
        None,
    )
    .await;
    assert_eq!(pstatus, StatusCode::OK, "{pjson}");
    assert_eq!(pjson["ok"], true, "{pjson}");
    assert_eq!(pjson["recorded"], true, "{pjson}");
    assert!(
        pjson["session_walk_ns"].as_u64().unwrap_or(0) > 0,
        "{pjson}"
    );
    assert!(
        pjson["line"].as_str().unwrap_or("").contains("session="),
        "{pjson}"
    );
}

#[tokio::test]
async fn next_action_http_and_mcp() {
    let kit = temp_kit("http-next");
    enable_claim(&kit.join("data"));
    let created = tickets::create(&kit, "PH-S2469 next row", "", "gsv").expect("create");
    let app = app_kit(kit);
    let (gstatus, gjson) = get_json(&app, "/api/tickets").await;
    assert_eq!(gstatus, StatusCode::OK, "{gjson}");
    assert_eq!(gjson["next"]["hint"], "claim-next", "{gjson}");
    assert_eq!(gjson["next"]["tool"], "gsv_tickets_claim", "{gjson}");
    assert_eq!(gjson["next"]["ticket_id"], created.id, "{gjson}");
    let (pstatus, pjson) = post_json(
        &app,
        "/api/tickets/next",
        json!({ "hint": "claim-next" }),
        Some("http://127.0.0.1:9999"),
        None,
    )
    .await;
    assert_eq!(pstatus, StatusCode::OK, "{pjson}");
    assert_eq!(pjson["ok"], true, "{pjson}");
    assert_eq!(pjson["hint"], "claim-next", "{pjson}");
    assert_eq!(pjson["tool"], "gsv_tickets_claim", "{pjson}");
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
                        "id": 11,
                        "method": "tools/call",
                        "params": {
                            "name": "gsv_tickets_next",
                            "arguments": { "hint": "claim-next" }
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
    let text = mcp_json["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or("");
    assert!(text.contains("claim-next"), "{text}");
    assert!(text.contains("gsv_tickets_claim"), "{text}");
}

#[test]
fn join_env_and_presence_honor_squad_cap() {
    let kit = temp_kit("jail-cap");
    let data = kit.join("data");
    settings::save(
        &data,
        &settings::SettingsFile {
            jail: settings::JailSettings { id: "alice".into() },
            tickets: settings::TicketsSettings {
                mode: "squad".into(),
                squad_cap: 1,
                member_count: 1,
                chat_kind: "channel".into(),
                ..Default::default()
            },
            workflows: settings::Workflows {
                enabled: vec!["ticket-claim".into(), "ticket-squad".into()],
            },
            ..Default::default()
        },
    )
    .expect("save");
    let store = tickets::new_presence_store();
    let env = tickets::join_env(&kit, &data, &store);
    assert_eq!(env["jail_id"], "alice");
    assert_eq!(env["squad_cap"], 1);
    assert_eq!(env["bot_slot_cap"], settings::TG_CHANNEL_ADMINS_MAX);
    assert_eq!(env["loopback_mcp"], "http://127.0.0.1:9999/mcp");
    assert_eq!(env["chat_role"], "local");
    assert!(
        env["hint"]
            .as_str()
            .unwrap_or("")
            .contains("Telegram token"),
        "{env}"
    );
    assert!(!env.to_string().contains("bot_token"), "{env}");
    let first = tickets::wire_presence(
        &kit,
        &data,
        &store,
        &json!({ "actor": "a", "ide": "cursor", "agent": "one" }),
    );
    assert_eq!(first["ok"], true);
    assert_eq!(first["accepted"], true);
    let second = tickets::wire_presence(
        &kit,
        &data,
        &store,
        &json!({ "actor": "b", "ide": "cursor", "agent": "two" }),
    );
    assert_eq!(second["ok"], true);
    assert_eq!(second["accepted"], false);
    assert_eq!(second["error"], "squad full");
    let listed = tickets::wire_list(&kit, &data, &store);
    assert_eq!(listed["jail_id"], "alice");
    assert_eq!(listed["env"]["squad_full"], true);
    let nxt = tickets::wire_next(
        &kit,
        &data,
        &store,
        &json!({ "actor": "c", "ide": "cursor", "agent": "three" }),
        "",
        "",
    );
    assert_eq!(nxt["accepted"], false);
    assert_eq!(nxt["online"], 1);
}

#[test]
fn galaxy_glue_saves_jail_and_squad_cap() {
    assert!(
        gsv::server::INDEX_HTML.contains("setJail"),
        "settings jail input glue"
    );
    assert!(
        gsv::server::INDEX_HTML.contains("setSquadCap"),
        "settings squad cap glue"
    );
    assert!(
        gsv::server::INDEX_HTML.contains("setMemberCount"),
        "settings member count glue"
    );
    assert!(
        gsv::server::INDEX_HTML.contains("setMode"),
        "settings mode glue"
    );
    assert!(
        gsv::server::INDEX_HTML.contains("setRole"),
        "settings channel role glue"
    );
    assert!(
        gsv::server::INDEX_HTML.contains("data-hook-source"),
        "Galaxy hook GitHub glue"
    );
    assert!(
        gsv::server::INDEX_HTML.contains("setWf"),
        "settings workflow chips glue"
    );
    assert!(
        gsv::server::INDEX_HTML.contains("color-scheme:dark"),
        "dark form controls"
    );
}
