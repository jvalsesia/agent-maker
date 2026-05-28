use super::{
    fusionauth::FusionAuthClient,
    jwks::JwksCache,
    model::{AuthContext, AuthError, Claims, FaTokenResponse},
};
use crate::config::AuthConfig;
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header};

#[derive(Clone)]
pub struct AuthService {
    pub cfg: AuthConfig,
    pub fa: FusionAuthClient,
    pub jwks: JwksCache,
}

impl AuthService {
    pub fn new(cfg: AuthConfig, http: reqwest::Client) -> Self {
        let fa = FusionAuthClient::new(&cfg, http.clone());
        let jwks = JwksCache::new(cfg.fusionauth_base_url.clone(), http);
        if !cfg.disable_auth {
            let warm = jwks.clone();
            tokio::spawn(async move {
                if let Err(e) = warm.refresh().await {
                    tracing::warn!(error = %e, "initial JWKS fetch failed; will retry on demand");
                }
            });
            jwks.spawn_background_refresher();
        }
        Self { cfg, fa, jwks }
    }

    pub fn for_tests() -> Self {
        let cfg = AuthConfig {
            fusionauth_base_url: "http://127.0.0.1:0".into(),
            fusionauth_tenant_id: String::new(),
            fusionauth_application_id: String::new(),
            fusionauth_client_id: String::new(),
            fusionauth_client_secret: String::new(),
            fusionauth_api_key: String::new(),
            cookie_domain: None,
            cookie_secure: false,
            disable_auth: true,
            is_development: true,
        };
        let http = reqwest::Client::new();
        let fa = FusionAuthClient::new(&cfg, http.clone());
        let jwks = JwksCache::new(cfg.fusionauth_base_url.clone(), http);
        Self { cfg, fa, jwks }
    }

    pub async fn login(&self, email: &str, password: &str) -> Result<FaTokenResponse, AuthError> {
        self.fa.login(email, password).await
    }

    pub async fn refresh(&self, refresh_token: &str) -> Result<FaTokenResponse, AuthError> {
        self.fa.refresh(refresh_token).await
    }

    pub async fn logout(&self, refresh_token: Option<&str>) -> Result<(), AuthError> {
        self.fa.logout(refresh_token).await
    }

    pub async fn validate_access(&self, token: &str) -> Result<AuthContext, AuthError> {
        let header = decode_header(token)
            .map_err(|e| AuthError::InvalidToken(format!("header: {e}")))?;
        let kid = header
            .kid
            .ok_or_else(|| AuthError::InvalidToken("missing kid".into()))?;
        let key: DecodingKey = self.jwks.get_or_refresh(&kid).await?;
        let mut validation = Validation::new(Algorithm::RS256);
        validation.validate_aud = false;
        let data = decode::<Claims>(token, &key, &validation)
            .map_err(|e| AuthError::InvalidToken(e.to_string()))?;
        Ok(AuthContext {
            sub: data.claims.sub,
            email: data.claims.email,
            roles: data.claims.roles,
        })
    }
}
