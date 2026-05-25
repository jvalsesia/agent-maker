//! Integration tests for the F02 /api/agents/* surface.
//! Real Postgres via `sqlx::test`, real FileStore in a tempdir.

use agent_maker::{
    build_app_with_store,
    secrets::{AnyStore, FileStore, SecretStore},
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
    Request::builder().method(method).uri(uri).body(Body::empty()).unwrap()
}

fn valid_agent(name: &str) -> Value {
    json!({
        "name": name,
        "preamble": "short blurb",
        "system_prompt": "You are a sharp, opinionated writing editor. Be specific and concise.",
        "provider": "anthropic",
        "model": "claude-haiku-4-5",
    })
}

#[sqlx::test(migrations = "./migrations")]
async fn create_then_list_and_get(pool: PgPool) {
    let app = build_app_with_store(pool, make_store());

    let resp = app
        .clone()
        .oneshot(req_json("POST", "/api/agents", valid_agent("Editor")))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = json_body(resp).await;
    let id = body["agent"]["id"].as_str().unwrap().to_string();
    assert_eq!(body["agent"]["name"], "Editor");
    assert_eq!(body["agent"]["has_override_key"], false);
    assert!(body["warnings"].as_array().unwrap().is_empty());

    let resp = app.clone().oneshot(req_get("/api/agents")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    let agents = body["agents"].as_array().unwrap();
    assert_eq!(agents.len(), 1);
    assert_eq!(agents[0]["attached_skill_count"], 0);
    assert_eq!(agents[0]["conversation_count"], 0);

    let resp = app
        .oneshot(req_get(&format!("/api/agents/{id}")))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["agent"]["id"], id);
}

#[sqlx::test(migrations = "./migrations")]
async fn duplicate_name_returns_field_error(pool: PgPool) {
    let app = build_app_with_store(pool, make_store());
    let _ = app
        .clone()
        .oneshot(req_json("POST", "/api/agents", valid_agent("Editor")))
        .await
        .unwrap();
    let resp = app
        .oneshot(req_json("POST", "/api/agents", valid_agent("Editor")))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body = json_body(resp).await;
    assert_eq!(body["error"]["field"], "name");
}

#[sqlx::test(migrations = "./migrations")]
async fn validation_failure_returns_400(pool: PgPool) {
    let app = build_app_with_store(pool, make_store());
    let mut bad = valid_agent("X");
    bad["provider"] = json!("bogus");
    let resp = app
        .oneshot(req_json("POST", "/api/agents", bad))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body = json_body(resp).await;
    assert_eq!(body["error"]["field"], "provider");
}

#[sqlx::test(migrations = "./migrations")]
async fn update_persists_and_bumps_updated_at(pool: PgPool) {
    let app = build_app_with_store(pool, make_store());
    let resp = app
        .clone()
        .oneshot(req_json("POST", "/api/agents", valid_agent("Editor")))
        .await
        .unwrap();
    let created = json_body(resp).await;
    let id = created["agent"]["id"].as_str().unwrap().to_string();
    let original_updated = created["agent"]["updated_at"].as_str().unwrap().to_string();

    // Sleep a moment so the trigger advances updated_at meaningfully.
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;

    let mut patch = valid_agent("Editor");
    patch["preamble"] = json!("new blurb");
    let resp = app
        .clone()
        .oneshot(req_json("PUT", &format!("/api/agents/{id}"), patch))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["agent"]["preamble"], "new blurb");
    assert_ne!(body["agent"]["updated_at"], original_updated);
}

#[sqlx::test(migrations = "./migrations")]
async fn response_language_defaults_to_auto_and_round_trips(pool: PgPool) {
    let app = build_app_with_store(pool, make_store());

    // Omitted → defaults to "auto".
    let resp = app
        .clone()
        .oneshot(req_json("POST", "/api/agents", valid_agent("Default Lang")))
        .await
        .unwrap();
    let body = json_body(resp).await;
    assert_eq!(body["agent"]["response_language"], "auto");

    // Explicit supported locale persists.
    let mut withlang = valid_agent("PT Agent");
    withlang["response_language"] = json!("pt-BR");
    let resp = app
        .clone()
        .oneshot(req_json("POST", "/api/agents", withlang))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = json_body(resp).await;
    let id = body["agent"]["id"].as_str().unwrap().to_string();
    assert_eq!(body["agent"]["response_language"], "pt-BR");

    let resp = app.oneshot(req_get(&format!("/api/agents/{id}"))).await.unwrap();
    let body = json_body(resp).await;
    assert_eq!(body["agent"]["response_language"], "pt-BR");
}

#[sqlx::test(migrations = "./migrations")]
async fn response_language_rejects_invalid(pool: PgPool) {
    let app = build_app_with_store(pool, make_store());
    let mut bad = valid_agent("Bad Lang");
    bad["response_language"] = json!("fr");
    let resp = app
        .oneshot(req_json("POST", "/api/agents", bad))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrations = "./migrations")]
