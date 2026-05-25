//! Integration tests for the F01/F09 /api/settings surface.
//! Real Postgres via `sqlx::test`, real FileStore in a tempdir.

use agent_maker::{
    build_app_with_store,
    secrets::{AnyStore, FileStore},
};
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::{Value, json};
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;

fn make_store() -> Arc<AnyStore> {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.keep();
    Arc::new(AnyStore::File(FileStore::open_or_create(&path).unwrap()))
}

async fn json_body(resp: axum::response::Response) -> Value {
    let bytes = to_bytes(resp.into_body(), 1 << 20).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

fn req_json(method: &str, uri: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

fn req_get(uri: &str) -> Request<Body> {
    Request::builder().uri(uri).body(Body::empty()).unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn locale_defaults_to_en(pool: PgPool) {
    let app = build_app_with_store(pool, make_store());
    let resp = app.oneshot(req_get("/api/settings")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["appearance"]["locale"], "en");
}

#[sqlx::test(migrations = "./migrations")]
async fn update_locale_persists(pool: PgPool) {
    let app = build_app_with_store(pool, make_store());

    let resp = app
        .clone()
        .oneshot(req_json("PUT", "/api/settings", json!({ "appearance": { "locale": "pt-BR" } })))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["appearance"]["locale"], "pt-BR");

    // Reload: the locale survives.
    let resp = app.oneshot(req_get("/api/settings")).await.unwrap();
    let body = json_body(resp).await;
    assert_eq!(body["appearance"]["locale"], "pt-BR");
}

#[sqlx::test(migrations = "./migrations")]
async fn update_locale_rejects_unsupported(pool: PgPool) {
    let app = build_app_with_store(pool, make_store());

    let resp = app
        .clone()
        .oneshot(req_json("PUT", "/api/settings", json!({ "appearance": { "locale": "xx-YY" } })))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // Unchanged after the rejected write.
    let resp = app.oneshot(req_get("/api/settings")).await.unwrap();
    let body = json_body(resp).await;
    assert_eq!(body["appearance"]["locale"], "en");
}
