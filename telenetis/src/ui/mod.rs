use crate::state::AppState;
use axum::{
    extract::{Query, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse},
    routing::get,
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
    let rows: Vec<serde_json::Value> = tickets
        .iter()
        .map(|t| {
            json!({
                "id": t.id,
                "title": t.title,
                "status": t.status,
                "product": t.product,
                "claimed_by": t.claimed_by,
            })
        })
        .collect();
    Json(json!({"tickets": rows}))
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
