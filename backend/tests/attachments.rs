//! Integration tests for the F04 /api/agents/:id/skills* and /api/agents/:id/compose surface.

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
    Arc::new(AnyStore::File(Box::new(FileStore::open_or_create(&path).unwrap())))
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

fn agent_payload(name: &str) -> Value {
    json!({
        "name": name,
        "preamble": null,
        "system_prompt": "You are a sharp, opinionated writing editor. Be specific and direct.",
        "provider": "anthropic",
        "model": "claude-haiku-4-5",
    })
}

fn skill_payload(name: &str, body: &str) -> Value {
    json!({
        "name": name,
        "description": "Test skill description",
        "body": body,
    })
}

async fn create_agent(app: &axum::Router, name: &str) -> uuid::Uuid {
    let resp = app
        .clone()
        .oneshot(req_json("POST", "/api/agents", agent_payload(name)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    json_body(resp).await["agent"]["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap()
}

async fn create_skill(app: &axum::Router, name: &str, body: &str) -> uuid::Uuid {
    let resp = app
        .clone()
        .oneshot(req_json("POST", "/api/skills", skill_payload(name, body)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    json_body(resp).await["skill"]["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn attach_list_reorder_detach_lifecycle(pool: PgPool) {
    let app = build_app_with_store(pool, make_store());
    let agent_id = create_agent(&app, "A1").await;
    let s1 = create_skill(&app, "S1", "Prefer short sentences. Be concise always.").await;
    let s2 = create_skill(&app, "S2", "Cite sources whenever you make a factual claim.").await;

    // Empty list.
    let resp = app
        .clone()
        .oneshot(req_get(&format!("/api/agents/{agent_id}/skills")))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert!(body["attached"].as_array().unwrap().is_empty());

    // Attach both, S1 then S2.
    let resp = app
        .clone()
        .oneshot(req_json(
            "PUT",
            &format!("/api/agents/{agent_id}/skills"),
            json!({ "skill_ids": [s1, s2] }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    let attached = body["attached"].as_array().unwrap();
    assert_eq!(attached.len(), 2);
    assert_eq!(attached[0]["skill_id"], s1.to_string());
    assert_eq!(attached[0]["position"], 0);
    assert_eq!(attached[1]["skill_id"], s2.to_string());
    assert_eq!(attached[1]["position"], 1);
    assert!(body["warnings"].as_array().unwrap().is_empty());

    // Reorder: S2 then S1.
    let resp = app
        .clone()
        .oneshot(req_json(
            "PUT",
            &format!("/api/agents/{agent_id}/skills"),
            json!({ "skill_ids": [s2, s1] }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    let attached = body["attached"].as_array().unwrap();
    assert_eq!(attached[0]["skill_id"], s2.to_string());
    assert_eq!(attached[0]["position"], 0);
    assert_eq!(attached[1]["skill_id"], s1.to_string());
    assert_eq!(attached[1]["position"], 1);

    // Detach S2; S1 should be renumbered to position 0.
    let resp = app
        .clone()
        .oneshot(req_empty(
            "DELETE",
            &format!("/api/agents/{agent_id}/skills/{s2}"),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let resp = app
        .clone()
        .oneshot(req_get(&format!("/api/agents/{agent_id}/skills")))
        .await
        .unwrap();
    let body = json_body(resp).await;
    let attached = body["attached"].as_array().unwrap();
    assert_eq!(attached.len(), 1);
    assert_eq!(attached[0]["skill_id"], s1.to_string());
    assert_eq!(attached[0]["position"], 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn replace_with_more_than_twenty_skills_fails(pool: PgPool) {
    let app = build_app_with_store(pool, make_store());
    let agent_id = create_agent(&app, "A1").await;

    let mut ids = Vec::new();
    for i in 0..21 {
        ids.push(create_skill(&app, &format!("S{i}"), "Be helpful and brief always.").await);
    }

    let resp = app
        .clone()
        .oneshot(req_json(
            "PUT",
            &format!("/api/agents/{agent_id}/skills"),
            json!({ "skill_ids": ids }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body = json_body(resp).await;
    assert_eq!(body["error"]["code"], "validation_error");
    assert_eq!(body["error"]["field"], "skill_ids");
}

#[sqlx::test(migrations = "./migrations")]
async fn replace_with_duplicate_skill_ids_fails(pool: PgPool) {
    let app = build_app_with_store(pool, make_store());
    let agent_id = create_agent(&app, "A1").await;
    let s1 = create_skill(&app, "S1", "Prefer short sentences. Be concise.").await;

    let resp = app
        .clone()
        .oneshot(req_json(
            "PUT",
            &format!("/api/agents/{agent_id}/skills"),
            json!({ "skill_ids": [s1, s1] }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body = json_body(resp).await;
    assert_eq!(body["error"]["code"], "validation_error");
}

#[sqlx::test(migrations = "./migrations")]
async fn replace_with_unknown_skill_fails(pool: PgPool) {
    let app = build_app_with_store(pool, make_store());
    let agent_id = create_agent(&app, "A1").await;

    let bogus = uuid::Uuid::new_v4();
    let resp = app
        .clone()
        .oneshot(req_json(
            "PUT",
            &format!("/api/agents/{agent_id}/skills"),
            json!({ "skill_ids": [bogus] }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body = json_body(resp).await;
    assert_eq!(body["error"]["code"], "validation_error");
}

#[sqlx::test(migrations = "./migrations")]
async fn replace_against_missing_agent_returns_404(pool: PgPool) {
    let app = build_app_with_store(pool, make_store());
    let bogus_agent = uuid::Uuid::new_v4();

    let resp = app
        .clone()
        .oneshot(req_json(
            "PUT",
            &format!("/api/agents/{bogus_agent}/skills"),
            json!({ "skill_ids": [] }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[sqlx::test(migrations = "./migrations")]
async fn detach_unattached_skill_returns_404(pool: PgPool) {
    let app = build_app_with_store(pool, make_store());
    let agent_id = create_agent(&app, "A1").await;
    let s1 = create_skill(&app, "S1", "Prefer short sentences. Be concise.").await;

    let resp = app
        .clone()
        .oneshot(req_empty(
            "DELETE",
            &format!("/api/agents/{agent_id}/skills/{s1}"),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[sqlx::test(migrations = "./migrations")]
async fn deleting_agent_cascades_attachments(pool: PgPool) {
    let app = build_app_with_store(pool.clone(), make_store());
    let agent_id = create_agent(&app, "A1").await;
    let s1 = create_skill(&app, "S1", "Prefer short sentences. Be concise.").await;

    app.clone()
        .oneshot(req_json(
            "PUT",
            &format!("/api/agents/{agent_id}/skills"),
            json!({ "skill_ids": [s1] }),
        ))
        .await
        .unwrap();

    app.clone()
        .oneshot(req_empty("DELETE", &format!("/api/agents/{agent_id}")))
        .await
        .unwrap();

    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM agent_skills WHERE agent_id = $1")
        .bind(agent_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn compose_returns_system_prompt_and_skill_bodies(pool: PgPool) {
    let app = build_app_with_store(pool, make_store());
    let agent_id = create_agent(&app, "A1").await;
    let s1 = create_skill(&app, "S1", "Prefer short sentences. Be concise always.").await;
    let s2 = create_skill(&app, "S2", "Cite sources whenever possible in your replies.").await;

    app.clone()
        .oneshot(req_json(
            "PUT",
            &format!("/api/agents/{agent_id}/skills"),
            json!({ "skill_ids": [s1, s2] }),
        ))
        .await
        .unwrap();

    let resp = app
        .clone()
        .oneshot(req_get(&format!("/api/agents/{agent_id}/compose")))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    let composed = body["composed"].as_str().unwrap();
    assert!(composed.starts_with("You are a sharp, opinionated writing editor"));
    assert!(composed.contains("Prefer short sentences. Be concise always."));
    assert!(composed.contains("Cite sources whenever possible in your replies."));

    let length = body["length_chars"].as_i64().unwrap();
    assert_eq!(length, composed.chars().count() as i64);
    // Anthropic default budget is 200_000 * 4.
    assert_eq!(body["model_context_chars"], 800_000);
    let fraction = body["fraction"].as_f64().unwrap();
    assert!(fraction > 0.0 && fraction < 1.0);
    assert!(body["warning"].is_null());
}
