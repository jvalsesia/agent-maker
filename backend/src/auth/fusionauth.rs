use super::model::{AuthError, FaTokenResponse};
use crate::config::AuthConfig;
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct FusionAuthClient {
    http: reqwest::Client,
    base_url: String,
    application_id: String,
    api_key: String,
}

impl FusionAuthClient {
    pub fn new(cfg: &AuthConfig, http: reqwest::Client) -> Self {
        Self {
            http,
            base_url: cfg.fusionauth_base_url.trim_end_matches('/').to_string(),
            application_id: cfg.fusionauth_application_id.clone(),
            api_key: cfg.fusionauth_api_key.clone(),
        }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub async fn login(&self, email: &str, password: &str) -> Result<FaTokenResponse, AuthError> {
        #[derive(Serialize)]
        struct Body<'a> {
            #[serde(rename = "loginId")]
            login_id: &'a str,
            password: &'a str,
            #[serde(rename = "applicationId")]
            application_id: &'a str,
        }
        let url = format!("{}/api/login", self.base_url);
        let resp = self
            .http
            .post(&url)
            .header("Authorization", &self.api_key)
            .json(&Body {
                login_id: email,
                password,
                application_id: &self.application_id,
            })
            .send()
            .await
            .map_err(|e| AuthError::Unavailable(format!("login transport: {e}")))?;
        let status = resp.status();
        if status == reqwest::StatusCode::OK || status == reqwest::StatusCode::ACCEPTED {
            #[derive(Deserialize)]
            struct Wrap {
                token: String,
                #[serde(rename = "refreshToken")]
                refresh_token: Option<String>,
            }
            let body: Wrap = resp
                .json()
                .await
                .map_err(|e| AuthError::Internal(format!("login decode: {e}")))?;
            Ok(FaTokenResponse {
                token: body.token,
                refresh_token: body.refresh_token,
            })
        } else if status.is_client_error() {
            Err(AuthError::InvalidCredentials)
        } else {
            Err(AuthError::Unavailable(format!("login status {status}")))
        }
    }

    pub async fn refresh(&self, refresh_token: &str) -> Result<FaTokenResponse, AuthError> {
        #[derive(Serialize)]
        struct Body<'a> {
            #[serde(rename = "refreshToken")]
            refresh_token: &'a str,
        }
        let url = format!("{}/api/jwt/refresh", self.base_url);
        let resp = self
            .http
            .post(&url)
            .header("Authorization", &self.api_key)
            .json(&Body { refresh_token })
            .send()
            .await
            .map_err(|e| AuthError::Unavailable(format!("refresh transport: {e}")))?;
        let status = resp.status();
        if status == reqwest::StatusCode::OK {
            #[derive(Deserialize)]
            struct Wrap {
                token: String,
                #[serde(rename = "refreshToken")]
                refresh_token: Option<String>,
            }
            let body: Wrap = resp
                .json()
                .await
                .map_err(|e| AuthError::Internal(format!("refresh decode: {e}")))?;
            Ok(FaTokenResponse {
                token: body.token,
                refresh_token: body.refresh_token,
            })
        } else if status.is_client_error() {
            Err(AuthError::RefreshInvalid)
        } else {
            Err(AuthError::Unavailable(format!("refresh status {status}")))
        }
    }

    pub async fn logout(&self, refresh_token: Option<&str>) -> Result<(), AuthError> {
        let url = match refresh_token {
            Some(rt) => format!("{}/api/logout?refreshToken={rt}", self.base_url),
            None => format!("{}/api/logout", self.base_url),
        };
        // Best-effort: ignore status code; FusionAuth returns 200 even when
        // the refresh token is unknown.
        let _ = self
            .http
            .post(&url)
            .header("Authorization", &self.api_key)
            .send()
            .await;
        Ok(())
    }
}
