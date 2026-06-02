use super::AppState;
use crate::error::AppResult;
use crate::subagents::model::{AttachInput, ReorderInput, UpdateInput};
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
        .route(
            "/agents/{agent_id}/subagents",
            get(list).post(attach).put(reorder),
        )
        .route(
            "/agents/{agent_id}/subagents/{child_id}",
            delete(detach).put(update),
        )
}

async fn list(
    State(s): State<Arc<AppState>>,
    Path(agent_id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let attached = s.subagents.list(agent_id).await?;
    Ok(Json(serde_json::json!({ "attached": attached })))
}

async fn attach(
    State(s): State<Arc<AppState>>,
    Path(agent_id): Path<Uuid>,
    Json(body): Json<AttachInput>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let row = s.subagents.attach(agent_id, body).await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!({ "attached": row }))))
}

async fn update(
    State(s): State<Arc<AppState>>,
    Path((agent_id, child_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<UpdateInput>,
) -> AppResult<Json<serde_json::Value>> {
    let row = s.subagents.update(agent_id, child_id, body).await?;
    Ok(Json(serde_json::json!({ "attached": row })))
}

async fn reorder(
    State(s): State<Arc<AppState>>,
    Path(agent_id): Path<Uuid>,
    Json(body): Json<ReorderInput>,
) -> AppResult<Json<serde_json::Value>> {
    let attached = s.subagents.reorder(agent_id, body).await?;
    Ok(Json(serde_json::json!({ "attached": attached })))
}

async fn detach(
    State(s): State<Arc<AppState>>,
    Path((agent_id, child_id)): Path<(Uuid, Uuid)>,
) -> AppResult<StatusCode> {
    s.subagents.detach(agent_id, child_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
