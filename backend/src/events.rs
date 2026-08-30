use axum::{
    extract::State,
    response::sse::{Event, KeepAlive, Sse},
    routing::get,
    Router,
};
use futures::{future, StreamExt};
use std::convert::Infallible;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;

use crate::state::AppState;
use crate::state::{NotifEvent, StateEvent, UpdateProgress};

fn make_sse_stream<T: serde::Serialize + Clone + Send + 'static>(
    rx: broadcast::Receiver<T>,
    event_type: &'static str,
) -> impl futures::Stream<Item = Result<Event, Infallible>> {
    BroadcastStream::new(rx).filter_map(move |r| match r {
        Ok(evt) => future::ready(Some(Ok(Event::default()
            .event(event_type)
            .json_data(evt)
            .unwrap()))),
        Err(_) => future::ready(None),
    })
}

async fn sse_events_h(
    State(tx): State<broadcast::Sender<StateEvent>>,
) -> Sse<impl futures::Stream<Item = Result<Event, Infallible>>> {
    Sse::new(make_sse_stream(tx.subscribe(), "containers")).keep_alive(KeepAlive::default())
}

async fn sse_updates_h(
    State(tx): State<broadcast::Sender<UpdateProgress>>,
) -> Sse<impl futures::Stream<Item = Result<Event, Infallible>>> {
    Sse::new(make_sse_stream(tx.subscribe(), "update-progress")).keep_alive(KeepAlive::default())
}

async fn sse_notifications_h(
    State(tx): State<broadcast::Sender<NotifEvent>>,
) -> Sse<impl futures::Stream<Item = Result<Event, Infallible>>> {
    Sse::new(make_sse_stream(tx.subscribe(), "notification")).keep_alive(KeepAlive::default())
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/events", get(sse_events_h))
        .route("/api/updates", get(sse_updates_h))
        .route("/api/notifications", get(sse_notifications_h))
}
