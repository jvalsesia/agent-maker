use axum::{Json, http::StatusCode, response::IntoResponse};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("validation error: {message}")]
    Validation { message: String, field: Option<String> },

    #[error("provider error: {message}")]
    Provider {
        provider: String,
        code: String,
        message: String,
        status: StatusCode,
    },

    #[error("not found: {0}")]
    NotFound(String),

    #[error("template not found: {0}")]
    TemplateNotFound(String),

    #[error("confirmation required")]
    ConfirmationRequired,

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("context too large: {0}")]
    ContextTooLarge(String),

    #[error("{0}")]
    Unauthorized(String),

    #[error("authentication is temporarily unavailable")]
    AuthUnavailable,

    #[error("key store error: {0}")]
    KeyStore(String),

    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("internal: {0}")]
    Internal(String),
}

impl AppError {
    pub fn validation<S: Into<String>>(message: S) -> Self {
        Self::Validation { message: message.into(), field: None }
    }
    pub fn validation_field<S: Into<String>, F: Into<String>>(field: F, message: S) -> Self {
        Self::Validation { message: message.into(), field: Some(field.into()) }
    }

    fn code(&self) -> String {
        match self {
            AppError::Validation { .. } => "validation_error".into(),
            AppError::Provider { code, .. } => code.clone(),
            AppError::NotFound(_) => "not_found".into(),
            AppError::TemplateNotFound(_) => "template_not_found".into(),
            AppError::ConfirmationRequired => "confirmation_required".into(),
            AppError::Conflict(_) => "conflict".into(),
            AppError::ContextTooLarge(_) => "context_too_large".into(),
            AppError::Unauthorized(_) => "unauthorized".into(),
            AppError::AuthUnavailable => "auth_unavailable".into(),
            AppError::KeyStore(_) => "key_write_failed".into(),
            AppError::Database(_) => "database_error".into(),
            AppError::Io(_) => "io_error".into(),
            AppError::Internal(_) => "internal_error".into(),
        }
    }

    fn status(&self) -> StatusCode {
        match self {
            AppError::Validation { .. } | AppError::ConfirmationRequired => StatusCode::BAD_REQUEST,
            AppError::Provider { status, .. } => *status,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::TemplateNotFound(_) => StatusCode::NOT_FOUND,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::ContextTooLarge(_) => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            AppError::AuthUnavailable => StatusCode::SERVICE_UNAVAILABLE,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[derive(Serialize)]
struct ErrorBody<'a> {
    error: ErrorInner<'a>,
}

#[derive(Serialize)]
struct ErrorInner<'a> {
    code: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    provider: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    field: Option<&'a str>,
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let code = self.code();
        tracing::warn!(error = %self, code = %code, "request failed");
        let status = self.status();
        let (provider, field) = match &self {
            AppError::Provider { provider, .. } => (Some(provider.as_str()), None),
            AppError::Validation { field, .. } => (None, field.as_deref()),
            _ => (None, None),
        };
        let body = ErrorBody {
            error: ErrorInner { code, message: self.to_string(), provider, field },
        };
        (status, Json(body)).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;
