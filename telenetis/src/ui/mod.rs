use crate::state::AppState;
use axum::{
    extract::{Query, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::json;

pub mod miniapp;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(dashboard))
        .route("/app", get(app_page))
        .route("/board", get(board_page))
        .route("/flows", get(flows_page))
        .route("/roles", get(roles_page))
        .route("/health", get(health))
        .route("/api/status", get(status))
        .route("/api/tickets", get(api_tickets))
        .route("/api/flows", get(api_flows))
        .route("/static/app.css", get(serve_css))
        .route("/static/app.js", get(serve_js))
        .route("/api/verify", get(api_verify_init_data))
        .route("/api/mini-app/i18n", get(api_mini_app_i18n))
        .route("/api/live/config", get(api_live_config))
        .route("/api/snapshot", get(api_snapshot))
        .route("/api/board/claim", post(api_board_claim))
        .route("/api/board/done", post(api_board_done))
        .route("/api/board/error", post(api_board_error))
        .route("/api/board/reclaim", post(api_board_reclaim))
        .with_state(state)
}

/// Resolve the Mini App UI strings for a requested language. The client asks
/// with its `initDataUnsafe.user.language_code`; unknown codes fall back to
/// English via the table in [`miniapp`].
#[derive(Deserialize)]
struct I18nQuery {
    lang: Option<String>,
}

async fn api_mini_app_i18n(Query(q): Query<I18nQuery>) -> Json<serde_json::Value> {
    let lang = miniapp::Lang::parse(q.lang.as_deref().unwrap_or("en"));
    let mut strings = serde_json::Map::new();
    for key in miniapp::I18N_KEYS {
        strings.insert((*key).to_string(), json!(miniapp::t(key, lang)));
    }
    Json(json!({
        "lang": lang.as_str(),
        "strings": serde_json::Value::Object(strings),
    }))
}

/// Server-authoritative live-stream config for the Mini App JS client
/// (plan P2). Mirrors [`crate::stream::backoff`] so reconnecting clients wait
/// the same exponential-backoff schedule the Rust server defines and tests.
async fn api_live_config(State(state): State<AppState>) -> Json<serde_json::Value> {
    let backoff = state.live_reconnect();
    Json(json!({
        "reconnect": {
            "base_ms": backoff.base_ms,
            "cap_ms": backoff.cap_ms,
            "max_attempts": backoff.max_attempts,
        },
        "keepalive_secs": crate::stream::backoff::WS_KEEPALIVE_SECS,
    }))
}

#[derive(Deserialize)]
struct SnapshotQuery {
    lang: Option<String>,
}

/// Consolidated cold-start bundle (plan P3). The Mini App fetches this once on
/// `/start` and hydrates every region of the app from a single round-trip
/// instead of issuing status + tickets + flows + workers + i18n + live-config
/// requests sequentially — that is what makes the first paint fast in a cold
/// Telegram WebView. The optional `?lang=` selects the i18n table (en default).
async fn api_snapshot(
    State(state): State<AppState>,
    Query(query): Query<SnapshotQuery>,
) -> Json<serde_json::Value> {
    let lang = miniapp::Lang::parse(query.lang.as_deref().unwrap_or("en"));
    let tickets = state.tickets().await;
    let presence = state.presence_map().await;
    let flows = state.recent_flows(50).await;
    let backoff = state.live_reconnect();

    let mut strings = serde_json::Map::new();
    for key in miniapp::I18N_KEYS {
        strings.insert((*key).to_string(), json!(miniapp::t(key, lang)));
    }

    Json(json!({
        "v": 1,
        "ts": chrono::Utc::now().timestamp_millis(),
        "status": {
            "online": state.is_online(),
            "jail_id": state.jail_id(),
            "tickets_count": tickets.len(),
            "workers_online": presence.len(),
            "recent_flows": flows.len(),
        },
        "tickets": wire_tickets(&tickets),
        "workers": wire_workers(&presence),
        "flows": wire_flows(&flows),
        "i18n": {"lang": lang.as_str(), "strings": serde_json::Value::Object(strings)},
        "live": {
            "reconnect": {
                "base_ms": backoff.base_ms,
                "cap_ms": backoff.cap_ms,
                "max_attempts": backoff.max_attempts,
            },
            "keepalive_secs": crate::stream::backoff::WS_KEEPALIVE_SECS,
        },
    }))
}

