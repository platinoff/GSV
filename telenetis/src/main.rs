use axum::middleware;
use tracing_subscriber::EnvFilter;

use telenetis::state::register_self_presence;

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

    dotenvy::from_path(format!("{}/.env", env!("CARGO_MANIFEST_DIR"))).ok();

    let config = telenetis::config::Config::from_env();
    let state = telenetis::state::AppState::new(config.clone());
    let gsv_client = telenetis::gsv::client::GsvClient::new(&config);
    telenetis::gsv::poll::spawn_poll_loop(gsv_client, state.clone());

    // Auto-stand up a public HTTPS tunnel (ngrok) when one is needed so the
    // Telegram WebApp button (`/app`) works from phones / remote clients. The
    // live URL is published on state where webhook/button codepaths read it.
    {
        let tunnel_state = state.clone();
        let config = config.clone();
        tokio::spawn(async move {
            match telenetis::tunnel::ensure_public_url(&config).await {
                Ok(url) => {
                    tunnel_state.set_tunnel_url(url.clone()).await;
                    tracing::info!("Public tunnel URL: {}", url);
                    if !config.bot_token.is_empty() {
                        let bot = telenetis::bot::telegram::TelegramBot::new(&config);
                        match bot.set_chat_menu_button(&url).await {
                            Ok(_) => {
                                tracing::info!("Telegram menu button points at {}", url)
                            }
                            Err(e) => {
                                tracing::warn!("Failed to set Telegram menu button: {}", e)
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Could not auto-start tunnel: {}", e);
                }
            }
        });
    }

    register_self_presence(&state);
    let heartbeat_state = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            register_self_presence(&heartbeat_state);
        }
    });

    let bot = telenetis::bot::telegram::TelegramBot::new(&config);
    if config.bot_token.is_empty() {
        tracing::warn!(
            "TELENETIS_BOT_TOKEN not set — Telegram bot features (polling/webhooks) are disabled."
        );
    } else if let Some(webhook_url) = &config.webhook_url {
        let full = if webhook_url.ends_with("/webhook") {
            webhook_url.clone()
        } else {
            format!("{}/webhook", webhook_url.trim_end_matches('/'))
        };
        match bot.set_webhook(&full).await {
            Ok(_) => tracing::info!("Telegram webhook registered at {}", full),
            Err(e) => {
                tracing::warn!("Failed to register Telegram webhook at {}: {}", full, e);
                let poll_state = state.clone();
                tokio::spawn(async move {
                    telenetis::bot::webhook::run_polling(poll_state).await;
                });
            }
        }
    } else {
        tracing::info!(
            "TELENETIS_WEBHOOK_URL not set — using long polling (getUpdates). \
             Bot will receive Telegram updates without a public tunnel."
        );
        let poll_state = state.clone();
        tokio::spawn(async move {
            telenetis::bot::webhook::run_polling(poll_state).await;
        });
    }

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
