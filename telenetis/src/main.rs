use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive("telenetis=debug".parse().unwrap()),
        )
        .init();

    let config = telenetis::config::Config::from_env();
    let state = telenetis::state::AppState::new(config.clone());
    let app = telenetis::ui::router(state.clone())
        .merge(telenetis::bot::webhook::router(state.clone()))
        .merge(telenetis::stream::ws::router(state.clone()));

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.port))
        .await
        .unwrap();

    tracing::info!("Telenetis starting on port {}", config.port);
    axum::serve(listener, app).await.unwrap();
}
