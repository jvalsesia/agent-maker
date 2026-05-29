//! In-memory JWKS cache. Holds a `kid → DecodingKey` map guarded by an
//! async `RwLock`; fetches lazily (cold or on `kid` miss) and tolerates Clerk's
//! background key rotation by re-fetching before rejecting.

use crate::error::{AppError, AppResult};
use jsonwebtoken::DecodingKey;
use serde::Deserialize;
use std::collections::HashMap;
use tokio::sync::RwLock;

#[derive(Debug, Deserialize)]
struct Jwk {
    kid: String,
    n: String,
    e: String,
    #[serde(default)]
    kty: String,
}

#[derive(Debug, Deserialize)]
struct JwkSet {
    keys: Vec<Jwk>,
}

pub struct JwksCache {
    /// Clerk JWKS endpoint. Empty when the cache is seeded for tests / disabled.
    url: String,
    http: reqwest::Client,
    keys: RwLock<HashMap<String, DecodingKey>>,
}

impl JwksCache {
    pub fn new(url: String) -> Self {
        Self { url, http: reqwest::Client::new(), keys: RwLock::new(HashMap::new()) }
    }

    /// A cache pre-populated with a single key and no fetch URL (tests).
    pub fn seeded(kid: String, key: DecodingKey) -> Self {
        let mut map = HashMap::new();
        map.insert(kid, key);
        Self { url: String::new(), http: reqwest::Client::new(), keys: RwLock::new(map) }
    }

    /// Look up a decoding key by `kid`. On a miss, lazily re-fetch the JWKS once
    /// and look again. Returns `Ok(None)` when the key is genuinely absent (or no
    /// URL is configured to fetch from); returns `Err(AuthUnavailable)` when a
    /// fetch is required but fails.
    pub async fn key_for(&self, kid: &str) -> AppResult<Option<DecodingKey>> {
        if let Some(key) = self.keys.read().await.get(kid).cloned() {
            return Ok(Some(key));
        }
        if self.url.is_empty() {
            // Seeded/no-network cache: nothing to fetch.
            return Ok(None);
        }
        self.refresh().await?;
        Ok(self.keys.read().await.get(kid).cloned())
    }

    /// Fetch the JWKS and replace the cache. Errors map to `AuthUnavailable`.
    pub async fn refresh(&self) -> AppResult<()> {
        if self.url.is_empty() {
            return Err(AppError::AuthUnavailable);
        }
        let body = self
            .http
            .get(&self.url)
            .send()
            .await
            .map_err(|e| {
                tracing::warn!(error = %e, "JWKS fetch failed");
                AppError::AuthUnavailable
            })?
            .error_for_status()
            .map_err(|e| {
                tracing::warn!(error = %e, "JWKS endpoint returned an error status");
                AppError::AuthUnavailable
            })?
            .text()
            .await
            .map_err(|_| AppError::AuthUnavailable)?;

        let parsed = Self::parse(&body)?;
        *self.keys.write().await = parsed;
        Ok(())
    }

    /// Parse a JWKS document into decoding keys, keyed by `kid`. Non-RSA keys are
    /// skipped. Fails only when the JSON itself is unparseable.
    fn parse(body: &str) -> AppResult<HashMap<String, DecodingKey>> {
        let set: JwkSet = serde_json::from_str(body).map_err(|e| {
            tracing::warn!(error = %e, "JWKS document did not parse");
            AppError::AuthUnavailable
        })?;
        let mut map = HashMap::with_capacity(set.keys.len());
        for jwk in set.keys {
            if !jwk.kty.is_empty() && jwk.kty != "RSA" {
                continue;
            }
            match DecodingKey::from_rsa_components(&jwk.n, &jwk.e) {
                Ok(key) => {
                    map.insert(jwk.kid, key);
                }
                Err(e) => tracing::warn!(kid = %jwk.kid, error = %e, "skipping malformed JWK"),
            }
        }
        Ok(map)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::testkeys;

    #[tokio::test]
    async fn jwks_parses_rsa_jwk_into_key() {
        let tk = testkeys::generate("kid-1");
        let map = JwksCache::parse(&tk.jwks_json).unwrap();
        assert!(map.contains_key("kid-1"));

        let cache = JwksCache::new(String::new());
        *cache.keys.write().await = map;
        assert!(cache.key_for("kid-1").await.unwrap().is_some());
    }

    #[tokio::test]
    async fn jwks_returns_none_for_unknown_kid() {
        // Seeded cache (no URL) → a missing kid resolves to None without panicking
        // or attempting a network fetch.
        let tk = testkeys::generate("present");
        let cache = JwksCache::seeded("present".into(), tk.decoding);
        assert!(cache.key_for("absent").await.unwrap().is_none());
    }
}
