//! Integration tests for `/api/agents/:id/conversations*` and
//! `/api/conversations/:id*`. Real Postgres via `sqlx::test`, file-backed
//! secret store via tempdir. No provider HTTP traffic.

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
use uuid::Uuid;

fn make_store() -> Arc<AnyStore> {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.keep();
    Arc::new(AnyStore::File(FileStore::open_or_create(&path).unwrap()))
}

async fn json_body(resp: axum::response::Response) -> Value {
    let bytes = to_bytes(resp.into_body(), 1 << 20).await.unwrap();
    if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    }
}

fn req_get(uri: &str) -> Request<Body> {
    Request::builder().uri(uri).body(Body::empty()).unwrap()
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
           VALUES ($1, $2, 'anthropic', 'claude-haiku-4-5')
           RETURNING id"#,
    )
    .bind("test-agent")
    .bind("you are a tester")
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn seed_message(pool: &PgPool, conv_id: Uuid, role: &str, content: &str) {
    sqlx::query(
        r#"INSERT INTO messages (conversation_id, role, content)
           VALUES ($1, $2, $3)"#,
    )
    .bind(conv_id)
    .bind(role)
    .bind(content)
    .execute(pool)
    .await
    .unwrap();
}

#[sqlx::test(migrations = "./migrations")]
async fn list_auto_creates_singleton_when_agent_empty(pool: PgPool) {
    let agent_id = seed_agent(&pool).await;
    let app = build_app_with_store(pool.clone(), make_store());

    let resp = app
        .clone()
        .oneshot(req_get(&format!("/api/agents/{agent_id}/conversations")))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    let list = body["conversations"].as_array().unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0]["title"], "New conversation");
    let first_id = list[0]["id"].as_str().unwrap().to_string();

    // Second list call must not create another row.
    let resp = app
        .oneshot(req_get(&format!("/api/agents/{agent_id}/conversations")))
        .await
        .unwrap();
    let body = json_body(resp).await;
    let list2 = body["conversations"].as_array().unwrap();
    assert_eq!(list2.len(), 1);
    assert_eq!(list2[0]["id"].as_str().unwrap(), first_id);
}

