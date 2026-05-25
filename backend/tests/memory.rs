//! Integration tests for `/api/conversations/:id/memory*`.
//! Real Postgres via `sqlx::test`, file-backed secret store via tempdir,
//! OpenAI HTTP surface stubbed with mockito.

// `ENV_LOCK` deliberately serializes env-var mutation across each async test body;
// the std mutex guard is meant to be held across awaits here.
#![allow(clippy::await_holding_lock)]

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
use uuid::Uuid;

static ENV_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

fn make_store() -> Arc<AnyStore> {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.keep();
    Arc::new(AnyStore::File(Box::new(FileStore::open_or_create(&path).unwrap())))
}

async fn json_body(resp: axum::response::Response) -> Value {
    let bytes = to_bytes(resp.into_body(), 1 << 20).await.unwrap();
    if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    }
}

fn req_json(method: &str, uri: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

fn req_delete(uri: &str) -> Request<Body> {
    Request::builder()
        .method("DELETE")
        .uri(uri)
        .body(Body::empty())
        .unwrap()
}

async fn seed_agent(pool: &PgPool) -> Uuid {
    sqlx::query_scalar(
        r#"INSERT INTO agents (name, system_prompt, provider, model)
           VALUES ($1, 'be helpful', 'anthropic', 'claude-haiku-4-5')
           RETURNING id"#,
    )
    .bind(format!("test-agent-{}", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn seed_conversation(pool: &PgPool, agent_id: Uuid) -> Uuid {
    sqlx::query_scalar(
        r#"INSERT INTO conversations (agent_id, title) VALUES ($1, 'chat')
           RETURNING id"#,
    )
    .bind(agent_id)
    .fetch_one(pool)
    .await
    .unwrap()
}

/// Insert messages with monotonically increasing created_at offsets.
async fn seed_messages(pool: &PgPool, conv: Uuid, items: &[(&str, &str)]) {
    for (i, (role, content)) in items.iter().enumerate() {
        sqlx::query(
            r#"INSERT INTO messages (conversation_id, role, content, created_at)
               VALUES ($1, $2, $3, now() + ($4 || ' seconds')::interval)"#,
        )
        .bind(conv)
        .bind(*role)
        .bind(*content)
        .bind((i as i64).to_string())
        .execute(pool)
        .await
        .unwrap();
    }
}

async fn put_openai_key(app: axum::Router) {
    app.oneshot(req_json(
        "PUT",
        "/api/settings/providers/openai/key",
        json!({ "key": "sk-test-1234567890" }),
    ))
    .await
    .unwrap();
}

/// Build a mockito endpoint that returns a fixed embedding vector for every call.
async fn mock_embedding_ok(server: &mut mockito::ServerGuard, val: f32) -> mockito::Mock {
    let mut data: Vec<f32> = vec![0.0; 1536];
    data[0] = val;
    let body = json!({ "data": [{ "embedding": data }], "model": "text-embedding-3-small" });
    server
        .mock("POST", "/v1/embeddings")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(body.to_string())
        .expect_at_least(1)
        .create_async()
        .await
}

#[sqlx::test(migrations = "./migrations")]
async fn embed_pending_creates_rows_idempotently(pool: PgPool) {
    let _g = ENV_LOCK.lock().unwrap();
    let mut server = mockito::Server::new_async().await;
    let _m = mock_embedding_ok(&mut server, 0.1).await;
    unsafe { std::env::set_var("OPENAI_API_BASE", server.url()) };

    let agent = seed_agent(&pool).await;
    let conv = seed_conversation(&pool, agent).await;
    seed_messages(
        &pool,
        conv,
        &[("user", "alpha"), ("assistant", "beta"), ("user", "gamma")],
    )
    .await;

    let app = build_app_with_store(pool.clone(), make_store());
    put_openai_key(app.clone()).await;

    let resp = app
        .clone()
        .oneshot(req_json(
            "POST",
            &format!("/api/conversations/{conv}/memory/embed-pending"),
            json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["embedded"], 3);
    assert_eq!(body["skipped_already_present"], 0);
    assert_eq!(body["failed"], 0);

    // Second call: all already present.
    let resp = app
        .oneshot(req_json(
            "POST",
            &format!("/api/conversations/{conv}/memory/embed-pending"),
            json!({}),
        ))
        .await
        .unwrap();
    let body = json_body(resp).await;
    assert_eq!(body["embedded"], 0);
    assert_eq!(body["skipped_already_present"], 3);
}

#[sqlx::test(migrations = "./migrations")]
async fn embed_pending_skips_system_messages(pool: PgPool) {
    let _g = ENV_LOCK.lock().unwrap();
    let mut server = mockito::Server::new_async().await;
    let _m = mock_embedding_ok(&mut server, 0.2).await;
    unsafe { std::env::set_var("OPENAI_API_BASE", server.url()) };

    let agent = seed_agent(&pool).await;
    let conv = seed_conversation(&pool, agent).await;
    seed_messages(
        &pool,
        conv,
        &[("system", "ignore me"), ("user", "hello"), ("assistant", "hi")],
    )
    .await;

    let app = build_app_with_store(pool.clone(), make_store());
    put_openai_key(app.clone()).await;

    let resp = app
        .oneshot(req_json(
            "POST",
            &format!("/api/conversations/{conv}/memory/embed-pending"),
            json!({}),
        ))
        .await
        .unwrap();
    let body = json_body(resp).await;
    assert_eq!(body["embedded"], 2);
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM message_embeddings WHERE conversation_id = $1")
            .bind(conv)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 2);
}

#[sqlx::test(migrations = "./migrations")]
async fn embed_pending_reports_failure_count_when_provider_500s(pool: PgPool) {
    let _g = ENV_LOCK.lock().unwrap();
    let mut server = mockito::Server::new_async().await;
    let _m = server
        .mock("POST", "/v1/embeddings")
        .with_status(500)
        .with_body(r#"{"error":{"message":"boom"}}"#)
        .expect_at_least(1)
        .create_async()
        .await;
    unsafe { std::env::set_var("OPENAI_API_BASE", server.url()) };

    let agent = seed_agent(&pool).await;
    let conv = seed_conversation(&pool, agent).await;
    seed_messages(&pool, conv, &[("user", "a"), ("assistant", "b")]).await;

    let app = build_app_with_store(pool, make_store());
    put_openai_key(app.clone()).await;

    let resp = app
        .oneshot(req_json(
            "POST",
            &format!("/api/conversations/{conv}/memory/embed-pending"),
            json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["embedded"], 0);
    assert_eq!(body["failed"], 2);
}

#[sqlx::test(migrations = "./migrations")]
async fn query_returns_recent_window_only_when_k_is_zero(pool: PgPool) {
    // No env lock needed: k=0 short-circuits before any embedding call.
    let agent = seed_agent(&pool).await;
    let conv = seed_conversation(&pool, agent).await;
    seed_messages(
        &pool,
        conv,
        &[
            ("user", "m1"),
            ("assistant", "m2"),
            ("user", "m3"),
            ("assistant", "m4"),
            ("user", "m5"),
        ],
    )
    .await;

    let app = build_app_with_store(pool, make_store());
    let resp = app
        .oneshot(req_json(
            "POST",
            &format!("/api/conversations/{conv}/memory/query"),
            json!({ "query": "anything", "recent_n": 4, "top_k": 0 }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["effective_k"], 0);
    assert_eq!(body["retrieved"].as_array().unwrap().len(), 0);
    assert_eq!(body["recent"].as_array().unwrap().len(), 4);
    assert_eq!(body["degraded"], false);
}

#[sqlx::test(migrations = "./migrations")]
async fn query_returns_top_k_excluding_recent_window(pool: PgPool) {
    let _g = ENV_LOCK.lock().unwrap();
    let mut server = mockito::Server::new_async().await;
    let _m = mock_embedding_ok(&mut server, 0.5).await;
    unsafe { std::env::set_var("OPENAI_API_BASE", server.url()) };

    let agent = seed_agent(&pool).await;
    let conv = seed_conversation(&pool, agent).await;

    // 20 user/assistant messages
    let mut items: Vec<(&str, String)> = Vec::new();
    for i in 0..20 {
        let role = if i % 2 == 0 { "user" } else { "assistant" };
        items.push((role, format!("msg-{i:02}")));
    }
    for (i, (role, content)) in items.iter().enumerate() {
        sqlx::query(
            r#"INSERT INTO messages (conversation_id, role, content, created_at)
               VALUES ($1, $2, $3, now() + ($4 || ' seconds')::interval)"#,
        )
        .bind(conv)
        .bind(*role)
        .bind(content)
        .bind((i as i64).to_string())
        .execute(&pool)
        .await
        .unwrap();
    }

    let app = build_app_with_store(pool.clone(), make_store());
    put_openai_key(app.clone()).await;
    // Embed all 20.
    app.clone()
        .oneshot(req_json(
            "POST",
            &format!("/api/conversations/{conv}/memory/embed-pending"),
            json!({}),
        ))
        .await
        .unwrap();

    // Recent IDs (last 4 by created_at).
    let recent_ids: Vec<Uuid> = sqlx::query_scalar(
        r#"SELECT id FROM messages
           WHERE conversation_id = $1 AND role IN ('user','assistant')
           ORDER BY created_at DESC, id DESC LIMIT 4"#,
    )
    .bind(conv)
    .fetch_all(&pool)
    .await
    .unwrap();

    let resp = app
        .oneshot(req_json(
            "POST",
            &format!("/api/conversations/{conv}/memory/query"),
            json!({ "query": "anything", "recent_n": 4, "top_k": 3 }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["effective_n"], 4);
    assert_eq!(body["effective_k"], 3);
    let retrieved = body["retrieved"].as_array().unwrap();
    assert_eq!(retrieved.len(), 3);
    for r in retrieved {
        let id: Uuid = r["id"].as_str().unwrap().parse().unwrap();
        assert!(
            !recent_ids.contains(&id),
            "retrieved id {id} must be outside the recent-4 tail"
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn query_resolves_overrides_then_globals(pool: PgPool) {
    let agent = seed_agent(&pool).await;
    // Set agent overrides: N=6, K=2.
    sqlx::query("UPDATE agents SET recent_n_override = 6, top_k_override = 2 WHERE id = $1")
        .bind(agent)
        .execute(&pool)
        .await
        .unwrap();
    let conv = seed_conversation(&pool, agent).await;

    let app = build_app_with_store(pool, make_store());
    // No openai key -> retrieval will degrade, but we only care about effective_n/k.
    let resp = app
        .oneshot(req_json(
            "POST",
            &format!("/api/conversations/{conv}/memory/query"),
            json!({ "query": "anything" }),
        ))
        .await
        .unwrap();
    let body = json_body(resp).await;
    assert_eq!(body["effective_n"], 6);
    assert_eq!(body["effective_k"], 2);
}

#[sqlx::test(migrations = "./migrations")]
async fn query_validates_bounds(pool: PgPool) {
    let agent = seed_agent(&pool).await;
    let conv = seed_conversation(&pool, agent).await;
    let app = build_app_with_store(pool, make_store());

    for (n, k) in [(Some(2), None), (None, Some(11))] {
        let mut body = serde_json::Map::new();
        body.insert("query".into(), json!("hi"));
        if let Some(n) = n {
            body.insert("recent_n".into(), json!(n));
        }
        if let Some(k) = k {
            body.insert("top_k".into(), json!(k));
        }
        let resp = app
            .clone()
            .oneshot(req_json(
                "POST",
                &format!("/api/conversations/{conv}/memory/query"),
                Value::Object(body),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let body = json_body(resp).await;
        assert_eq!(body["error"]["code"], "validation_error");
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn query_degrades_when_no_openai_key(pool: PgPool) {
    let agent = seed_agent(&pool).await;
    let conv = seed_conversation(&pool, agent).await;
    seed_messages(&pool, conv, &[("user", "hi"), ("assistant", "hello")]).await;

    let app = build_app_with_store(pool, make_store());
    let resp = app
        .oneshot(req_json(
            "POST",
            &format!("/api/conversations/{conv}/memory/query"),
            json!({ "query": "anything", "top_k": 3 }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["degraded"], true);
    assert_eq!(body["degraded_reason"], "no_openai_key");
    assert_eq!(body["retrieved"].as_array().unwrap().len(), 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn clear_removes_embeddings_only(pool: PgPool) {
    let _g = ENV_LOCK.lock().unwrap();
    let mut server = mockito::Server::new_async().await;
    let _m = mock_embedding_ok(&mut server, 0.3).await;
    unsafe { std::env::set_var("OPENAI_API_BASE", server.url()) };

    let agent = seed_agent(&pool).await;
    let conv = seed_conversation(&pool, agent).await;
    seed_messages(
        &pool,
        conv,
        &[
            ("user", "a"),
            ("assistant", "b"),
            ("user", "c"),
            ("assistant", "d"),
            ("user", "e"),
        ],
    )
    .await;

    let app = build_app_with_store(pool.clone(), make_store());
    put_openai_key(app.clone()).await;
    app.clone()
        .oneshot(req_json(
            "POST",
            &format!("/api/conversations/{conv}/memory/embed-pending"),
            json!({}),
        ))
        .await
        .unwrap();

    let pre: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM message_embeddings WHERE conversation_id = $1")
            .bind(conv)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(pre, 5);

    let resp = app
        .oneshot(req_delete(&format!("/api/conversations/{conv}/memory")))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["removed"], 5);

    let post: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM message_embeddings WHERE conversation_id = $1")
            .bind(conv)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(post, 0);
    let msg_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM messages WHERE conversation_id = $1")
            .bind(conv)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(msg_count, 5);
}

#[sqlx::test(migrations = "./migrations")]
async fn query_404_for_missing_conversation(pool: PgPool) {
    let app = build_app_with_store(pool, make_store());
    let bogus = Uuid::new_v4();
    let resp = app
        .oneshot(req_json(
            "POST",
            &format!("/api/conversations/{bogus}/memory/query"),
            json!({ "query": "hi" }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[sqlx::test(migrations = "./migrations")]
async fn query_rejects_empty_query_string(pool: PgPool) {
    let agent = seed_agent(&pool).await;
    let conv = seed_conversation(&pool, agent).await;
    let app = build_app_with_store(pool, make_store());
    let resp = app
        .oneshot(req_json(
            "POST",
            &format!("/api/conversations/{conv}/memory/query"),
            json!({ "query": "   " }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body = json_body(resp).await;
    assert_eq!(body["error"]["code"], "validation_error");
}
