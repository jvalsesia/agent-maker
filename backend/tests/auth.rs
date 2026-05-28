//! Integration tests that exercise the auth surface without a live
//! FusionAuth. They verify the request-gating behavior end-to-end:
//!
//! - With `AuthService::for_tests()` (the harness used by every
//!   `build_app_with_store` test), the middleware is in DISABLE_AUTH
//!   mode and protected routes return 200 — proving that wiring auth
//!   in front of the existing API did not regress callers.
//! - `/auth/me` reports the dev fake identity in this mode.
//! - `/api/health` is public regardless of auth state.
//!
//! Tests against a real FusionAuth (kickstart admin login, refresh
//! rotation, logout) require the docker-compose stack to be up and
//! are documented under `docs/F10-iam/verify.md`. They are not
//! wired here yet — see Phase 4 deviations in the implementation
//! report.

use agent_maker::{
    build_app_with_store,
    secrets::{AnyStore, FileStore},
};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;

fn db_url() -> String {
    std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://agentmaker:agentmaker@127.0.0.1:5432/agentmaker".into())
}

async fn pool() -> PgPool {
    let pool = sqlx::PgPool::connect(&db_url()).await.expect("connect");
    sqlx::migrate!("./migrations").run(&pool).await.expect("migrate");
    pool
}

fn store() -> Arc<AnyStore> {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.keep();
    Arc::new(AnyStore::File(Box::new(FileStore::open_or_create(&path).unwrap())))
}

#[tokio::test]
async fn health_is_public() {
    let app = build_app_with_store(pool().await, store());
    let resp = app
        .oneshot(Request::get("/api/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn protected_route_passes_with_disable_auth_dev_harness() {
    // for_tests() sets disable_auth=true + is_development=true, mirroring the
    // production DISABLE_AUTH=1 dev escape hatch. Existing protected routes
    // should remain reachable.
    let app = build_app_with_store(pool().await, store());
    let resp = app
        .oneshot(Request::get("/api/agents").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn me_reports_dev_identity_in_disable_auth_mode() {
    let app = build_app_with_store(pool().await, store());
    let resp = app
        .oneshot(Request::get("/auth/me").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["email"], "dev@local");
    assert_eq!(v["roles"][0], "admin");
}

#[tokio::test]
async fn auth_routes_namespace_returns_404_for_unknown_paths() {
    let app = build_app_with_store(pool().await, store());
    let resp = app
        .oneshot(Request::get("/auth/does-not-exist").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
