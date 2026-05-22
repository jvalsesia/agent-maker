use super::AppState;
use crate::{
    conversations::model::{CreateInput, RenameInput},
    error::AppResult,
};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::get,
};
use std::sync::Arc;
use uuid::Uuid;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/agents/:agent_id/conversations",
            get(list).post(create),
        )
        .route(
            "/conversations/:id",
            get(get_one).patch(rename).delete(delete_one),
        )
        .route("/conversations/:id/messages", get(list_messages))
}

async fn list(
    State(s): State<Arc<AppState>>,
    Path(agent_id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let rows = s.conversations.list_for_agent(agent_id).await?;
    Ok(Json(serde_json::json!({ "conversations": rows })))
}

async fn create(
    State(s): State<Arc<AppState>>,
    Path(agent_id): Path<Uuid>,
    body: Option<Json<CreateInput>>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let input = body.map(|Json(b)| b).unwrap_or_default();
    let conv = s.conversations.create(agent_id, input).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "conversation": conv, "warnings": [] })),
    ))
}

async fn get_one(
    State(s): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let conv = s.conversations.get(id).await?;
    Ok(Json(serde_json::json!({ "conversation": conv })))
}

async fn rename(
    State(s): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(body): Json<RenameInput>,
) -> AppResult<Json<serde_json::Value>> {
    let conv = s.conversations.rename(id, body).await?;
    Ok(Json(serde_json::json!({ "conversation": conv, "warnings": [] })))
}

async fn delete_one(
    State(s): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    s.conversations.delete(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_messages(
    State(s): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let resp = s.conversations.list_messages(id).await?;
    Ok(Json(serde_json::to_value(resp).unwrap()))
}
