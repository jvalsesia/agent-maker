use super::AppState;
use crate::{error::AppResult, skill_attachments::model::AttachReplaceInput};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get},
};
use std::sync::Arc;
use uuid::Uuid;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/agents/{agent_id}/skills", get(list).put(replace))
        .route("/agents/{agent_id}/skills/{skill_id}", delete(detach))
        .route("/agents/{agent_id}/compose", get(compose))
}

async fn list(
    State(s): State<Arc<AppState>>,
    Path(agent_id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let rows = s.attachments.list(agent_id).await?;
    Ok(Json(serde_json::json!({ "attached": rows })))
}

async fn replace(
    State(s): State<Arc<AppState>>,
    Path(agent_id): Path<Uuid>,
    Json(body): Json<AttachReplaceInput>,
) -> AppResult<Json<serde_json::Value>> {
    let resp = s.attachments.replace(agent_id, body).await?;
    Ok(Json(serde_json::to_value(resp).unwrap()))
}

async fn detach(
    State(s): State<Arc<AppState>>,
    Path((agent_id, skill_id)): Path<(Uuid, Uuid)>,
) -> AppResult<StatusCode> {
    s.attachments.detach_one(agent_id, skill_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn compose(
    State(s): State<Arc<AppState>>,
    Path(agent_id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let preview = s.attachments.compose(agent_id).await?;
    Ok(Json(serde_json::to_value(preview).unwrap()))
}
