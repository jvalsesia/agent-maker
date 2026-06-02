//! Integration tests for the F11 /api/agents/:id/subagents surface (CRUD,
//! cycle rejection, reorder, and detach gap-closing). Real Postgres via
//! `sqlx::test`; auth disabled through `build_app_with_store`.

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
    Request::builder().method(method).uri(uri).body(Body::empty()).unwrap()
}

async fn seed_agent(pool: &PgPool, name: &str) -> Uuid {
    sqlx::query_scalar(
        r#"INSERT INTO agents (name, system_prompt, provider, model)
           VALUES ($1, 'BASE SYSTEM PROMPT', 'anthropic', 'claude-haiku-4-5') RETURNING id"#,
    )
    .bind(format!("{name}-{}", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn attach_list_detach_roundtrip(pool: PgPool) {
    let parent = seed_agent(&pool, "parent").await;
    let child = seed_agent(&pool, "child").await;
    let app = build_app_with_store(pool, make_store());

    // Attach with explicit alias + description.
    let resp = app
        .clone()
        .oneshot(req_json(
            "POST",
            &format!("/api/agents/{parent}/subagents"),
            json!({ "child_id": child, "alias": "reviewer", "description": "use for review" }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = json_body(resp).await;
    assert_eq!(body["attached"]["alias"], "reviewer");
    assert_eq!(body["attached"]["position"], 0);

    // List reflects the attachment.
    let resp = app
        .clone()
        .oneshot(req_get(&format!("/api/agents/{parent}/subagents")))
        .await
        .unwrap();
    let body = json_body(resp).await;
    let attached = body["attached"].as_array().unwrap();
    assert_eq!(attached.len(), 1);
    assert_eq!(attached[0]["child_id"], child.to_string());
    assert_eq!(attached[0]["description"], "use for review");

    // Detach removes it.
    let resp = app
        .clone()
        .oneshot(req_empty("DELETE", &format!("/api/agents/{parent}/subagents/{child}")))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let resp = app
        .oneshot(req_get(&format!("/api/agents/{parent}/subagents")))
        .await
        .unwrap();
    let body = json_body(resp).await;
    assert!(body["attached"].as_array().unwrap().is_empty());
}

#[sqlx::test(migrations = "./migrations")]
async fn attach_defaults_alias_from_child_name(pool: PgPool) {
    let parent = seed_agent(&pool, "parent").await;
    // Use a deterministic child name to assert the slug.
    let child: Uuid = sqlx::query_scalar(
        "INSERT INTO agents (name, system_prompt, provider, model)
         VALUES ('Code Reviewer', 'p', 'anthropic', 'claude-haiku-4-5') RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let app = build_app_with_store(pool, make_store());

    let resp = app
        .oneshot(req_json(
            "POST",
            &format!("/api/agents/{parent}/subagents"),
            json!({ "child_id": child }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = json_body(resp).await;
    assert_eq!(body["attached"]["alias"], "code-reviewer");
}

#[sqlx::test(migrations = "./migrations")]
async fn attach_rejects_self(pool: PgPool) {
    let agent = seed_agent(&pool, "solo").await;
    let app = build_app_with_store(pool, make_store());

    let resp = app
        .oneshot(req_json(
            "POST",
            &format!("/api/agents/{agent}/subagents"),
            json!({ "child_id": agent }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body = json_body(resp).await;
    assert_eq!(body["error"]["field"], "child_id");
}

#[sqlx::test(migrations = "./migrations")]
async fn attach_rejects_cycle_http(pool: PgPool) {
    let a = seed_agent(&pool, "a").await;
    let b = seed_agent(&pool, "b").await;
    let app = build_app_with_store(pool, make_store());

    // A -> B succeeds.
    let resp = app
        .clone()
        .oneshot(req_json(
            "POST",
            &format!("/api/agents/{a}/subagents"),
            json!({ "child_id": b, "alias": "bee" }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // B -> A would close a cycle and is rejected.
    let resp = app
        .oneshot(req_json(
            "POST",
            &format!("/api/agents/{b}/subagents"),
            json!({ "child_id": a, "alias": "ay" }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body = json_body(resp).await;
    assert_eq!(body["error"]["field"], "child_id");
}

#[sqlx::test(migrations = "./migrations")]
async fn attach_rejects_eleventh(pool: PgPool) {
    let parent = seed_agent(&pool, "hub").await;
    let app = build_app_with_store(pool.clone(), make_store());

    for i in 0..10 {
        let child = seed_agent(&pool, &format!("c{i}")).await;
        let resp = app
            .clone()
            .oneshot(req_json(
                "POST",
                &format!("/api/agents/{parent}/subagents"),
                json!({ "child_id": child, "alias": format!("a{i}") }),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED, "attach {i} should succeed");
    }

    let eleventh = seed_agent(&pool, "overflow").await;
    let resp = app
        .oneshot(req_json(
            "POST",
            &format!("/api/agents/{parent}/subagents"),
            json!({ "child_id": eleventh, "alias": "too-many" }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body = json_body(resp).await;
    assert_eq!(body["error"]["field"], "child_id");
}

#[sqlx::test(migrations = "./migrations")]
async fn attach_rejects_duplicate_alias(pool: PgPool) {
    let parent = seed_agent(&pool, "p").await;
    let c1 = seed_agent(&pool, "c1").await;
    let c2 = seed_agent(&pool, "c2").await;
    let app = build_app_with_store(pool, make_store());

    let ok = app
        .clone()
        .oneshot(req_json(
            "POST",
            &format!("/api/agents/{parent}/subagents"),
            json!({ "child_id": c1, "alias": "dup" }),
        ))
        .await
        .unwrap();
    assert_eq!(ok.status(), StatusCode::CREATED);

    let clash = app
        .oneshot(req_json(
            "POST",
            &format!("/api/agents/{parent}/subagents"),
            json!({ "child_id": c2, "alias": "dup" }),
        ))
        .await
        .unwrap();
    assert_eq!(clash.status(), StatusCode::BAD_REQUEST);
    let body = json_body(clash).await;
    assert_eq!(body["error"]["field"], "alias");
}

#[sqlx::test(migrations = "./migrations")]
async fn update_alias_and_description(pool: PgPool) {
    let parent = seed_agent(&pool, "p").await;
    let child = seed_agent(&pool, "c").await;
    let app = build_app_with_store(pool, make_store());

    app.clone()
        .oneshot(req_json(
            "POST",
            &format!("/api/agents/{parent}/subagents"),
            json!({ "child_id": child, "alias": "old" }),
        ))
        .await
        .unwrap();

    let resp = app
        .clone()
        .oneshot(req_json(
            "PUT",
            &format!("/api/agents/{parent}/subagents/{child}"),
            json!({ "alias": "new", "description": "updated hint" }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["attached"]["alias"], "new");
    assert_eq!(body["attached"]["description"], "updated hint");

    // Persisted across a fresh list.
    let resp = app
        .oneshot(req_get(&format!("/api/agents/{parent}/subagents")))
        .await
        .unwrap();
    let body = json_body(resp).await;
    assert_eq!(body["attached"][0]["alias"], "new");
}

#[sqlx::test(migrations = "./migrations")]
async fn reorder_persists(pool: PgPool) {
    let parent = seed_agent(&pool, "p").await;
    let a = seed_agent(&pool, "a").await;
    let b = seed_agent(&pool, "b").await;
    let c = seed_agent(&pool, "c").await;
    let app = build_app_with_store(pool, make_store());

    for (child, alias) in [(a, "aa"), (b, "bb"), (c, "cc")] {
        app.clone()
            .oneshot(req_json(
                "POST",
                &format!("/api/agents/{parent}/subagents"),
                json!({ "child_id": child, "alias": alias }),
            ))
            .await
            .unwrap();
    }

    // Reverse the order.
    let resp = app
        .clone()
        .oneshot(req_json(
            "PUT",
            &format!("/api/agents/{parent}/subagents"),
            json!({ "ordered_child_ids": [c, b, a] }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let resp = app
        .oneshot(req_get(&format!("/api/agents/{parent}/subagents")))
        .await
        .unwrap();
    let body = json_body(resp).await;
    let attached = body["attached"].as_array().unwrap();
    let ids: Vec<&str> = attached.iter().map(|x| x["child_id"].as_str().unwrap()).collect();
    assert_eq!(ids, vec![c.to_string(), b.to_string(), a.to_string()]);
    let positions: Vec<i64> = attached.iter().map(|x| x["position"].as_i64().unwrap()).collect();
    assert_eq!(positions, vec![0, 1, 2], "positions renumber contiguously");
}

#[sqlx::test(migrations = "./migrations")]
async fn reorder_rejects_non_permutation(pool: PgPool) {
    let parent = seed_agent(&pool, "p").await;
    let a = seed_agent(&pool, "a").await;
    let b = seed_agent(&pool, "b").await;
    let app = build_app_with_store(pool, make_store());

    for (child, alias) in [(a, "aa"), (b, "bb")] {
        app.clone()
            .oneshot(req_json(
                "POST",
                &format!("/api/agents/{parent}/subagents"),
                json!({ "child_id": child, "alias": alias }),
            ))
            .await
            .unwrap();
    }

    let stranger = Uuid::new_v4();
    let resp = app
        .oneshot(req_json(
            "PUT",
            &format!("/api/agents/{parent}/subagents"),
            json!({ "ordered_child_ids": [a, stranger] }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrations = "./migrations")]
async fn agents_list_surfaces_subagent_chips(pool: PgPool) {
    let parent = seed_agent(&pool, "parent").await;
    // Deterministic child name so we can assert the chip's name field.
    let child: Uuid = sqlx::query_scalar(
        "INSERT INTO agents (name, system_prompt, provider, model)
         VALUES ('Code Reviewer', 'p', 'anthropic', 'claude-haiku-4-5') RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let app = build_app_with_store(pool, make_store());

    app.clone()
        .oneshot(req_json(
            "POST",
            &format!("/api/agents/{parent}/subagents"),
            json!({ "child_id": child, "alias": "reviewer" }),
        ))
        .await
        .unwrap();

    let resp = app.oneshot(req_get("/api/agents")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    let agents = body["agents"].as_array().unwrap();

    let parent_row = agents
        .iter()
        .find(|a| a["id"] == parent.to_string())
        .expect("parent present in list");
    let chips = parent_row["subagents"].as_array().unwrap();
    assert_eq!(chips.len(), 1);
    assert_eq!(chips[0]["alias"], "reviewer");
    assert_eq!(chips[0]["name"], "Code Reviewer");

    // The child agent, with no attachments of its own, reports an empty array.
    let child_row = agents
        .iter()
        .find(|a| a["id"] == child.to_string())
        .expect("child present in list");
    assert!(child_row["subagents"].as_array().unwrap().is_empty());
}

#[sqlx::test(migrations = "./migrations")]
async fn detach_closes_gap(pool: PgPool) {
    let parent = seed_agent(&pool, "p").await;
    let a = seed_agent(&pool, "a").await;
    let b = seed_agent(&pool, "b").await;
    let c = seed_agent(&pool, "c").await;
    let app = build_app_with_store(pool, make_store());

    for (child, alias) in [(a, "aa"), (b, "bb"), (c, "cc")] {
        app.clone()
            .oneshot(req_json(
                "POST",
                &format!("/api/agents/{parent}/subagents"),
                json!({ "child_id": child, "alias": alias }),
            ))
            .await
            .unwrap();
    }

    // Detach the middle one.
    app.clone()
        .oneshot(req_empty("DELETE", &format!("/api/agents/{parent}/subagents/{b}")))
        .await
        .unwrap();

    let resp = app
        .oneshot(req_get(&format!("/api/agents/{parent}/subagents")))
        .await
        .unwrap();
    let body = json_body(resp).await;
    let attached = body["attached"].as_array().unwrap();
    let ids: Vec<&str> = attached.iter().map(|x| x["child_id"].as_str().unwrap()).collect();
    assert_eq!(ids, vec![a.to_string(), c.to_string()]);
    let positions: Vec<i64> = attached.iter().map(|x| x["position"].as_i64().unwrap()).collect();
    assert_eq!(positions, vec![0, 1], "gap closed after detach");
}
