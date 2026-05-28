use super::{
    cookies::{ACCESS_COOKIE, extract},
    model::{AuthContext, AuthError},
};
use crate::routes::AppState;
use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Json, Response},
};
use axum_extra::extract::cookie::CookieJar;
use std::sync::Arc;
use uuid::Uuid;

pub async fn require_auth(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    mut req: Request,
    next: Next,
) -> Response {
    // Health endpoint stays public so docker/Railway healthchecks don't
    // need credentials. axum strips the `/api` prefix on nested routes
    // before middleware sees the request, so match either form.
    let path = req.uri().path();
    if path == "/api/health" || path == "/health" {
        return next.run(req).await;
    }

    let auth = &state.auth;
    if auth.cfg.disable_auth && auth.cfg.is_development {
        req.extensions_mut().insert(dev_fake_context());
        return next.run(req).await;
    }

    let token = match extract(&jar, ACCESS_COOKIE) {
        Some(t) if !t.is_empty() => t,
        _ => return unauthorized("missing_token"),
    };

    match auth.validate_access(&token).await {
        Ok(ctx) => {
            req.extensions_mut().insert(ctx);
            next.run(req).await
        }
        Err(AuthError::Unavailable(msg)) => {
            tracing::warn!(error = %msg, "auth unavailable");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({"error": "auth_unavailable"})),
            )
                .into_response()
        }
        Err(e) => {
            tracing::debug!(error = %e, "access token rejected");
            unauthorized("invalid_token")
        }
    }
}

pub async fn require_admin(
    req: Request,
    next: Next,
) -> Response {
    if let Some(ctx) = req.extensions().get::<AuthContext>() {
        if ctx.is_admin() {
            return next.run(req).await;
        }
    }
    (
        StatusCode::FORBIDDEN,
        Json(serde_json::json!({"error": "forbidden"})),
    )
        .into_response()
}

fn unauthorized(code: &'static str) -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(serde_json::json!({"error": code})),
    )
        .into_response()
}

fn dev_fake_context() -> AuthContext {
    AuthContext {
        sub: Uuid::nil(),
        email: "dev@local".to_string(),
        roles: vec!["admin".to_string()],
    }
}
