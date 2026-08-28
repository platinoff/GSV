use axum::body::Body;
use axum::http::{Request, StatusCode};
use telenetis::config::Config;
use telenetis::state::AppState;
use tower::ServiceExt;

fn test_state() -> AppState {
    let cfg = Config {
        bot_token: "test_token".to_string(),
        gsv_url: "http://127.0.0.1:9999".to_string(),
        port: 9800,
        jail_id: "integration-jail".to_string(),
        godfather_channel_id: 0,
        webhook_url: None,
        public_url: None,
        tunnel_enabled: false,
        ngrok_bin: None,
    };
    AppState::new(cfg)
}

#[tokio::test]
async fn health_and_status_endpoints() {
    let state = test_state();
    let app = telenetis::ui::router(state.clone())
        .merge(telenetis::bot::webhook::router(state.clone()))
        .merge(telenetis::stream::ws::router(state.clone()))
        .merge(telenetis::stream::sse::router(state.clone()));

    // /health
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // /api/status
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), 1024 * 64)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["jail_id"], "integration-jail");

    // /board
    let resp = app
        .clone()
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
async fn webhook_accepts_telegram_update() {
    let state = test_state();
    let app = telenetis::bot::webhook::router(state);
    let payload = serde_json::json!({
        "message": {
            "chat": {"id": 123},
            "from": {"username": "tester"},
            "text": "/start"
        }
    });
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/webhook")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn static_assets_served() {
    let state = test_state();
    let app = telenetis::ui::router(state);
    for uri in ["/static/app.css", "/static/app.js", "/", "/flows", "/roles"] {
        let resp = app
            .clone()
            .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK, "uri {}", uri);
    }
}

#[tokio::test]
async fn snapshot_endpoint_served() {
    let state = test_state();
    let app = telenetis::ui::router(state.clone())
        .merge(telenetis::stream::ws::router(state.clone()))
        .merge(telenetis::stream::sse::router(state.clone()));

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/snapshot?lang=en")
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
    assert_eq!(json["status"]["jail_id"], "integration-jail");
    assert_eq!(json["i18n"]["lang"], "en");
    assert!(json["tickets"].is_array());
    assert!(json["flows"].is_array());
    assert!(json["workers"].is_array());
    assert_eq!(json["live"]["keepalive_secs"], 25);
}