async fn clone_suffixes_copy_and_numbers_collisions(pool: PgPool) {
    let app = build_app_with_store(pool, make_store());
    let resp = app
        .clone()
        .oneshot(req_json("POST", "/api/agents", valid_agent("Editor")))
        .await
        .unwrap();
    let id = json_body(resp).await["agent"]["id"].as_str().unwrap().to_string();

    let resp = app
        .clone()
        .oneshot(req_empty("POST", &format!("/api/agents/{id}/clone")))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    assert_eq!(json_body(resp).await["agent"]["name"], "Editor (copy)");

    // Cloning again must produce a numbered suffix
    let resp = app
        .oneshot(req_empty("POST", &format!("/api/agents/{id}/clone")))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    assert_eq!(
        json_body(resp).await["agent"]["name"],
        "Editor (copy) (2)"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn delete_removes_agent_and_namespaced_secret(pool: PgPool) {
    let store = make_store();
    let app = build_app_with_store(pool, store.clone());

    let resp = app
        .clone()
        .oneshot(req_json("POST", "/api/agents", valid_agent("Editor")))
        .await
        .unwrap();
    let id = json_body(resp).await["agent"]["id"].as_str().unwrap().to_string();

    // Save a per-agent key
    let resp = app
        .clone()
        .oneshot(req_json(
            "PUT",
            &format!("/api/agents/{id}/key"),
            json!({ "key": "sk-test-1234567890ABCDE" }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let saved = json_body(resp).await;
    assert_eq!(saved["has_override_key"], true);
    assert!(saved["key_masked"].as_str().unwrap().ends_with("BCDE"));

    let slug = format!("agent:{id}:anthropic");
    assert!(store.get(&slug).await.is_ok());

    // Delete the agent — the secret must go too.
    let resp = app
        .clone()
        .oneshot(req_empty("DELETE", &format!("/api/agents/{id}")))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Agent gone.
    let resp = app
        .oneshot(req_get(&format!("/api/agents/{id}")))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // Secret gone.
    assert!(matches!(
        store.get(&slug).await,
        Err(agent_maker::secrets::SecretError::NotFound)
    ));
}

#[sqlx::test(migrations = "./migrations")]
async fn delete_key_clears_override_flag(pool: PgPool) {
    let app = build_app_with_store(pool, make_store());
    let resp = app
        .clone()
        .oneshot(req_json("POST", "/api/agents", valid_agent("Editor")))
        .await
        .unwrap();
    let id = json_body(resp).await["agent"]["id"].as_str().unwrap().to_string();

    app.clone()
        .oneshot(req_json(
            "PUT",
            &format!("/api/agents/{id}/key"),
            json!({ "key": "sk-test-1234567890ABCDE" }),
        ))
        .await
        .unwrap();

    let resp = app
        .clone()
        .oneshot(req_empty("DELETE", &format!("/api/agents/{id}/key")))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let resp = app
        .oneshot(req_get(&format!("/api/agents/{id}")))
        .await
        .unwrap();
    let body = json_body(resp).await;
    assert_eq!(body["agent"]["has_override_key"], false);
}

#[sqlx::test(migrations = "./migrations")]
async fn list_search_and_sort(pool: PgPool) {
    let app = build_app_with_store(pool, make_store());
    for n in ["Zeta", "Alpha", "Beta"] {
        app.clone()
            .oneshot(req_json("POST", "/api/agents", valid_agent(n)))
            .await
            .unwrap();
    }

    let resp = app
        .clone()
        .oneshot(req_get("/api/agents?sort=name&order=asc"))
        .await
        .unwrap();
    let body = json_body(resp).await;
    let names: Vec<&str> = body["agents"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, vec!["Alpha", "Beta", "Zeta"]);

    let resp = app
        .oneshot(req_get("/api/agents?q=eta"))
        .await
        .unwrap();
    let body = json_body(resp).await;
    let names: Vec<&str> = body["agents"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, vec!["Beta", "Zeta"]);
}

#[sqlx::test(migrations = "./migrations")]
async fn models_endpoint_returns_static_list(pool: PgPool) {
    let app = build_app_with_store(pool, make_store());
    let resp = app
        .oneshot(req_get("/api/agents/models?provider=anthropic"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["provider"], "anthropic");
    let models = body["models"].as_array().unwrap();
    assert!(!models.is_empty());
}

#[sqlx::test(migrations = "./migrations")]
async fn soft_warnings_returned_on_short_prompt(pool: PgPool) {
    let app = build_app_with_store(pool, make_store());
    let mut a = valid_agent("Editor");
    a["system_prompt"] = json!("hi");
    let resp = app
        .oneshot(req_json("POST", "/api/agents", a))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = json_body(resp).await;
    let warnings = body["warnings"].as_array().unwrap();
    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0]["field"], "system_prompt");
}
