//! Clerk-backed authentication: pure JWT verification with an in-memory JWKS
//! cache. No vendor SDK — see `auth-intent.md`.
//!
//! Auth is **enforced iff** both `CLERK_JWKS_URL` and `CLERK_ISSUER` are set
//! (`AuthState::enabled()`). With both unset the [`require_auth`] middleware is
//! a pass-through, preserving local dev and the integration test suite. The
//! "exactly one set" misconfiguration is rejected at startup in `config.rs`.

pub mod claims;
pub mod jwks;
pub mod middleware;

pub use claims::Claims;
pub use middleware::require_auth;

use jwks::JwksCache;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

/// Clerk auth configuration, sourced from `Config`.
#[derive(Debug, Clone, Default)]
pub struct AuthConfig {
    pub jwks_url: Option<String>,
    pub issuer: Option<String>,
}

/// Shared, cheaply cloneable authentication state placed in `AppState`.
#[derive(Clone)]
pub struct AuthState {
    inner: Arc<Inner>,
}

struct Inner {
    enabled: bool,
    issuer: String,
    cache: JwksCache,
    warned: AtomicBool,
}

impl AuthState {
    /// Build from configuration. Enabled only when both the JWKS URL and issuer
    /// are present.
    pub fn new(cfg: AuthConfig) -> Self {
        let enabled = cfg.jwks_url.is_some() && cfg.issuer.is_some();
        let cache = JwksCache::new(cfg.jwks_url.unwrap_or_default());
        Self {
            inner: Arc::new(Inner {
                enabled,
                issuer: cfg.issuer.unwrap_or_default(),
                cache,
                warned: AtomicBool::new(false),
            }),
        }
    }

    /// A disabled (pass-through) auth state — used by local dev and the existing
    /// test suite when no Clerk environment is configured.
    pub fn disabled() -> Self {
        Self::new(AuthConfig::default())
    }

    /// Test-only constructor: an enabled state whose JWKS cache is pre-seeded
    /// with a single key (no network), for integration tests that mint their own
    /// tokens against a locally generated keypair.
    pub fn seeded(
        issuer: impl Into<String>,
        kid: impl Into<String>,
        key: jsonwebtoken::DecodingKey,
    ) -> Self {
        Self {
            inner: Arc::new(Inner {
                enabled: true,
                issuer: issuer.into(),
                cache: JwksCache::seeded(kid.into(), key),
                warned: AtomicBool::new(false),
            }),
        }
    }

    pub fn enabled(&self) -> bool {
        self.inner.enabled
    }

    pub(crate) fn issuer(&self) -> &str {
        &self.inner.issuer
    }

    pub(crate) fn cache(&self) -> &JwksCache {
        &self.inner.cache
    }

    /// Emit a single warning the first time a request flows through disabled auth.
    pub(crate) fn warn_disabled_once(&self) {
        if !self.inner.warned.swap(true, Ordering::Relaxed) {
            tracing::warn!(
                "authentication is DISABLED (CLERK_JWKS_URL/CLERK_ISSUER unset); \
                 all /api requests are served unauthenticated"
            );
        }
    }
}

/// Test-only RSA keypair generation shared by the auth unit tests. Generates a
/// fresh 2048-bit key and returns the signing key plus a JWKS document advertising
/// the matching public key.
#[cfg(test)]
pub(crate) mod testkeys {
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    use jsonwebtoken::{DecodingKey, EncodingKey};
    use rsa::{RsaPrivateKey, pkcs1::EncodeRsaPrivateKey, traits::PublicKeyParts};

    pub struct TestKey {
        pub kid: String,
        pub encoding: EncodingKey,
        pub decoding: DecodingKey,
        pub jwks_json: String,
    }

    pub fn generate(kid: &str) -> TestKey {
        let mut rng = rand::thread_rng();
        let priv_key = RsaPrivateKey::new(&mut rng, 2048).expect("generate rsa key");
        let der = priv_key.to_pkcs1_der().expect("encode pkcs1 der");
        let encoding = EncodingKey::from_rsa_der(der.as_bytes());

        let n = URL_SAFE_NO_PAD.encode(priv_key.n().to_bytes_be());
        let e = URL_SAFE_NO_PAD.encode(priv_key.e().to_bytes_be());
        let decoding = DecodingKey::from_rsa_components(&n, &e).expect("build decoding key");
        let jwks_json = format!(
            r#"{{"keys":[{{"kty":"RSA","use":"sig","alg":"RS256","kid":"{kid}","n":"{n}","e":"{e}"}}]}}"#
        );

        TestKey { kid: kid.to_string(), encoding, decoding, jwks_json }
    }
}
