use super::AppState;
use crate::{
    error::AppResult,
    skills::model::{CloneRequest, ListQuery, SkillUpsert},
};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
};
use std::sync::Arc;
use uuid::Uuid;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/skills", get(list).post(create))
        .route("/skills/{id}", get(get_one).put(update).delete(delete))
        .route("/skills/{id}/clone", post(clone_skill))
}

async fn list(
    State(s): State<Arc<AppState>>,
    Query(q): Query<ListQuery>,
) -> AppResult<Json<serde_json::Value>> {
    let rows = s.skills.list(q).await?;
    Ok(Json(serde_json::json!({ "skills": rows })))
}

async fn create(
    State(s): State<Arc<AppState>>,
    Json(body): Json<SkillUpsert>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let resp = s.skills.create(body).await?;
    Ok((StatusCode::CREATED, Json(serde_json::to_value(resp).unwrap())))
}

async fn get_one(
    State(s): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let detail = s.skills.get_detail(id).await?;
    Ok(Json(serde_json::to_value(detail).unwrap()))
}

async fn update(
    State(s): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(body): Json<SkillUpsert>,
) -> AppResult<Json<serde_json::Value>> {
    let resp = s.skills.update(id, body).await?;
    Ok(Json(serde_json::to_value(resp).unwrap()))
}

async fn delete(
    State(s): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    s.skills.delete(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn clone_skill(
    State(s): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    body: Option<Json<CloneRequest>>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let req = body.map(|Json(b)| b).unwrap_or_default();
    let resp = s.skills.clone_skill(id, req).await?;
    Ok((StatusCode::CREATED, Json(serde_json::to_value(resp).unwrap())))
}