#[sqlx::test(migrations = "./migrations")]
async fn list_returns_sorted_by_last_activity_desc(pool: PgPool) {
    let agent_id = seed_agent(&pool).await;

    // Insert three conversations with explicit last_activity_at values.
    for (title, secs_ago) in [("A", 30), ("B", 10), ("C", 20)] {
        sqlx::query(
            r#"INSERT INTO conversations (agent_id, title, last_activity_at)
               VALUES ($1, $2, now() - ($3 || ' seconds')::interval)"#,
        )
        .bind(agent_id)
        .bind(title)
        .bind(secs_ago.to_string())
        .execute(&pool)
        .await
        .unwrap();
    }

    let app = build_app_with_store(pool, make_store());
    let resp = app
        .oneshot(req_get(&format!("/api/agents/{agent_id}/conversations")))
        .await
        .unwrap();
    let body = json_body(resp).await;
    let titles: Vec<String> = body["conversations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["title"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(titles, vec!["B", "C", "A"]);
}

#[sqlx::test(migrations = "./migrations")]
async fn create_default_title_and_explicit_title(pool: PgPool) {
    let agent_id = seed_agent(&pool).await;
    let app = build_app_with_store(pool, make_store());

    let resp = app
        .clone()
        .oneshot(req_json(
            "POST",
            &format!("/api/agents/{agent_id}/conversations"),
            json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = json_body(resp).await;
    assert_eq!(body["conversation"]["title"], "New conversation");

    let resp = app
        .oneshot(req_json(
            "POST",
            &format!("/api/agents/{agent_id}/conversations"),
            json!({ "title": "Research notes" }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = json_body(resp).await;
    assert_eq!(body["conversation"]["title"], "Research notes");
}

#[sqlx::test(migrations = "./migrations")]
async fn rename_updates_title_and_last_activity(pool: PgPool) {
    let agent_id = seed_agent(&pool).await;
    let conv_id: Uuid = sqlx::query_scalar(
        r#"INSERT INTO conversations (agent_id, title, last_activity_at)
           VALUES ($1, 'Old', now() - interval '1 hour')
           RETURNING id"#,
    )
    .bind(agent_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    let before: chrono::DateTime<chrono::Utc> =
        sqlx::query_scalar("SELECT last_activity_at FROM conversations WHERE id = $1")
            .bind(conv_id)
            .fetch_one(&pool)
            .await
            .unwrap();

    let app = build_app_with_store(pool.clone(), make_store());
    let resp = app
        .oneshot(req_json(
            "PATCH",
            &format!("/api/conversations/{conv_id}"),
            json!({ "title": "Renamed" }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["conversation"]["title"], "Renamed");

    let after: chrono::DateTime<chrono::Utc> =
        sqlx::query_scalar("SELECT last_activity_at FROM conversations WHERE id = $1")
            .bind(conv_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(after > before, "last_activity_at should advance on rename");
}

#[sqlx::test(migrations = "./migrations")]
async fn rename_rejects_empty_and_overlong_title(pool: PgPool) {
    let agent_id = seed_agent(&pool).await;
    let conv_id: Uuid = sqlx::query_scalar(
        r#"INSERT INTO conversations (agent_id, title) VALUES ($1, 'Original')
           RETURNING id"#,
    )
    .bind(agent_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    let app = build_app_with_store(pool, make_store());

    let resp = app
        .clone()
        .oneshot(req_json(
            "PATCH",
            &format!("/api/conversations/{conv_id}"),
            json!({ "title": "   " }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body = json_body(resp).await;
    assert_eq!(body["error"]["code"], "validation_error");
    assert_eq!(body["error"]["field"], "title");

    let overlong = "x".repeat(81);
    let resp = app
        .oneshot(req_json(
            "PATCH",
            &format!("/api/conversations/{conv_id}"),
            json!({ "title": overlong }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body = json_body(resp).await;
    assert_eq!(body["error"]["field"], "title");
}

#[sqlx::test(migrations = "./migrations")]
async fn get_returns_404_for_missing(pool: PgPool) {
    let app = build_app_with_store(pool, make_store());
    let bogus = Uuid::new_v4();
    let resp = app
        .oneshot(req_get(&format!("/api/conversations/{bogus}")))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[sqlx::test(migrations = "./migrations")]
async fn delete_cascades_messages(pool: PgPool) {
    let agent_id = seed_agent(&pool).await;
    let conv_id: Uuid = sqlx::query_scalar(
        r#"INSERT INTO conversations (agent_id, title) VALUES ($1, 'doomed')
           RETURNING id"#,
    )
    .bind(agent_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    seed_message(&pool, conv_id, "user", "hello").await;
    seed_message(&pool, conv_id, "assistant", "hi").await;

    let app = build_app_with_store(pool.clone(), make_store());
    let resp = app
        .oneshot(req_delete(&format!("/api/conversations/{conv_id}")))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let msg_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM messages WHERE conversation_id = $1")
            .bind(conv_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(msg_count, 0);
    let conv_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM conversations WHERE id = $1")
        .bind(conv_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(conv_count, 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn list_messages_orders_by_created_at_asc(pool: PgPool) {
    let agent_id = seed_agent(&pool).await;
    let conv_id: Uuid = sqlx::query_scalar(
        r#"INSERT INTO conversations (agent_id, title) VALUES ($1, 'chat')
           RETURNING id"#,
    )
    .bind(agent_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    // Insert messages with explicit timestamps to guarantee ordering.
    for (i, (role, content)) in [
        ("user", "first"),
        ("assistant", "second"),
        ("user", "third"),
    ]
    .iter()
    .enumerate()
    {
        sqlx::query(
            r#"INSERT INTO messages (conversation_id, role, content, created_at)
               VALUES ($1, $2, $3, now() + ($4 || ' seconds')::interval)"#,
        )
        .bind(conv_id)
        .bind(role)
        .bind(content)
        .bind((i as i64).to_string())
        .execute(&pool)
        .await
        .unwrap();
    }

    let app = build_app_with_store(pool, make_store());
    let resp = app
        .oneshot(req_get(&format!("/api/conversations/{conv_id}/messages")))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    let contents: Vec<String> = body["messages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["content"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(contents, vec!["first", "second", "third"]);
    assert_eq!(body["conversation"]["id"].as_str().unwrap(), conv_id.to_string());
}

#[sqlx::test(migrations = "./migrations")]
async fn list_messages_returns_404_for_missing_conversation(pool: PgPool) {
    let app = build_app_with_store(pool, make_store());
    let bogus = Uuid::new_v4();
    let resp = app
        .oneshot(req_get(&format!("/api/conversations/{bogus}/messages")))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
