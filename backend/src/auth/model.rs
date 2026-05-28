use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AuthContext {
    pub sub: Uuid,
    pub email: String,
    pub roles: Vec<String>,
}

impl AuthContext {
    pub fn is_admin(&self) -> bool {
        self.roles.iter().any(|r| r == "admin")
    }
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct MeResponse {
    pub email: String,
    pub roles: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub email: String,
    #[serde(default)]
    pub roles: Vec<String>,
    pub exp: i64,
}

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("missing access token cookie")]
    MissingCookie,
    #[error("missing refresh token cookie")]
    MissingRefresh,
    #[error("invalid token: {0}")]
    InvalidToken(String),
    #[error("unknown signing key id")]
    UnknownKid,
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("refresh invalid")]
    RefreshInvalid,
    #[error("forbidden")]
    Forbidden,
    #[error("authentication service unavailable: {0}")]
    Unavailable(String),
    #[error("internal auth error: {0}")]
    Internal(String),
}

#[derive(Debug, Deserialize)]
pub struct FaTokenResponse {
    pub token: String,
    #[serde(rename = "refreshToken")]
    pub refresh_token: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct FaJwks {
    pub keys: Vec<FaJwk>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct FaJwk {
    pub kid: String,
    pub n: String,
    pub e: String,
}
