use async_stream::stream;
use axum::{
    extract::State,
    response::sse::{Event, Sse},
    routing::get,
    Router,
};
use futures_core::stream::Stream;
use std::sync::Arc;

use crate::db::DbPool;
use crate::services::scheduler::{EventSender, MonitorEvent};

pub fn router(pool: DbPool, event_sender: EventSender) -> Router {
    let state = Arc::new((pool, event_sender));

    Router::new()
        .route("/", get(sse_stream))
        .route("/monitors", get(sse_monitors))
        .route("/heartbeats", get(sse_heartbeats))
        .with_state(state)
}

type SseState = Arc<(DbPool, EventSender)>;

async fn sse_stream(
    State(state): State<SseState>,
) -> Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>> {
    let (_pool, event_sender) = state.as_ref().clone();
    let mut receiver = event_sender.subscribe();

    let stream = stream! {
        while let Ok(event) = receiver.recv().await {
            let data = serde_json::to_string(&event).unwrap_or_default();
            yield Ok(Event::default().data(data));
        }
    };

    Sse::new(stream)
}

async fn sse_monitors(
    State(state): State<SseState>,
) -> Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>> {
    let (_pool, event_sender) = state.as_ref().clone();
    let mut receiver = event_sender.subscribe();

    let stream = stream! {
        while let Ok(MonitorEvent { id, event_type }) = receiver.recv().await {
            let data = serde_json::json!({
                "id": id,
                "event_type": event_type,
            }).to_string();
            yield Ok(Event::default().data(data));
        }
    };

    Sse::new(stream)
}

async fn sse_heartbeats(
    State(state): State<SseState>,
) -> Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>> {
    let (_pool, event_sender) = state.as_ref().clone();
    let mut receiver = event_sender.subscribe();

    let stream = stream! {
        while let Ok(MonitorEvent { id, event_type }) = receiver.recv().await {
            let data = serde_json::json!({
                "id": id,
                "event_type": event_type,
            }).to_string();
            yield Ok(Event::default().data(data));
        }
    };

    Sse::new(stream)
}
