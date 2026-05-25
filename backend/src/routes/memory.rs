use super::AppState;
use crate::{
    error::AppResult,
    memory::model::{MemoryQuery},
};
use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{delete, post},
};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/conversations/:id/memory/query",
            post(query),
        )
        .route(
            "/conversations/:id/memory/embed-pending",
            post(embed_pending),
        )
        .route("/conversations/:id/memory", delete(clear).get(stats))
}

async fn stats(
    State(s): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let embedded = s.memory.stats(id).await?;
    Ok(Json(json!({ "embedded": embedded })))
}

async fn query(
    State(s): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(body): Json<MemoryQuery>,
) -> AppResult<Json<serde_json::Value>> {
    let block = s.memory.query(id, body).await?;
    Ok(Json(serde_json::to_value(block).unwrap()))
}

async fn embed_pending(
    State(s): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let report = s.memory.embed_pending(id).await?;
    Ok(Json(serde_json::to_value(report).unwrap()))
}

async fn clear(
    State(s): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let report = s.memory.clear(id).await?;
    Ok(Json(serde_json::to_value(report).unwrap()))
}
