use super::model::{AuthError, FaJwks};
use jsonwebtoken::DecodingKey;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

const REFRESH_INTERVAL: Duration = Duration::from_secs(600);

#[derive(Clone)]
pub struct JwksCache {
    inner: Arc<RwLock<Inner>>,
    base_url: String,
    http: reqwest::Client,
}

struct Inner {
    keys: HashMap<String, DecodingKey>,
    last_refresh: Option<Instant>,
}

impl JwksCache {
    pub fn new(base_url: String, http: reqwest::Client) -> Self {
        Self {
            inner: Arc::new(RwLock::new(Inner {
                keys: HashMap::new(),
                last_refresh: None,
            })),
            base_url,
            http,
        }
    }

    pub async fn bootstrap(&self) -> Result<(), AuthError> {
        self.refresh().await
    }

    pub async fn refresh(&self) -> Result<(), AuthError> {
        let url = format!("{}/.well-known/jwks.json", self.base_url.trim_end_matches('/'));
        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| AuthError::Unavailable(format!("jwks fetch: {e}")))?;
        if !resp.status().is_success() {
            return Err(AuthError::Unavailable(format!(
                "jwks fetch status {}",
                resp.status()
            )));
        }
        let jwks: FaJwks = resp
            .json()
            .await
            .map_err(|e| AuthError::Internal(format!("jwks decode: {e}")))?;
        let mut keys = HashMap::with_capacity(jwks.keys.len());
        for jwk in jwks.keys {
            let key = DecodingKey::from_rsa_components(&jwk.n, &jwk.e)
                .map_err(|e| AuthError::Internal(format!("jwks key: {e}")))?;
            keys.insert(jwk.kid, key);
        }
        let mut w = self.inner.write().expect("jwks lock");
        w.keys = keys;
        w.last_refresh = Some(Instant::now());
        Ok(())
    }

    pub fn get(&self, kid: &str) -> Option<DecodingKey> {
        let r = self.inner.read().expect("jwks lock");
        r.keys.get(kid).cloned()
    }

    pub async fn get_or_refresh(&self, kid: &str) -> Result<DecodingKey, AuthError> {
        if let Some(k) = self.get(kid) {
            return Ok(k);
        }
        self.refresh().await?;
        self.get(kid).ok_or(AuthError::UnknownKid)
    }

    pub fn spawn_background_refresher(&self) {
        let this = self.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(REFRESH_INTERVAL).await;
                if let Err(e) = this.refresh().await {
                    tracing::warn!(error = %e, "jwks background refresh failed");
                }
            }
        });
    }
}
