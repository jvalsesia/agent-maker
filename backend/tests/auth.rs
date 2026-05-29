//! Integration tests for the F10 `require_auth` middleware over the real router.
//! Real Postgres via `sqlx::test`, real FileStore in a tempdir. Tokens are minted
//! against a locally generated RSA keypair seeded into an enabled `AuthState` — no
//! network, no Clerk account.

use agent_maker::{
    auth::{AuthConfig, AuthState},
    build_app_with_auth, build_app_with_store,
    secrets::{AnyStore, FileStore},
};
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, encode};
use rsa::{RsaPrivateKey, pkcs1::EncodeRsaPrivateKey, traits::PublicKeyParts};
use serde_json::{Value, json};
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;

const ISSUER: &str = "https://clerk.test.example";
const KID: &str = "test-key-1";

fn make_store() -> Arc<AnyStore> {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.keep();
    Arc::new(AnyStore::File(Box::new(FileStore::open_or_create(&path).unwrap())))
}

/// Generate a fresh RSA keypair: returns the signing key plus the matching
/// public decoding key.
fn keypair() -> (EncodingKey, DecodingKey) {
    let mut rng = rand::thread_rng();
    let priv_key = RsaPrivateKey::new(&mut rng, 2048).unwrap();
    let der = priv_key.to_pkcs1_der().unwrap();
    let encoding = EncodingKey::from_rsa_der(der.as_bytes());
    let n = URL_SAFE_NO_PAD.encode(priv_key.n().to_bytes_be());
    let e = URL_SAFE_NO_PAD.encode(priv_key.e().to_bytes_be());
    let decoding = DecodingKey::from_rsa_components(&n, &e).unwrap();
    (encoding, decoding)
}

fn sign(enc: &EncodingKey, claims: &Value) -> String {
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(KID.to_string());
    encode(&header, claims, enc).unwrap()
}

fn valid_claims() -> Value {
    json!({ "sub": "user_abc", "sid": "sess_1", "iss": ISSUER, "exp": 4_102_444_800usize })
}

async fn json_body(resp: axum::response::Response) -> Value {
    let bytes = to_bytes(resp.into_body(), 1 << 20).await.unwrap();
    if bytes.is_empty() {
        return Value::Null;
    }
    serde_json::from_slice(&bytes).unwrap()
}

fn get(uri: &str) -> Request<Body> {
    Request::builder().uri(uri).body(Body::empty()).unwrap()
}

fn get_auth(uri: &str, token: &str) -> Request<Body> {
    Request::builder()
        .uri(uri)
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn protected_route_401_without_token(pool: PgPool) {
    let (_enc, dec) = keypair();
    let auth = AuthState::seeded(ISSUER, KID, dec);
    let app = build_app_with_auth(pool, make_store(), auth);

    let resp = app.oneshot(get("/api/agents")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let body = json_body(resp).await;
    assert_eq!(body["error"]["code"], "unauthorized");
}

#[sqlx::test(migrations = "./migrations")]
async fn protected_route_200_with_valid_token(pool: PgPool) {
    let (enc, dec) = keypair();
    let auth = AuthState::seeded(ISSUER, KID, dec);
    let app = build_app_with_auth(pool, make_store(), auth);

    let token = sign(&enc, &valid_claims());
    let resp = app.oneshot(get_auth("/api/agents", &token)).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[sqlx::test(migrations = "./migrations")]
async fn health_is_public_when_auth_enabled(pool: PgPool) {
    let (_enc, dec) = keypair();
    let auth = AuthState::seeded(ISSUER, KID, dec);
    let app = build_app_with_auth(pool, make_store(), auth);

    let resp = app.oneshot(get("/api/health")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["status"], "ok");
}

#[sqlx::test(migrations = "./migrations")]
async fn disabled_mode_allows_unauthenticated(pool: PgPool) {
    // No Clerk env → pass-through middleware (regression guard for the existing suite).
    let app = build_app_with_store(pool, make_store());
    let resp = app.oneshot(get("/api/agents")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[sqlx::test(migrations = "./migrations")]
async fn jwks_unavailable_returns_503(pool: PgPool) {
    // Enabled, but the JWKS endpoint is unreachable and the cache is cold.
    let auth = AuthState::new(AuthConfig {
        jwks_url: Some("http://127.0.0.1:1/.well-known/jwks.json".to_string()),
        issuer: Some(ISSUER.to_string()),
    });
    let app = build_app_with_auth(pool, make_store(), auth);

    // A well-formed token (any key) carrying a kid forces a JWKS fetch, which fails.
    let (enc, _dec) = keypair();
    let token = sign(&enc, &valid_claims());
    let resp = app.oneshot(get_auth("/api/agents", &token)).await.unwrap();
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body = json_body(resp).await;
    assert_eq!(body["error"]["code"], "auth_unavailable");
}
