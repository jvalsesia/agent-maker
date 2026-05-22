//! End-to-end HTTP tests against the in-process Axum app, exercising real Postgres
//! (per-test database via `sqlx::test`) and a real `FileStore` (per-test tempdir).
//! Provider HTTP calls are stubbed with mockito.

use agent_maker::{
    build_app_with_store,
    secrets::{AnyStore, FileStore},
};
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use once_cell::sync::Lazy;
use serde_json::{Value, json};
use sqlx::PgPool;
use std::sync::{Arc, Mutex};
use tower::ServiceExt;

// Serializes tests that mutate process-wide environment variables
// (ANTHROPIC_API_BASE / OPENAI_API_BASE).
static ENV_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

fn make_store() -> Arc<AnyStore> {
    let dir = tempfile::tempdir().unwrap();
    // Intentionally leak: lives for the duration of the test process; rust will clean up tempdir
    // when the binary exits.
    let path = dir.keep();
    Arc::new(AnyStore::File(FileStore::open_or_create(&path).unwrap()))
}

async fn json_body(resp: axum::response::Response) -> Value {
    let bytes = to_bytes(resp.into_body(), 1 << 20).await.unwrap();
    if bytes.is_empty() {
        return Value::Null;
    }
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

fn req_delete(uri: &str) -> Request<Body> {
    Request::builder()
        .method("DELETE")
        .uri(uri)
        .body(Body::empty())
        .unwrap()
}

// ---- health ----

#[sqlx::test(migrations = "./migrations")]
async fn test_health_after_migrations(pool: PgPool) {
    let app = build_app_with_store(pool, make_store());
    let resp = app.oneshot(req_get("/api/health")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["status"], "ok");
    assert_eq!(body["db"], "ok");
}

// ---- settings defaults ----

#[sqlx::test(migrations = "./migrations")]
async fn test_settings_default_state(pool: PgPool) {
    let app = build_app_with_store(pool, make_store());
    let resp = app.oneshot(req_get("/api/settings")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["default_provider"], "anthropic");
    assert_eq!(body["memory_defaults"]["recent_n"], 10);
    assert_eq!(body["memory_defaults"]["top_k"], 5);
    assert_eq!(body["appearance"]["theme"], "system");
    let providers = body["providers"].as_array().unwrap();
    assert_eq!(providers.len(), 3);
    for p in providers {
        assert_eq!(p["key_configured"], false);
    }
}

// ---- settings PUT ----

#[sqlx::test(migrations = "./migrations")]
async fn test_settings_put_partial(pool: PgPool) {
    let app = build_app_with_store(pool, make_store());

    let resp = app
        .clone()
        .oneshot(req_json(
            "PUT",
            "/api/settings",
            json!({ "appearance": { "theme": "dark" } }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let resp = app.oneshot(req_get("/api/settings")).await.unwrap();
    let body = json_body(resp).await;
    assert_eq!(body["appearance"]["theme"], "dark");
    // unrelated fields unchanged
    assert_eq!(body["memory_defaults"]["recent_n"], 10);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_settings_put_validation(pool: PgPool) {
    let app = build_app_with_store(pool, make_store());
    let resp = app
        .oneshot(req_json(
            "PUT",
            "/api/settings",
            json!({ "memory_defaults": { "recent_n": 2 } }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body = json_body(resp).await;
    assert_eq!(body["error"]["code"], "validation_error");
    assert_eq!(body["error"]["field"], "memory_defaults.recent_n");
}

// ---- provider keys ----

#[sqlx::test(migrations = "./migrations")]
async fn test_provider_key_save_then_get(pool: PgPool) {
    let app = build_app_with_store(pool, make_store());

    let resp = app
        .clone()
        .oneshot(req_json(
            "PUT",
            "/api/settings/providers/anthropic/key",
            json!({ "key": "sk-ant-api03-XYZ123ABC" }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let resp = app.oneshot(req_get("/api/settings")).await.unwrap();
    let body = json_body(resp).await;
    let providers = body["providers"].as_array().unwrap();
    let anthropic = providers.iter().find(|p| p["name"] == "anthropic").unwrap();
    assert_eq!(anthropic["key_configured"], true);
    assert_eq!(anthropic["key_masked"], "sk-ant-***...3ABC");
    // raw key never echoed back
    let blob = body.to_string();
    assert!(!blob.contains("sk-ant-api03-XYZ123ABC"), "raw key leaked: {blob}");
}

#[sqlx::test(migrations = "./migrations")]
async fn test_provider_key_delete(pool: PgPool) {
    let app = build_app_with_store(pool, make_store());
    app.clone()
        .oneshot(req_json(
            "PUT",
            "/api/settings/providers/openai/key",
            json!({ "key": "sk-openai-ABCDE12345" }),
        ))
        .await
        .unwrap();

    let resp = app
        .clone()
        .oneshot(req_delete("/api/settings/providers/openai/key"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let resp = app.oneshot(req_get("/api/settings")).await.unwrap();
    let body = json_body(resp).await;
    let openai = body["providers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["name"] == "openai")
        .unwrap();
    assert_eq!(openai["key_configured"], false);
}

// ---- provider test endpoint (mockito) ----

#[sqlx::test(migrations = "./migrations")]
async fn test_provider_test_invalid_key(pool: PgPool) {
    let _g = ENV_LOCK.lock().unwrap();
    let mut server = mockito::Server::new_async().await;
    let m = server
        .mock("POST", "/v1/messages")
        .with_status(401)
        .with_body(r#"{"error":{"message":"invalid x-api-key"}}"#)
        .create_async()
        .await;
    // Safety: the global ENV_LOCK above serializes any test that touches this env var.
    unsafe { std::env::set_var("ANTHROPIC_API_BASE", server.url()) };

    let app = build_app_with_store(pool, make_store());
    app.clone()
        .oneshot(req_json(
            "PUT",
            "/api/settings/providers/anthropic/key",
            json!({ "key": "sk-ant-BOGUS-00000000" }),
        ))
        .await
        .unwrap();
    let resp = app
        .oneshot(req_json(
            "POST",
            "/api/settings/providers/anthropic/test",
            json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body = json_body(resp).await;
    assert_eq!(body["error"]["code"], "invalid_api_key");
    assert_eq!(body["error"]["provider"], "anthropic");
    m.assert_async().await;
}

#[sqlx::test(migrations = "./migrations")]
async fn test_provider_test_success(pool: PgPool) {
    let _g = ENV_LOCK.lock().unwrap();
    let mut server = mockito::Server::new_async().await;
    let m = server
        .mock("POST", "/v1/messages")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"id":"msg_1","content":[{"type":"text","text":"ok"}]}"#)
        .create_async()
        .await;
    unsafe { std::env::set_var("ANTHROPIC_API_BASE", server.url()) };

    let app = build_app_with_store(pool, make_store());
    app.clone()
        .oneshot(req_json(
            "PUT",
            "/api/settings/providers/anthropic/key",
            json!({ "key": "sk-ant-LIVE-12345678" }),
        ))
        .await
        .unwrap();
    let resp = app
        .oneshot(req_json(
            "POST",
            "/api/settings/providers/anthropic/test",
            json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["ok"], true);
    assert_eq!(body["model_used"], "claude-haiku-4-5");
    assert!(body["latency_ms"].as_u64().is_some());
    m.assert_async().await;
}

// ---- wipe ----

#[sqlx::test(migrations = "./migrations")]
async fn test_wipe_clears_db_and_secrets(pool: PgPool) {
    let store = make_store();
    let app = build_app_with_store(pool.clone(), store.clone());

    // seed: save a key, change settings, insert an agent row
    app.clone()
        .oneshot(req_json(
            "PUT",
            "/api/settings/providers/anthropic/key",
            json!({ "key": "sk-ant-XXXXYYYY9999" }),
        ))
        .await
        .unwrap();
    app.clone()
        .oneshot(req_json(
            "PUT",
            "/api/settings",
            json!({ "appearance": { "theme": "dark" } }),
        ))
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO agents (name, system_prompt, provider, model) VALUES ($1, $2, 'anthropic', 'claude-haiku-4-5')",
    )
    .bind("test-agent")
    .bind("you are a tester")
    .execute(&pool)
    .await
    .unwrap();

    // wipe missing confirmation -> 400
    let resp = app
        .clone()
        .oneshot(req_json(
            "POST",
            "/api/settings/data/wipe",
            json!({ "confirm": "nope" }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // wipe with confirmation
    let resp = app
        .clone()
        .oneshot(req_json(
            "POST",
            "/api/settings/data/wipe",
            json!({ "confirm": "WIPE" }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // settings reset, providers cleared, agents truncated
    let resp = app.oneshot(req_get("/api/settings")).await.unwrap();
    let body = json_body(resp).await;
    assert_eq!(body["appearance"]["theme"], "system");
    for p in body["providers"].as_array().unwrap() {
        assert_eq!(p["key_configured"], false);
    }
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM agents")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);

    // secret store has been cleared
    use agent_maker::secrets::{SecretError, SecretStore};
    assert!(matches!(
        store.get("anthropic").await,
        Err(SecretError::NotFound)
    ));
}

// ---- pgvector ----

#[sqlx::test(migrations = "./migrations")]
async fn test_pgvector_extension_loaded(pool: PgPool) {
    let row: (String,) = sqlx::query_as("SELECT extname FROM pg_extension WHERE extname='vector'")
        .fetch_one(&pool)
        .await
        .expect("vector extension must be loaded");
    assert_eq!(row.0, "vector");
}
