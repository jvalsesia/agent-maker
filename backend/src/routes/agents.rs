use super::AppState;
use crate::{
    agents::{
        model::{AgentUpsert, CloneRequest, ListQuery, PutAgentKey},
        service::available_models,
    },
    error::{AppError, AppResult},
    llm::ProviderName,
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
        .route("/agents", get(list).post(create))
        .route(
            "/agents/:id",
            get(get_one).put(update).delete(delete),
        )
        .route("/agents/:id/clone", post(clone_agent))
        .route(
            "/agents/:id/key",
            axum::routing::put(put_key).delete(delete_key),
        )
        .route("/agents/models", get(models))
}

async fn list(
    State(s): State<Arc<AppState>>,
    Query(q): Query<ListQuery>,
) -> AppResult<Json<serde_json::Value>> {
    let rows = s.agents.list(q).await?;
    Ok(Json(serde_json::json!({ "agents": rows })))
}

async fn create(
    State(s): State<Arc<AppState>>,
    Json(body): Json<AgentUpsert>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let resp = s.agents.create(body).await?;
    Ok((StatusCode::CREATED, Json(serde_json::to_value(resp).unwrap())))
}

async fn get_one(
    State(s): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let agent = s.agents.get(id).await?;
    Ok(Json(serde_json::json!({ "agent": agent })))
}

async fn update(
    State(s): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(body): Json<AgentUpsert>,
) -> AppResult<Json<serde_json::Value>> {
    let resp = s.agents.update(id, body).await?;
    Ok(Json(serde_json::to_value(resp).unwrap()))
}

async fn delete(
    State(s): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    s.agents.delete(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn clone_agent(
    State(s): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    body: Option<Json<CloneRequest>>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let req = body.map(|Json(b)| b).unwrap_or_default();
    let resp = s.agents.clone_agent(id, req).await?;
    Ok((StatusCode::CREATED, Json(serde_json::to_value(resp).unwrap())))
}

async fn put_key(
    State(s): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(body): Json<PutAgentKey>,
) -> AppResult<Json<serde_json::Value>> {
    if body.key.trim().is_empty() {
        return Err(AppError::validation_field("key", "is required"));
    }
    let out = s.agents.save_key(id, &body.key).await?;
    Ok(Json(serde_json::to_value(out).unwrap()))
}

async fn delete_key(
    State(s): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    s.agents.delete_key(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, serde::Deserialize)]
struct ModelsQuery {
    provider: String,
}

async fn models(
    State(_): State<Arc<AppState>>,
    Query(q): Query<ModelsQuery>,
) -> AppResult<Json<serde_json::Value>> {
    let p: ProviderName = q
        .provider
        .parse()
        .map_err(|_| AppError::validation_field("provider", "unknown provider"))?;
    Ok(Json(serde_json::json!({
        "provider": p.as_str(),
        "models": available_models(p),
    })))
}