/// Wire ticket rows for the JSON surface; the router, board rows and snapshot
/// all share the same shape so the Mini App can render one `<tr>` template.
/// `actions` is the server-authoritative, status-driven set of board actions
/// the row may offer (band 219) — the Mini App renders exactly these buttons,
/// never a fixed claim/done/error triad.
fn wire_tickets(tickets: &[crate::state::TicketRow]) -> Vec<serde_json::Value> {
    tickets
        .iter()
        .map(|t| {
            let actions: Vec<&str> = crate::actions::available_actions(&t.status)
                .iter()
                .map(|a| a.as_str())
                .collect();
            json!({
                "id": t.id,
                "title": t.title,
                "body": t.body,
                "status": t.status,
                "product": t.product,
                "claimed_by": t.claimed_by,
                "scenario": t.scenario,
                "actions": actions,
            })
        })
        .collect()
}

fn wire_workers(
    presence: &std::collections::HashMap<String, crate::state::WorkerPresence>,
) -> Vec<serde_json::Value> {
    let mut rows: Vec<serde_json::Value> = presence
        .values()
        .map(|w| {
            let status_str = match w.status {
                crate::state::WorkerStatus::Ready => "ready",
                crate::state::WorkerStatus::Busy => "busy",
                crate::state::WorkerStatus::Offline => "offline",
            };
            json!({
                "jail_id": w.jail_id,
                "actor": w.actor,
                "ide": w.ide,
                "model": w.model,
                "agent": w.agent,
                "rank": w.rank,
                "status": status_str,
                "timezone": w.timezone,
            })
        })
        .collect();
    rows.sort_by(|a, b| a["jail_id"].as_str().cmp(&b["jail_id"].as_str()));
    rows
}

fn wire_flows(flows: &[crate::state::FlowEvent]) -> Vec<serde_json::Value> {
    flows
        .iter()
        .map(|f| {
            json!({
                "ts": f.ts.to_rfc3339(),
                "jail_id": f.jail_id,
                "action": f.action,
                "detail": f.detail,
            })
        })
        .collect()
}

#[derive(Deserialize)]
struct VerifyQuery {
    #[serde(rename = "initData", default)]
    init_data: String,
    #[serde(rename = "authDate", default)]
    auth_date: Option<i64>,
}

/// Server-side verification surface for the Telegram Mini App handshake.
/// The client sends the raw `initData` from `initDataUnsafe` along with the
/// client's current unix time; the server HMAC-SHA256 verifies the signature
/// against the bot token and enforces `auth_date` freshness, returning
/// `{ok, error?}` so the Mini App can decide whether to trust requests.
async fn api_verify_init_data(
    State(state): State<AppState>,
    Query(query): Query<VerifyQuery>,
) -> Json<serde_json::Value> {
    let token = &state.config().bot_token;
    if token.is_empty() {
        return Json(json!({"ok": false, "error": "bot token not configured"}));
    }
    if query.init_data.is_empty() {
        return Json(json!({"ok": false, "error": "no initData"}));
    }
    let now = query
        .auth_date
        .unwrap_or_else(|| chrono::Utc::now().timestamp());
    match crate::security::verify_init_data(
        &query.init_data,
        token,
        now,
        crate::security::initdata::DEFAULT_MAX_AGE_SECS,
    ) {
        Ok(()) => Json(json!({"ok": true})),
        Err(e) => Json(json!({"ok": false, "error": e.to_string()})),
    }
}

/// Query params for the board-action surface: the Telegram `initData`
/// handshake string (band 214) plus an optional `authDate` clock override that
/// mirrors `/api/verify`, so the freshness window is testable.
#[derive(Deserialize)]
struct ActionQuery {
    #[serde(rename = "initData", default)]
    init_data: String,
    #[serde(rename = "authDate", default)]
    auth_date: Option<i64>,
}

async fn api_board_claim(
    State(state): State<AppState>,
    Query(q): Query<ActionQuery>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    api_board_action(crate::actions::BoardAction::Claim, state, q, body).await
}

