//! Validated identity carried by an authenticated request.
//!
//! [`validate_token`] performs the cryptographic check (signature against the
//! cached JWKS key, future `exp`, matching `iss`). The [`Claims`] extractor reads
//! the already-validated claims that [`super::require_auth`] inserts into request
//! extensions, so handlers never re-verify.

use super::jwks::JwksCache;
use crate::error::{AppError, AppResult};
use axum::{extract::FromRequestParts, http::request::Parts};
use jsonwebtoken::{Algorithm, Validation, errors::ErrorKind};
use serde::{Deserialize, Serialize};

/// The subset of Clerk session-token claims we rely on.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Clerk user ID — stable identity.
    pub sub: String,
    /// Clerk session ID (present on session tokens).
    #[serde(default)]
    pub sid: Option<String>,
    /// Expiry, unix seconds.
    pub exp: usize,
    /// Issuer.
    #[serde(default)]
    pub iss: Option<String>,
}

/// Validate a bearer token against the JWKS cache. Confirms signature, future
/// `exp`, and matching `iss`. Clerk session tokens carry `azp` rather than a
/// standard `aud`, so audience is not validated.
///
/// Tolerates key rotation: if the cached key fails the signature check, the JWKS
/// is re-fetched once and validation is retried before rejecting.
pub async fn validate_token(token: &str, cache: &JwksCache, issuer: &str) -> AppResult<Claims> {
    let header = jsonwebtoken::decode_header(token)
        .map_err(|_| AppError::Unauthorized("malformed token header".into()))?;
    let kid = header
        .kid
        .ok_or_else(|| AppError::Unauthorized("token missing key id".into()))?;

    let mut validation = Validation::new(Algorithm::RS256);
    validation.validate_exp = true;
    validation.validate_aud = false;
    validation.set_issuer(&[issuer]);

    let key = cache
        .key_for(&kid)
        .await?
        .ok_or_else(|| AppError::Unauthorized("unknown signing key".into()))?;

    match jsonwebtoken::decode::<Claims>(token, &key, &validation) {
        Ok(data) => Ok(data.claims),
        Err(e) if matches!(e.kind(), ErrorKind::InvalidSignature) => {
            // Possible key rotation — force a refresh and retry once.
            let _ = cache.refresh().await;
            let key = cache
                .key_for(&kid)
                .await?
                .ok_or_else(|| AppError::Unauthorized("unknown signing key".into()))?;
            jsonwebtoken::decode::<Claims>(token, &key, &validation)
                .map(|d| d.claims)
                .map_err(map_jwt_error)
        }
        Err(e) => Err(map_jwt_error(e)),
    }
}

fn map_jwt_error(e: jsonwebtoken::errors::Error) -> AppError {
    let reason = match e.kind() {
        ErrorKind::ExpiredSignature => "token expired",
        ErrorKind::InvalidIssuer => "invalid token issuer",
        ErrorKind::InvalidSignature => "invalid token signature",
        _ => "invalid token",
    };
    AppError::Unauthorized(reason.into())
}

impl<S> FromRequestParts<S> for Claims
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<Claims>()
            .cloned()
            .ok_or_else(|| AppError::Unauthorized("missing authenticated identity".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::testkeys;
    use jsonwebtoken::{EncodingKey, Header, encode};

    const ISSUER: &str = "https://clerk.example.com";

    fn sign(kid: &str, enc: &EncodingKey, claims: &serde_json::Value) -> String {
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some(kid.to_string());
        encode(&header, claims, enc).unwrap()
    }

    fn future_exp() -> usize {
        // Far future, expressed without wall-clock reads (test determinism).
        4_102_444_800 // 2100-01-01
    }

    #[tokio::test]
    async fn claims_accepts_valid_token() {
        let tk = testkeys::generate("k1");
        let cache = JwksCache::seeded(tk.kid.clone(), tk.decoding);
        let token = sign(
            &tk.kid,
            &tk.encoding,
            &serde_json::json!({ "sub": "user_123", "sid": "sess_1", "iss": ISSUER, "exp": future_exp() }),
        );
        let claims = validate_token(&token, &cache, ISSUER).await.unwrap();
        assert_eq!(claims.sub, "user_123");
        assert_eq!(claims.sid.as_deref(), Some("sess_1"));
    }

    #[tokio::test]
    async fn claims_rejects_expired_token() {
        let tk = testkeys::generate("k1");
        let cache = JwksCache::seeded(tk.kid.clone(), tk.decoding);
        let token = sign(
            &tk.kid,
            &tk.encoding,
            &serde_json::json!({ "sub": "u", "iss": ISSUER, "exp": 1_000 }),
        );
        let err = validate_token(&token, &cache, ISSUER).await.unwrap_err();
        assert!(matches!(err, AppError::Unauthorized(_)));
    }

    #[tokio::test]
    async fn claims_rejects_wrong_issuer() {
        let tk = testkeys::generate("k1");
        let cache = JwksCache::seeded(tk.kid.clone(), tk.decoding);
        let token = sign(
            &tk.kid,
            &tk.encoding,
            &serde_json::json!({ "sub": "u", "iss": "https://evil.example", "exp": future_exp() }),
        );
        let err = validate_token(&token, &cache, ISSUER).await.unwrap_err();
        assert!(matches!(err, AppError::Unauthorized(_)));
    }

    #[tokio::test]
    async fn claims_rejects_bad_signature() {
        // Cache holds key A; token signed with key B (same advertised kid).
        let a = testkeys::generate("k1");
        let b = testkeys::generate("k1");
        let cache = JwksCache::seeded(a.kid.clone(), a.decoding);
        let token = sign(
            "k1",
            &b.encoding,
            &serde_json::json!({ "sub": "u", "iss": ISSUER, "exp": future_exp() }),
        );
        let err = validate_token(&token, &cache, ISSUER).await.unwrap_err();
        assert!(matches!(err, AppError::Unauthorized(_)));
    }
}
