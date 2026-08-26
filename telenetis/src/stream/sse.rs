use axum::extract::State;
use axum::response::sse::{Event, Sse};
use futures::stream::Stream;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

pub fn router(state: crate::state::AppState) -> axum::Router {
    axum::Router::new()
        .route("/events", axum::routing::get(sse_handler))
        .with_state(state)
}

async fn sse_handler(
    State(state): State<crate::state::AppState>,
) -> Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>> {
    let rx = state.flows_tx().subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|result| {
        let json = match result {
            Ok(event) => serde_json::to_string(&event).unwrap_or_default(),
            Err(_) => return None,
        };
        Some(Ok(Event::default().data(json)))
    });
    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(std::time::Duration::from_secs(30))
            .text("ping"),
    )
}
