use super::AppState;
use crate::chat::model::ChatStartRequest;
use crate::error::AppResult;
use axum::{
    Json, Router,
    extract::{Path, State},
    response::sse::{Event, Sse},
    routing::post,
};
use futures::{Stream, StreamExt};
use std::convert::Infallible;
use std::sync::Arc;
use uuid::Uuid;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/conversations/:id/chat", post(chat))
}

async fn chat(
    State(s): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(body): Json<ChatStartRequest>,
) -> AppResult<Sse<impl Stream<Item = Result<Event, Infallible>>>> {
    let rx = s.chat.start(id, body).await?;
    let stream = rx.map(|event| {
        let ev = Event::default()
            .json_data(&event)
            .expect("stream event serializes to JSON");
        Ok(ev)
    });
    Ok(Sse::new(stream))
}
