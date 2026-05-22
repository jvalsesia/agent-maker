//! Integration tests for `/api/templates/*`. Real Postgres via sqlx::test,
//! file-backed secret store via tempdir. No provider HTTP traffic.

use agent_maker::{
    build_app_with_store,
    secrets::{AnyStore, FileStore},
};
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::Value;
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
    if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    }
}

fn req_get(uri: &str) -> Request<Body> {
    Request::builder().uri(uri).body(Body::empty()).unwrap()
}

fn req_post(uri: &str) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::empty())
        .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn list_returns_ten_agents_and_ten_skills(pool: PgPool) {
    let app = build_app_with_store(pool, make_store());
    let resp = app.oneshot(req_get("/api/templates")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["agents"].as_array().unwrap().len(), 10);
    assert_eq!(body["skills"].as_array().unwrap().len(), 10);
    // First agent should be writing-editor with two suggested skills.
    let first = &body["agents"][0];
    assert_eq!(first["slug"], "writing-editor");
    assert_eq!(first["suggested_skills"].as_array().unwrap().len(), 2);
}

#[sqlx::test(migrations = "./migrations")]
async fn list_filters_by_category(pool: PgPool) {
    let app = build_app_with_store(pool, make_store());
    let resp = app
        .oneshot(req_get("/api/templates?category=writing"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    for a in body["agents"].as_array().unwrap() {
        assert_eq!(a["category"], "writing");
    }
    for s in body["skills"].as_array().unwrap() {
        assert_eq!(s["category"], "writing");
    }
    assert!(!body["agents"].as_array().unwrap().is_empty());
    assert!(!body["skills"].as_array().unwrap().is_empty());
}

#[sqlx::test(migrations = "./migrations")]
async fn preview_agent_includes_suggested_skill_descriptions(pool: PgPool) {
    let app = build_app_with_store(pool, make_store());
    let resp = app
        .oneshot(req_get("/api/templates/agents/writing-editor"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    let a = &body["agent"];
    assert_eq!(a["slug"], "writing-editor");
    assert_eq!(a["default_provider"], "anthropic");
    assert_eq!(a["default_model"], "claude-haiku-4-5");
    let suggested = a["suggested_skills"].as_array().unwrap();
    assert_eq!(suggested.len(), 2);
    assert_eq!(suggested[0]["slug"], "blunt-editor");
    assert!(suggested[0]["description"].as_str().unwrap().len() > 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn preview_skill_returns_body(pool: PgPool) {
    let app = build_app_with_store(pool, make_store());
    let resp = app
        .oneshot(req_get("/api/templates/skills/concise-replies"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["skill"]["slug"], "concise-replies");
    assert!(body["skill"]["body"].as_str().unwrap().len() > 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn unknown_slug_returns_template_not_found(pool: PgPool) {
    let app = build_app_with_store(pool, make_store());
    let resp = app
        .clone()
        .oneshot(req_get("/api/templates/agents/does-not-exist"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let body = json_body(resp).await;
    assert_eq!(body["error"]["code"], "template_not_found");

    let resp = app
        .oneshot(req_get("/api/templates/skills/does-not-exist"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let body = json_body(resp).await;
    assert_eq!(body["error"]["code"], "template_not_found");
}

#[sqlx::test(migrations = "./migrations")]
async fn adopt_agent_on_empty_workspace_creates_agent_and_skills(pool: PgPool) {
    let app = build_app_with_store(pool.clone(), make_store());
    let resp = app
        .oneshot(req_post("/api/templates/agents/writing-editor/adopt"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["agent"]["name"], "Writing Editor");
    let attached = body["attached_skill_ids"].as_array().unwrap();
    assert_eq!(attached.len(), 2);

    // verify rows in DB
    let agent_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM agents")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(agent_count, 1);
    let skill_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM skills")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(skill_count, 2);
    // attachments in declared order: blunt-editor then concise-replies
    let names: Vec<String> = sqlx::query_scalar(
        r#"SELECT s.name FROM agent_skills a
           JOIN skills s ON s.id = a.skill_id
           ORDER BY a.position"#,
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(names, vec!["Blunt Editor", "Concise Replies"]);
}

#[sqlx::test(migrations = "./migrations")]
async fn adopt_agent_reuses_existing_user_skill(pool: PgPool) {
    // Seed a user skill named exactly like one of the suggested templates.
    sqlx::query("INSERT INTO skills (name, description, body) VALUES ($1, $2, $3)")
        .bind("Blunt Editor")
        .bind("user copy")
        .bind("user body")
        .execute(&pool)
        .await
        .unwrap();

    let app = build_app_with_store(pool.clone(), make_store());
    let resp = app
        .oneshot(req_post("/api/templates/agents/writing-editor/adopt"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let skill_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM skills")
        .fetch_one(&pool)
        .await
        .unwrap();
    // Existing Blunt Editor reused + new Concise Replies adopted = 2
    assert_eq!(skill_count, 2);

    // The attached "Blunt Editor" must be the pre-existing one with body "user body".
    let body: String = sqlx::query_scalar(
        r#"SELECT s.body FROM agent_skills a
           JOIN skills s ON s.id = a.skill_id
           WHERE s.name = 'Blunt Editor'"#,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(body, "user body");
}

#[sqlx::test(migrations = "./migrations")]
async fn double_adopt_agent_suffixes_template(pool: PgPool) {
    let app = build_app_with_store(pool.clone(), make_store());

    let resp = app
        .clone()
        .oneshot(req_post("/api/templates/agents/writing-editor/adopt"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let resp = app
        .oneshot(req_post("/api/templates/agents/writing-editor/adopt"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["agent"]["name"], "Writing Editor (template)");

    let agent_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM agents")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(agent_count, 2);
}

#[sqlx::test(migrations = "./migrations")]
async fn adopt_skill_creates_single_row(pool: PgPool) {
    let app = build_app_with_store(pool.clone(), make_store());
    let resp = app
        .oneshot(req_post("/api/templates/skills/concise-replies/adopt"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["skill"]["name"], "Concise Replies");

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM skills")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
    // No agent rows created.
    let acount: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM agents")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(acount, 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn adopt_agent_unknown_slug_rolls_back(pool: PgPool) {
    let app = build_app_with_store(pool.clone(), make_store());
    let resp = app
        .oneshot(req_post("/api/templates/agents/nope/adopt"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let agent_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM agents")
        .fetch_one(&pool)
        .await
        .unwrap();
    let skill_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM skills")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(agent_count, 0);
    assert_eq!(skill_count, 0);
}
