use super::AppState;
use crate::{
    error::{AppError, AppResult},
    llm::{ProviderName, provider::ProviderTestError},
    secrets::SecretStore,
    settings::model::{PutKey, UpdateSettings, WipeRequest},
};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post, put},
};
use std::sync::Arc;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/health", get(health))
        .route("/settings", get(get_settings).put(put_settings))
        .route("/settings/providers/{name}/key", put(put_key).delete(delete_key))
        .route("/settings/providers/{name}/test", post(test_provider))
        .route("/settings/data/wipe", post(wipe))
}

async fn health(State(s): State<Arc<AppState>>) -> AppResult<Json<serde_json::Value>> {
    sqlx::query("SELECT 1").execute(&s.settings.pool).await?;
    Ok(Json(serde_json::json!({
        "status": "ok",
        "db": "ok",
        "key_store": s.settings.secrets.backend_name(),
    })))
}

async fn get_settings(State(s): State<Arc<AppState>>) -> AppResult<Json<serde_json::Value>> {
    let dto = s.settings.read().await?;
    Ok(Json(serde_json::to_value(dto).unwrap()))
}

async fn put_settings(
    State(s): State<Arc<AppState>>,
    Json(body): Json<UpdateSettings>,
) -> AppResult<Json<serde_json::Value>> {
    let dto = s.settings.update(body).await?;
    Ok(Json(serde_json::to_value(dto).unwrap()))
}

fn parse_provider(name: &str) -> AppResult<ProviderName> {
    name.parse::<ProviderName>().map_err(|_| {
        AppError::validation_field("provider", "unknown provider")
    })
}

async fn put_key(
    State(s): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(body): Json<PutKey>,
) -> AppResult<StatusCode> {
    let p = parse_provider(&name)?;
    s.settings.put_key(p, body).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_key(
    State(s): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> AppResult<StatusCode> {
    let p = parse_provider(&name)?;
    s.settings.delete_key(p).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn test_provider(
    State(s): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let p = parse_provider(&name)?;
    let base_url = s.settings.base_url_for(p).await?;
    let provider = s.providers.get(p);
    match provider.test(base_url.as_deref()).await {
        Ok(out) => Ok(Json(serde_json::to_value(out).unwrap())),
        Err(e) => Err(map_test_error(p, e)),
    }
}

fn map_test_error(p: ProviderName, e: ProviderTestError) -> AppError {
    let (code, message) = match &e {
        ProviderTestError::NoKey(_) => ("no_key", "No API key configured for this provider".to_string()),
        ProviderTestError::Http { status, body } if *status == 401 || *status == 403 => (
            "invalid_api_key",
            format!("{} returned {}: {}", p.as_str(), status, body),
        ),
        ProviderTestError::Http { status, body } => (
            "provider_http_error",
            format!("{} returned {}: {}", p.as_str(), status, body),
        ),
        ProviderTestError::Network(msg) => ("test_unreachable", msg.clone()),
        ProviderTestError::Backend(msg) => ("provider_backend_error", msg.clone()),
    };
    AppError::Provider {
        provider: p.as_str().to_string(),
        code: code.to_string(),
        message,
        status: StatusCode::BAD_REQUEST,
    }
}

async fn wipe(
    State(s): State<Arc<AppState>>,
    Json(body): Json<WipeRequest>,
) -> AppResult<StatusCode> {
    s.settings.wipe(body).await?;
    Ok(StatusCode::NO_CONTENT)
}
