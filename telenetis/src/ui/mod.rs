use crate::state::AppState;
use axum::{
    extract::State,
    http::{header, StatusCode},
    response::{Html, IntoResponse},
    routing::get,
    Json, Router,
};
use serde_json::json;

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
        .with_state(state)
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
            .oneshot(
                Request::builder()
                    .uri("/app")
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
            .contains("text/html"));
    }
}
