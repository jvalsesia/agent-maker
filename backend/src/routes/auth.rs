use super::AppState;
use crate::auth::{
    AuthContext, AuthError, LoginRequest, MeResponse,
    cookies::{ACCESS_COOKIE, REFRESH_COOKIE, clear_auth_cookies, extract, set_auth_cookies},
};
use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use axum_extra::extract::cookie::CookieJar;
use std::sync::Arc;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/login", post(login))
        .route("/refresh", post(refresh))
        .route("/logout", post(logout))
        .route("/me", get(me))
}

async fn login(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    Json(body): Json<LoginRequest>,
) -> Response {
    let auth = &state.auth;
    match auth.login(&body.email, &body.password).await {
        Ok(tokens) => match auth.validate_access(&tokens.token).await {
            Ok(ctx) => {
                let jar = set_auth_cookies(
                    jar,
                    &auth.cfg,
                    &tokens.token,
                    tokens.refresh_token.as_deref(),
                );
                (
                    jar,
                    Json(MeResponse {
                        email: ctx.email,
                        roles: ctx.roles,
                    }),
                )
                    .into_response()
            }
            Err(_) => unauthorized("invalid_token"),
        },
        Err(AuthError::InvalidCredentials) => unauthorized("invalid_credentials"),
        Err(AuthError::Unavailable(_)) => unavailable(),
        Err(_) => unauthorized("invalid_credentials"),
    }
}

async fn refresh(State(state): State<Arc<AppState>>, jar: CookieJar) -> Response {
    let auth = &state.auth;
    let rt = match extract(&jar, REFRESH_COOKIE) {
        Some(v) if !v.is_empty() => v,
        _ => return unauthorized("refresh_invalid"),
    };
    match auth.refresh(&rt).await {
        Ok(tokens) => match auth.validate_access(&tokens.token).await {
            Ok(ctx) => {
                let jar = set_auth_cookies(
                    jar,
                    &auth.cfg,
                    &tokens.token,
                    tokens.refresh_token.as_deref().or(Some(&rt)),
                );
                (
                    jar,
                    Json(MeResponse {
                        email: ctx.email,
                        roles: ctx.roles,
                    }),
                )
                    .into_response()
            }
            Err(_) => unauthorized("refresh_invalid"),
        },
        Err(AuthError::RefreshInvalid) => unauthorized("refresh_invalid"),
        Err(AuthError::Unavailable(_)) => unavailable(),
        Err(_) => unauthorized("refresh_invalid"),
    }
}

async fn logout(State(state): State<Arc<AppState>>, jar: CookieJar) -> Response {
    let auth = &state.auth;
    let rt = extract(&jar, REFRESH_COOKIE);
    let _ = auth.logout(rt.as_deref()).await;
    let jar = clear_auth_cookies(jar, &auth.cfg);
    (jar, StatusCode::NO_CONTENT).into_response()
}

async fn me(State(state): State<Arc<AppState>>, jar: CookieJar) -> Response {
    let auth = &state.auth;
    if auth.cfg.disable_auth && auth.cfg.is_development {
        return Json(MeResponse {
            email: "dev@local".into(),
            roles: vec!["admin".into()],
        })
        .into_response();
    }
    let token = match extract(&jar, ACCESS_COOKIE) {
        Some(t) if !t.is_empty() => t,
        _ => return unauthorized("missing_token"),
    };
    match auth.validate_access(&token).await {
        Ok(AuthContext { email, roles, .. }) => Json(MeResponse { email, roles }).into_response(),
        Err(AuthError::Unavailable(_)) => unavailable(),
        Err(_) => unauthorized("invalid_token"),
    }
}

fn unauthorized(code: &'static str) -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(serde_json::json!({"error": code})),
    )
        .into_response()
}

fn unavailable() -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(serde_json::json!({"error": "auth_unavailable"})),
    )
        .into_response()
}
