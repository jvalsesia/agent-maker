//! `require_auth` guard layered onto the protected `/api` sub-router.
//!
//! When auth is disabled (no Clerk env) it is a pass-through that warns once.
//! When enabled it extracts the bearer token, validates it, inserts the resulting
//! [`Claims`] into request extensions, and otherwise returns `401`/`503`.

use super::{AuthState, claims};
use crate::error::AppError;
use axum::{
    extract::{Request, State},
    http::header,
    middleware::Next,
    response::Response,
};

pub async fn require_auth(
    State(auth): State<AuthState>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    if !auth.enabled() {
        auth.warn_disabled_once();
        return Ok(next.run(req).await);
    }

    let token = bearer_token(&req)
        .ok_or_else(|| AppError::Unauthorized("missing or invalid bearer token".into()))?;
    let validated = claims::validate_token(token, auth.cache(), auth.issuer()).await?;
    req.extensions_mut().insert(validated);
    Ok(next.run(req).await)
}

/// Extract the `Bearer <token>` value from the `Authorization` header.
fn bearer_token(req: &Request) -> Option<&str> {
    req.headers()
        .get(header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .map(str::trim)
        .filter(|t| !t.is_empty())
}
