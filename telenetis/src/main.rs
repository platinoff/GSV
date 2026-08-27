use axum::middleware;
use tracing_subscriber::EnvFilter;

async fn security_headers_middleware(
    request: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let mut response = next.run(request).await;
    telenetis::security::auth::security_headers(&mut response);
    response
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive("telenetis=debug".parse().unwrap()),
        )
        .init();

    let config = telenetis::config::Config::from_env();
    let state = telenetis::state::AppState::new(config.clone());
    let gsv_client = telenetis::gsv::client::GsvClient::new(&config);
    telenetis::gsv::poll::spawn_poll_loop(gsv_client, state.clone());
    let app = telenetis::ui::router(state.clone())
        .merge(telenetis::bot::webhook::router(state.clone()))
        .merge(telenetis::stream::ws::router(state.clone()))
        .merge(telenetis::stream::sse::router(state.clone()))
        .layer(middleware::from_fn(security_headers_middleware))
        .layer(telenetis::security::limit_layer());

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.port))
        .await
        .expect("failed to bind TCP listener");

    tracing::info!("Telenetis starting on port {}", config.port);
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c()
                .await
                .expect("failed to listen for ctrl+c");
            tracing::info!("Shutting down telenetis");
        })
        .await
        .expect("server error");
}