async fn api_board_done(
    State(state): State<AppState>,
    Query(q): Query<ActionQuery>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    api_board_action(crate::actions::BoardAction::Done, state, q, body).await
}

async fn api_board_error(
    State(state): State<AppState>,
    Query(q): Query<ActionQuery>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    api_board_action(crate::actions::BoardAction::Error, state, q, body).await
}

async fn api_board_reclaim(
    State(state): State<AppState>,
    Query(q): Query<ActionQuery>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    api_board_action(crate::actions::BoardAction::Reclaim, state, q, body).await
}

/// Shared board-action handler (band 218, plan P4). Verifies the Telegram
/// Mini App `initData` handshake before anything is mutated — a public tunnel
/// must never let an anonymous caller claim/close tickets — then parses
/// `{id, note?}` and forwards server-side to GSV's `/api/tickets/{verb}`.
async fn api_board_action(
    action: crate::actions::BoardAction,
    state: AppState,
    q: ActionQuery,
    body: serde_json::Value,
) -> Response {
    let token = &state.config().bot_token;
    if token.is_empty() {
        return (
            StatusCode::BAD_GATEWAY,
            Json(crate::actions::err_json("bot token not configured")),
        )
            .into_response();
    }
    let now = q
        .auth_date
        .unwrap_or_else(|| chrono::Utc::now().timestamp());
    if let Err(e) = crate::security::verify_init_data(
        &q.init_data,
        token,
        now,
        crate::security::initdata::DEFAULT_MAX_AGE_SECS,
    ) {
        return (
            StatusCode::FORBIDDEN,
            Json(crate::actions::err_json(&format!("initData: {e}"))),
        )
            .into_response();
    }
    let parsed = match crate::actions::parse_body(&body) {
        Ok(p) => p,
        Err(message) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(crate::actions::err_json(&message)),
            )
                .into_response();
        }
    };
    let client = crate::gsv::client::GsvClient::new(state.config());
    match client
        .board_action(action, &parsed.id, parsed.note.as_deref())
        .await
    {
        Ok(v) => Json(v).into_response(),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(crate::actions::err_json(&format!("GSV: {e}"))),
        )
            .into_response(),
    }
}

async fn dashboard() -> Html<String> {
    Html(include_str!("templates/dashboard.html").to_string())
}

async fn app_page() -> Html<String> {
    Html(include_str!("templates/base.html").to_string())
}

async fn board_page() -> Html<String> {
    Html(include_str!("templates/board.html").to_string())
}

async fn flows_page() -> Html<String> {
    Html(include_str!("templates/flows.html").to_string())
}

async fn roles_page() -> Html<String> {
    Html(include_str!("templates/roles.html").to_string())
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "service": "telenetis",
        "version": "0.1.0"
    }))
}

async fn status(State(state): State<AppState>) -> Json<serde_json::Value> {
    let tickets = state.tickets().await;
    let presence = state.presence_map().await;
    let flows = state.recent_flows(10).await;

    Json(json!({
        "online": state.is_online(),
        "jail_id": state.jail_id(),
        "tickets_count": tickets.len(),
        "workers_online": presence.len(),
        "recent_flows": flows.len(),
    }))
}

async fn api_tickets(State(state): State<AppState>) -> Json<serde_json::Value> {
    let tickets = state.tickets().await;
    Json(json!({"tickets": wire_tickets(&tickets)}))
}

async fn api_flows(State(state): State<AppState>) -> Json<serde_json::Value> {
    let flows = state.recent_flows(50).await;
    Json(json!({"flows": flows}))
}

async fn serve_css() -> impl IntoResponse {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        include_str!("static/app.css"),
    )
}

