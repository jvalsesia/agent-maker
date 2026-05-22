//! Integration tests for the F03 /api/skills/* surface.

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

fn req_empty(method: &str, uri: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .body(Body::empty())
        .unwrap()
}

fn valid_skill(name: &str) -> Value {
    json!({
        "name": name,
        "description": "Keep responses tight and skimmable",
        "body": "Prefer short sentences. Avoid filler. Use bullet lists when listing.",
    })
}

#[sqlx::test(migrations = "./migrations")]
async fn create_then_list_and_get(pool: PgPool) {
    let app = build_app_with_store(pool, make_store());

    let resp = app
        .clone()
        .oneshot(req_json("POST", "/api/skills", valid_skill("Concise")))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = json_body(resp).await;
    let id = body["skill"]["id"].as_str().unwrap().to_string();
    assert_eq!(body["skill"]["name"], "Concise");
    assert!(body["warnings"].as_array().unwrap().is_empty());

    let resp = app.clone().oneshot(req_get("/api/skills")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    let skills = body["skills"].as_array().unwrap();
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0]["attached_agent_count"], 0);

    let resp = app
        .clone()
        .oneshot(req_get(&format!("/api/skills/{id}")))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["skill"]["name"], "Concise");
    assert_eq!(body["using_agents"].as_array().unwrap().len(), 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn duplicate_name_returns_validation_error(pool: PgPool) {
    let app = build_app_with_store(pool, make_store());

    let resp = app
        .clone()
        .oneshot(req_json("POST", "/api/skills", valid_skill("Dup")))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let resp = app
        .clone()
        .oneshot(req_json("POST", "/api/skills", valid_skill("Dup")))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body = json_body(resp).await;
    assert_eq!(body["error"]["code"], "validation_error");
}

#[sqlx::test(migrations = "./migrations")]
async fn update_persists_and_bumps_updated_at(pool: PgPool) {
    let app = build_app_with_store(pool, make_store());

    let resp = app
        .clone()
        .oneshot(req_json("POST", "/api/skills", valid_skill("Original")))
        .await
        .unwrap();
    let body = json_body(resp).await;
    let id = body["skill"]["id"].as_str().unwrap().to_string();
    let created_updated_at = body["skill"]["updated_at"].as_str().unwrap().to_string();

    // Sleep briefly so the timestamp can move.
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;

    let mut updated = valid_skill("Renamed");
    updated["body"] = json!("Be helpful and brief, no fluff at all in your responses.");
    let resp = app
        .clone()
        .oneshot(req_json("PUT", &format!("/api/skills/{id}"), updated))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["skill"]["name"], "Renamed");
    let new_updated_at = body["skill"]["updated_at"].as_str().unwrap();
    assert_ne!(new_updated_at, created_updated_at);
}

#[sqlx::test(migrations = "./migrations")]
async fn clone_appends_copy_suffix(pool: PgPool) {
    let app = build_app_with_store(pool, make_store());

    let resp = app
        .clone()
        .oneshot(req_json("POST", "/api/skills", valid_skill("Brief")))
        .await
        .unwrap();
    let id = json_body(resp).await["skill"]["id"].as_str().unwrap().to_string();

    let resp = app
        .clone()
        .oneshot(req_empty("POST", &format!("/api/skills/{id}/clone")))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = json_body(resp).await;
    assert_eq!(body["skill"]["name"], "Brief (copy)");

    // Second clone should suffix with (2).
    let resp = app
        .clone()
        .oneshot(req_empty("POST", &format!("/api/skills/{id}/clone")))
        .await
        .unwrap();
    let body = json_body(resp).await;
    assert_eq!(body["skill"]["name"], "Brief (copy) (2)");
}

#[sqlx::test(migrations = "./migrations")]
async fn delete_cascades_agent_skills(pool: PgPool) {
    let app = build_app_with_store(pool.clone(), make_store());

    // Create an agent and a skill, then insert agent_skills directly.
    let resp = app
        .clone()
        .oneshot(req_json(
            "POST",
            "/api/agents",
            json!({
                "name": "A1",
                "preamble": null,
                "system_prompt": "You are a sharp, opinionated writing editor. Be specific.",
                "provider": "anthropic",
                "model": "claude-haiku-4-5",
            }),
        ))
        .await
        .unwrap();
    let agent_id: uuid::Uuid = json_body(resp).await["agent"]["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();

    let resp = app
        .clone()
        .oneshot(req_json("POST", "/api/skills", valid_skill("S1")))
        .await
        .unwrap();
    let skill_id: uuid::Uuid = json_body(resp).await["skill"]["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();

    sqlx::query("INSERT INTO agent_skills (agent_id, skill_id, position) VALUES ($1, $2, 0)")
        .bind(agent_id)
        .bind(skill_id)
        .execute(&pool)
        .await
        .unwrap();

    // GET /skills/:id returns using_agents.
    let resp = app
        .clone()
        .oneshot(req_get(&format!("/api/skills/{skill_id}")))
        .await
        .unwrap();
    let body = json_body(resp).await;
    let using = body["using_agents"].as_array().unwrap();
    assert_eq!(using.len(), 1);
    assert_eq!(using[0]["name"], "A1");

    // List shows attached_agent_count = 1.
    let resp = app.clone().oneshot(req_get("/api/skills")).await.unwrap();
    let body = json_body(resp).await;
    assert_eq!(body["skills"][0]["attached_agent_count"], 1);

    // Delete the skill — agent_skills row should cascade away.
    let resp = app
        .clone()
        .oneshot(req_empty("DELETE", &format!("/api/skills/{skill_id}")))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM agent_skills WHERE skill_id = $1")
        .bind(skill_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn list_sort_by_attached_desc(pool: PgPool) {
    let app = build_app_with_store(pool.clone(), make_store());

    let resp = app
        .clone()
        .oneshot(req_json(
            "POST",
            "/api/agents",
            json!({
                "name": "A1",
                "system_prompt": "You are a sharp, opinionated writing editor. Be specific.",
                "provider": "anthropic",
                "model": "claude-haiku-4-5",
            }),
        ))
        .await
        .unwrap();
    let agent_id: uuid::Uuid = json_body(resp).await["agent"]["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();

    let mut ids = vec![];
    for name in ["A-pop", "B-cold", "C-mid"] {
        let resp = app
            .clone()
            .oneshot(req_json("POST", "/api/skills", valid_skill(name)))
            .await
            .unwrap();
        let id: uuid::Uuid = json_body(resp).await["skill"]["id"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap();
        ids.push((name, id));
    }

    // Only attach the first one.
    sqlx::query("INSERT INTO agent_skills (agent_id, skill_id, position) VALUES ($1, $2, 0)")
        .bind(agent_id)
        .bind(ids[0].1)
        .execute(&pool)
        .await
        .unwrap();

    let resp = app
        .clone()
        .oneshot(req_get("/api/skills?sort=attached&order=desc"))
        .await
        .unwrap();
    let body = json_body(resp).await;
    let skills = body["skills"].as_array().unwrap();
    assert_eq!(skills[0]["name"], "A-pop");
    assert_eq!(skills[0]["attached_agent_count"], 1);
}
