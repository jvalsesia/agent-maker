use super::AppState;
use crate::{error::AppResult, templates::model::Category};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, post},
};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub category: Option<String>,
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/templates", get(list))
        .route("/templates/agents/:slug", get(get_agent))
        .route("/templates/skills/:slug", get(get_skill))
        .route("/templates/agents/:slug/adopt", post(adopt_agent))
        .route("/templates/skills/:slug/adopt", post(adopt_skill))
}

async fn list(
    State(s): State<Arc<AppState>>,
    Query(q): Query<ListQuery>,
) -> AppResult<Json<serde_json::Value>> {
    let cat = q.category.as_deref().and_then(Category::parse);
    let resp = s.templates.list(cat);
    Ok(Json(serde_json::to_value(resp).unwrap()))
}

async fn get_agent(
    State(s): State<Arc<AppState>>,
    Path(slug): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let detail = s.templates.get_agent(&slug)?;
    Ok(Json(serde_json::json!({ "agent": detail })))
}

async fn get_skill(
    State(s): State<Arc<AppState>>,
    Path(slug): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let detail = s.templates.get_skill(&slug)?;
    Ok(Json(serde_json::json!({ "skill": detail })))
}

async fn adopt_agent(
    State(s): State<Arc<AppState>>,
    Path(slug): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let resp = s.templates.adopt_agent(&slug).await?;
    Ok(Json(serde_json::to_value(resp).unwrap()))
}

async fn adopt_skill(
    State(s): State<Arc<AppState>>,
    Path(slug): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let resp = s.templates.adopt_skill(&slug).await?;
    Ok(Json(serde_json::to_value(resp).unwrap()))
}