async fn serve_js() -> impl IntoResponse {
    (
        StatusCode::OK,
        [(
            header::CONTENT_TYPE,
            "application/javascript; charset=utf-8",
        )],
        include_str!("static/app.js"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::state::AppState;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    fn test_state() -> AppState {
        let cfg = Config {
            bot_token: "test".to_string(),
            gsv_url: "http://127.0.0.1:9999".to_string(),
            port: 9800,
            jail_id: "test-jail".to_string(),
            godfather_channel_id: 0,
            webhook_url: None,
            public_url: None,
            tunnel_enabled: false,
            ngrok_bin: None,
        };
        AppState::new(cfg)
    }

    #[tokio::test]
    async fn health_returns_ok() {
        let app = router(test_state());
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn static_css_served() {
        let app = router(test_state());
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/static/app.css")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(resp
            .headers()
            .get(header::CONTENT_TYPE)
            .unwrap()
            .to_str()
            .unwrap()
            .contains("text/css"));
    }

    #[tokio::test]
    async fn board_page_ok() {
        let app = router(test_state());
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/board")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn app_page_keeps_html_content_type_through_headers() {
        use crate::security::auth::security_headers;
        use axum::middleware;

        async fn headers(
            req: axum::http::Request<axum::body::Body>,
            next: middleware::Next,
        ) -> axum::response::Response {
            let mut resp = next.run(req).await;
            security_headers(&mut resp);
            resp
        }

        let app = router(test_state()).layer(middleware::from_fn(headers));
        let resp = app
            .oneshot(Request::builder().uri("/app").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(resp
            .headers()
            .get(header::CONTENT_TYPE)
            .unwrap()
            .to_str()
            .unwrap()
            .contains("text/html"));
    }

    // Reference initData for bot_token == "test" (matches test_state()).
    // Hash computed independently with OpenSSL (band 214). RAW form is
    // percent-encoded at request time via percent_encode_query().
    const TEST_USER_RAW: &str =
        "{\"id\":279058397,\"first_name\":\"Vlad\",\"language_code\":\"en\"}";
    const TEST_HASH: &str = "5cd657d1cc938ded22c052bb9450bb8f9d9842f450195b53703303cf555f410a";
    const TEST_TAMPERED_USER_RAW: &str =
        "{\"id\":999999,\"first_name\":\"Eve\",\"language_code\":\"en\"}";

    fn test_init_data(user: &str) -> String {
        format!(
            "auth_date=1750000000&query_id=AAHdF6IQAAAAAN0XohDhrOrc&user={}&hash={}",
            percent_encode_query(user),
            TEST_HASH
        )
    }

    fn percent_encode_query(input: &str) -> String {
        input
            .bytes()
            .map(|b| match b {
                b'!'
                | b'\''
                | b'('
                | b')'
                | b'*'
                | b'-'
                | b'.'
                | b'_'
                | b'~'
                | b'a'..=b'z'
                | b'A'..=b'Z'
                | b'0'..=b'9' => (b as char).to_string(),
                _ => format!("%{:02X}", b),
            })
            .collect()
    }

    #[tokio::test]
    async fn verify_endpoint_accepts_valid_init_data() {
        let app = router(test_state());
        let init = percent_encode_query(&test_init_data(TEST_USER_RAW));
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/verify?initData={}&authDate=1750000010", init))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["ok"], true);
    }

    #[tokio::test]
    async fn verify_endpoint_rejects_tampered_init_data() {
        let app = router(test_state());
        let init = percent_encode_query(&test_init_data(TEST_TAMPERED_USER_RAW));
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/verify?initData={}", init))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["ok"], false);
    }

    #[tokio::test]
    async fn verify_endpoint_missing_init_data_fails() {
        let app = router(test_state());
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/verify")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["ok"], false);
    }

    // ---- band 218 (plan P4) board action endpoints ----

    async fn post_action(
        verb: &str,
        init: &str,
        auth: i64,
        body: serde_json::Value,
    ) -> axum::response::Response {
        let app = router(test_state());
        let init_q = percent_encode_query(&test_init_data(init));
        app.oneshot(
            Request::builder()
                .uri(format!(
                    "/api/board/{verb}?initData={}&authDate={}",
                    init_q, auth
                ))
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn board_action_rejects_missing_init_data() {
        let app = router(test_state());
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/board/claim")
                    .method("POST")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"id":"T-1"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn board_action_rejects_tampered_init_data() {
        let resp = post_action(
            "claim",
            TEST_TAMPERED_USER_RAW,
            1750000010,
            serde_json::json!({"id": "T-1"}),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["ok"], false);
    }

    #[tokio::test]
    async fn board_action_rejects_missing_ticket_id() {
        let resp = post_action("claim", TEST_USER_RAW, 1750000010, serde_json::json!({})).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["ok"], false);
        assert!(json["error"].as_str().unwrap().contains("ticket id"));
    }

    #[tokio::test]
    async fn board_action_valid_claim_forwards_to_gsv() {
        // A verified handshake with a well-formed body passes validation and
        // reaches the GSV forward. Point the client at an unreachable GSV so
        // the test never touches a live board; the forward must surface as a
        // gateway failure rather than a 4xx.
        let mut cfg = Config {
            bot_token: "test".to_string(),
            gsv_url: "http://127.0.0.1:1".to_string(),
            port: 9800,
            jail_id: "test-jail".to_string(),
            godfather_channel_id: 0,
            webhook_url: None,
            public_url: None,
            tunnel_enabled: false,
            ngrok_bin: None,
        };
        cfg.bot_token = "test".to_string();
        let app = router(AppState::new(cfg));
        let init_q = percent_encode_query(&test_init_data(TEST_USER_RAW));
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/api/board/claim?initData={}&authDate=1750000010",
                        init_q
                    ))
                    .method("POST")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"id":"T-1"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);
        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["ok"], false);
    }

    #[tokio::test]
    async fn board_action_done_endpoint_registered() {
        // Same trust path for the done verb: tampered handshake is refused.
        let resp = post_action(
            "done",
            TEST_TAMPERED_USER_RAW,
            1750000010,
            serde_json::json!({"id": "T-1"}),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn board_action_error_endpoint_registered() {
        // Same trust path for the error verb: tampered handshake is refused.
        let resp = post_action(
            "error",
            TEST_TAMPERED_USER_RAW,
            1750000010,
            serde_json::json!({"id": "T-1"}),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn mini_app_i18n_returns_requested_language() {
        let app = router(test_state());
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/mini-app/i18n?lang=uk")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 8192).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["lang"], "uk");
        assert_eq!(json["strings"]["status.online"], "Онлайн");
        assert_eq!(json["strings"]["action.claim"], "Взяти");
    }

    #[tokio::test]
    async fn mini_app_i18n_defaults_to_english() {
        let app = router(test_state());
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/mini-app/i18n")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 8192).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["lang"], "en");
        assert_eq!(json["strings"]["status.online"], "Online");
        assert_eq!(json["strings"]["app.title"], "Telenetis");
    }

    #[tokio::test]
    async fn mini_app_i18n_falls_back_from_unknown_lang() {
        let app = router(test_state());
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/mini-app/i18n?lang=zz")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 8192).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["lang"], "en");
        assert_eq!(json["strings"]["nav.roles"], "Roles");
    }

    #[tokio::test]
    async fn live_config_reports_server_authoritative_backoff() {
        let app = router(test_state());
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/live/config")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let p = crate::stream::backoff::ReconnectPolicy::default();
        assert_eq!(json["reconnect"]["base_ms"], p.base_ms);
        assert_eq!(json["reconnect"]["cap_ms"], p.cap_ms);
        assert_eq!(json["reconnect"]["max_attempts"], p.max_attempts);
        assert_eq!(
            json["keepalive_secs"],
            crate::stream::backoff::WS_KEEPALIVE_SECS
        );
    }

    // ---- cold start (band 217, plan P3) snapshot + skeleton contracts ----

    #[tokio::test]
    async fn snapshot_returns_consolidated_bundle() {
        let app = router(test_state());
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/snapshot")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 64 * 1024)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        for key in [
            "v", "ts", "status", "tickets", "workers", "flows", "i18n", "live",
        ] {
            assert!(json.get(key).is_some(), "snapshot missing {}", key);
        }
        assert_eq!(json["v"], 1);
        assert_eq!(json["status"]["jail_id"], "test-jail");
        assert_eq!(json["i18n"]["lang"], "en");
    }

    #[tokio::test]
    async fn snapshot_reflects_seeded_state() {
        let state = test_state();
        state
            .set_tickets(vec![
                crate::state::TicketRow {
                    id: "T-1".to_string(),
                    title: "Fix bug".to_string(),
                    body: String::new(),
                    status: "open".to_string(),
                    product: "gsv".to_string(),
                    claimed_by: None,
                    scenario: Some("setup".to_string()),
                },
                crate::state::TicketRow {
                    id: "T-2".to_string(),
                    title: "Ship".to_string(),
                    body: String::new(),
                    status: "in_progress".to_string(),
                    product: "poolai".to_string(),
                    claimed_by: Some("test-jail".to_string()),
                    scenario: None,
                },
            ])
            .await;
        state
            .push_flow(crate::state::FlowEvent {
                ts: chrono::Utc::now(),
                jail_id: "jail-02".to_string(),
                action: "presence".to_string(),
                detail: "heartbeat".to_string(),
            })
            .await;
        state
            .update_presence(crate::state::WorkerPresence {
                jail_id: "jail-02".to_string(),
                actor: "alice".to_string(),
                ide: "cursor".to_string(),
                model: "m".to_string(),
                agent: "orchestrator".to_string(),
                rank: 7,
                status: crate::state::WorkerStatus::Ready,
                last_heartbeat: chrono::Utc::now(),
                timezone: "UTC".to_string(),
            })
            .await;

        let app = router(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/snapshot")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 64 * 1024)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"]["tickets_count"], 2);
        assert_eq!(json["status"]["workers_online"], 1);
        assert_eq!(json["tickets"].as_array().unwrap().len(), 2);
        assert_eq!(json["workers"][0]["jail_id"], "jail-02");
        assert_eq!(json["flows"].as_array().unwrap().len(), 1);
        assert_eq!(json["tickets"][0]["scenario"], "setup");
        assert_eq!(json["tickets"][1]["claimed_by"], "test-jail");
    }

    #[tokio::test]
    async fn snapshot_i18n_respects_lang_query() {
        let app = router(test_state());
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/snapshot?lang=uk")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 64 * 1024)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["i18n"]["lang"], "uk");
        assert_eq!(json["i18n"]["strings"]["action.claim"], "Взяти");
    }

    #[tokio::test]
    async fn snapshot_live_config_matches_backoff() {
        let app = router(test_state());
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/snapshot")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(resp.into_body(), 64 * 1024)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let p = crate::stream::backoff::ReconnectPolicy::default();
        assert_eq!(json["live"]["reconnect"]["base_ms"], p.base_ms);
        assert_eq!(json["live"]["reconnect"]["cap_ms"], p.cap_ms);
        assert_eq!(json["live"]["reconnect"]["max_attempts"], p.max_attempts);
        assert_eq!(
            json["live"]["keepalive_secs"],
            crate::stream::backoff::WS_KEEPALIVE_SECS
        );
    }

    #[test]
    fn all_templates_carry_skeleton_markup() {
        let templates: Vec<(&str, &str)> = vec![
            ("dashboard.html", include_str!("templates/dashboard.html")),
            ("base.html", include_str!("templates/base.html")),
            ("board.html", include_str!("templates/board.html")),
            ("flows.html", include_str!("templates/flows.html")),
            ("roles.html", include_str!("templates/roles.html")),
        ];
        for (name, html) in &templates {
            assert!(
                html.contains("class=\"skeleton"),
                "{} has no skeleton markup",
                name
            );
            assert!(
                html.contains("data-skeleton="),
                "{} has no data-skeleton clears",
                name
            );
            assert!(
                html.contains("aria-busy=\"true\""),
                "{} region is not marked busy",
                name
            );
            assert!(
                html.contains("data-area="),
                "{} regions have no hydration areas",
                name
            );
            assert!(
                html.contains("<link rel=\"preload\" href=\"/api/snapshot?lang=en\" as=\"fetch\""),
                "{} does not preload the snapshot",
                name
            );
        }
    }

    #[test]
    fn flow_log_page_starts_offline_with_skeleton() {
        let html = include_str!("templates/flows.html");
        assert!(html.contains("id=\"flow-log\""));
        assert!(html.contains("data-area=\"flow-log\""));
        assert!(html.contains("data-feed=\"offline\""));
        assert!(html.contains("data-skeleton=\"flow-log\""));
    }

    #[test]
    fn app_js_hydrates_from_snapshot() {
        let js = include_str!("static/app.js");
        // bootstrap() must open the WS immediately and fetch the one snapshot.
        assert!(js.contains("async function bootstrap()"));
        assert!(js.contains("connectWS();"));
        assert!(js.contains("fetchSnapshot"));
        assert!(js.contains("/api/snapshot?lang="));
        assert!(js.contains("hydrateFromSnapshot"));
        // Skeleton clearing + aria contract.
        assert!(js.contains("function clearArea"));
        assert!(js.contains("querySelectorAll('[data-skeleton]')"));
        assert!(js.contains("removeAttribute('aria-busy')"));
    }

    // ---- band 218 (plan P4) board action wiring contracts ----

    #[test]
    fn board_template_has_actions_column() {
        let html = include_str!("templates/board.html");
        assert!(
            html.contains("<th data-i18n=\"board.actions\">Actions</th>"),
            "board.html lacks the localized Actions column header"
        );
        // The actions column makes the board 6 wide; skeleton rows must match.
        assert!(html.contains("colspan=\"6\""));
        assert!(!html.contains("colspan=\"5\""));
    }

    #[test]
    fn board_actions_i18n_key_resolves_in_all_languages() {
        for lang in [
            crate::ui::miniapp::Lang::En,
            crate::ui::miniapp::Lang::Uk,
            crate::ui::miniapp::Lang::Ru,
        ] {
            assert!(
                !crate::ui::miniapp::t("board.actions", lang).is_empty(),
                "board.actions missing for {lang:?}"
            );
        }
    }

    #[test]
    fn action_verb_i18n_keys_resolve_in_all_languages() {
        let keys = [
            "action.claim",
            "action.done",
            "action.error",
            "action.reclaim",
            "action.claiming",
            "action.doing",
            "action.erroring",
            "action.reclaiming",
            "action.claimed",
            "action.done_ok",
            "action.error_ok",
            "action.reclaim_ok",
        ];
        for lang in [
            crate::ui::miniapp::Lang::En,
            crate::ui::miniapp::Lang::Uk,
            crate::ui::miniapp::Lang::Ru,
        ] {
            for key in &keys {
                assert!(
                    !crate::ui::miniapp::t(key, lang).is_empty(),
                    "{} missing for {lang:?}",
                    key
                );
            }
        }
    }

    #[test]
    fn app_js_wires_board_action_buttons() {
        let js = include_str!("static/app.js");
        assert!(js.contains("function makeActionButton"));
        assert!(js.contains("function actionButtonsCell"));
        assert!(js.contains("async function postBoardAction"));
        assert!(js.contains("function actionLabel"));
        // The POST must carry the initData handshake + authDate, then forward.
        assert!(js.contains("/api/board/${action}"));
        assert!(js.contains("initData"));
        assert!(js.contains("authDate"));
        // Buttons render per ticket row.
        assert!(js.contains("tr.appendChild(actionButtonsCell(tk))"));
    }

    // ---- band 219 (ticket lifecycle UX) contract tests ----

    #[test]
    fn app_js_renders_server_authoritative_actions() {
        let js = include_str!("static/app.js");
        // The row renders the actions the server wired onto the ticket, not a
        // fixed claim/done/error triad.
        assert!(js.contains("tk.actions"));
        assert!(js.contains("Array.isArray(tk.actions)"));
        assert!(js.contains("board.no_actions"));
        assert!(js.contains("function statusLabel"));
        // Reclaim rides the same makeActionButton path.
        assert!(js.contains("reclaim"));
    }

    #[test]
    fn app_js_wires_ticket_detail_and_offline_states() {
        let js = include_str!("static/app.js");
        assert!(js.contains("function renderBoardRowData"));
        assert!(js.contains("ticket-detail"));
        assert!(js.contains("board.detail"));
        assert!(js.contains("board.offline"));
        assert!(js.contains("function setBoardOffline"));
        assert!(js.contains("detail.hidden"));
    }

    #[tokio::test]
    async fn snapshot_wires_ticket_body_and_actions() {
        let state = test_state();
        state
            .set_tickets(vec![
                crate::state::TicketRow {
                    id: "T-1".to_string(),
                    title: "Open task".to_string(),
                    body: "   a description   ".to_string(),
                    status: "open".to_string(),
                    product: "gsv".to_string(),
                    claimed_by: None,
                    scenario: None,
                },
                crate::state::TicketRow {
                    id: "T-2".to_string(),
                    title: "Claimed task".to_string(),
                    body: String::new(),
                    status: "in_progress".to_string(),
                    product: "poolai".to_string(),
                    claimed_by: Some("jail-01".to_string()),
                    scenario: None,
                },
                crate::state::TicketRow {
                    id: "T-3".to_string(),
                    title: "Finished".to_string(),
                    body: String::new(),
                    status: "done".to_string(),
                    product: "gsv".to_string(),
                    claimed_by: None,
                    scenario: None,
                },
            ])
            .await;
        let app = router(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/snapshot")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(resp.into_body(), 64 * 1024)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let tickets = json["tickets"].as_array().unwrap();
        // Body now rides the wire for the ticket-detail view.
        assert_eq!(tickets[0]["body"], "   a description   ");
        // Context-sensitive actions: open → claim only; in_progress → the
        // three working verbs; terminal → none.
        assert_eq!(tickets[0]["actions"], serde_json::json!(["claim"]));
        assert_eq!(
            tickets[1]["actions"],
            serde_json::json!(["done", "error", "reclaim"])
        );
        assert_eq!(tickets[2]["actions"], serde_json::json!([]));
    }

    #[tokio::test]
    async fn board_reclaim_endpoint_registered() {
        // Reclaim uses the same trust path as the other verbs: a tampered
        // handshake is refused before anything reaches GSV.
        let resp = post_action(
            "reclaim",
            TEST_TAMPERED_USER_RAW,
            1750000010,
            serde_json::json!({"id": "T-1"}),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn board_reclaim_rejects_missing_init_data() {
        let app = router(test_state());
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/board/reclaim")
                    .method("POST")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"id":"T-1"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn board_reclaim_valid_forwards_to_gsv() {
        // A verified handshake with a well-formed body passes validation and
        // reaches the GSV reclaim forward; an unreachable GSV must surface as
        // a gateway failure rather than a 4xx.
        let cfg = Config {
            bot_token: "test".to_string(),
            gsv_url: "http://127.0.0.1:1".to_string(),
            port: 9800,
            jail_id: "test-jail".to_string(),
            godfather_channel_id: 0,
            webhook_url: None,
            public_url: None,
            tunnel_enabled: false,
            ngrok_bin: None,
        };
        let app = router(AppState::new(cfg));
        let init_q = percent_encode_query(&test_init_data(TEST_USER_RAW));
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/api/board/reclaim?initData={}&authDate=1750000010",
                        init_q
                    ))
                    .method("POST")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"id":"T-1"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);
        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["ok"], false);
    }

    #[tokio::test]
    async fn status_display_i18n_keys_resolve() {
        for lang in [
            crate::ui::miniapp::Lang::En,
            crate::ui::miniapp::Lang::Uk,
            crate::ui::miniapp::Lang::Ru,
        ] {
            for key in [
                "status.open",
                "status.in_progress",
                "status.done",
                "status.blocked",
                "status.closed",
                "board.detail",
                "board.no_description",
                "board.no_actions",
                "board.offline",
            ] {
                assert!(
                    !crate::ui::miniapp::t(key, lang).is_empty(),
                    "{} missing for {lang:?}",
                    key
                );
            }
        }
    }

    #[tokio::test]
    async fn i18n_table_matches_html_i18n_attributes() {
        let app = router(test_state());
        let resp = app
            .oneshot(Request::builder().uri("/app").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 64 * 1024)
            .await
            .unwrap();
        let html = String::from_utf8_lossy(&body);

        // /app (dashboard) shell only carries app.* and status.* keys — every
        // data-i18n attribute the template uses must resolve to a non-empty
        // string for at least the default English UI.
        for key in ["app.title", "app.subtitle", "status.loading"] {
            assert!(
                html.contains(&format!("data-i18n=\"{}\"", key)),
                "template missing attribute for {}",
                key
            );
            assert!(!crate::ui::miniapp::t(key, crate::ui::miniapp::Lang::En).is_empty());
        }
    }
}
