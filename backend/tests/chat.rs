//! Integration tests for `POST /api/conversations/:id/chat` (F07 Chat Runtime).
//! Real Postgres via `sqlx::test`; a stub `LlmProvider` and stub
//! `EmbeddingProvider` replace all network calls.

use agent_maker::{
    build_app_for_test,
    llm::{ChatDelta, ChatError, ChatRequest, ChatStream, ChatUsage, LlmProvider, ProviderName},
    llm::provider::{ProviderTestError, TestOutcome},
    memory::{EmbeddingError, EmbeddingProvider},
    secrets::{AnyStore, FileStore},
};
use async_trait::async_trait;
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::{Value, json};
use sqlx::PgPool;
use std::sync::{Arc, Mutex};
use tower::ServiceExt;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Stubs
// ---------------------------------------------------------------------------

#[derive(Clone)]
#[allow(dead_code)] // HttpErr exercises the mid-stream error arm in stream construction
enum StubItem {
    Delta(ChatDelta),
    HttpErr(u16, String),
}

/// A scripted streaming provider: records the last request, can fail preflight,
/// can gate the stream on a `Notify`, and yields a fixed list of deltas/errors.
#[derive(Clone)]
struct StubProvider {
    items: Arc<Vec<StubItem>>,
    preflight: Option<(u16, String)>,
    gate: Option<Arc<tokio::sync::Notify>>,
    recorded: Arc<Mutex<Option<ChatRequest>>>,
}

impl StubProvider {
    fn completion(text: &str) -> Self {
        let items = vec![
            StubItem::Delta(ChatDelta { content: text.to_string(), usage: None, finish_reason: None }),
            StubItem::Delta(ChatDelta {
                content: String::new(),
                usage: Some(ChatUsage { input_tokens: 12, output_tokens: 34 }),
                finish_reason: Some("stop".into()),
            }),
        ];
        Self { items: Arc::new(items), preflight: None, gate: None, recorded: Arc::new(Mutex::new(None)) }
    }
    fn preflight_error(status: u16) -> Self {
        Self {
            items: Arc::new(vec![]),
            preflight: Some((status, "invalid api key".into())),
            gate: None,
            recorded: Arc::new(Mutex::new(None)),
        }
    }
    fn gated(gate: Arc<tokio::sync::Notify>) -> Self {
        let items = vec![StubItem::Delta(ChatDelta {
            content: "late".into(),
            usage: None,
            finish_reason: Some("stop".into()),
        })];
        Self { items: Arc::new(items), preflight: None, gate: Some(gate), recorded: Arc::new(Mutex::new(None)) }
    }
}

#[async_trait]
impl LlmProvider for StubProvider {
    fn name(&self) -> ProviderName {
        ProviderName::Anthropic
    }
    async fn test(&self, _base_url: Option<&str>) -> Result<TestOutcome, ProviderTestError> {
        Ok(TestOutcome { ok: true, model_used: "stub".into(), latency_ms: 0 })
    }
    async fn chat(&self, req: ChatRequest) -> Result<ChatStream, ChatError> {
        *self.recorded.lock().unwrap() = Some(req);
        if let Some((status, body)) = &self.preflight {
            return Err(ChatError::Http { status: *status, body: body.clone() });
        }
        let items = self.items.clone();
        let gate = self.gate.clone();
        let stream = futures::stream::unfold((0usize, items, gate), |(idx, items, gate)| async move {
            if idx == 0
                && let Some(g) = &gate {
                    g.notified().await;
                }
            if idx >= items.len() {
                return None;
            }
            let out = match &items[idx] {
                StubItem::Delta(d) => Ok(d.clone()),
                StubItem::HttpErr(s, b) => Err(ChatError::Http { status: *s, body: b.clone() }),
            };
            Some((out, (idx + 1, items, gate)))
        });
        Ok(Box::pin(stream))
    }
}

/// Deterministic char-bucket embedder (dim 1536), so similar text → similar
/// vectors without any network call. Optionally fails to exercise degraded mode.
struct StubEmbedder {
    fail: bool,
}

#[async_trait]
impl EmbeddingProvider for StubEmbedder {
    async fn embed_one(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        if self.fail {
            return Err(EmbeddingError::Network("stub failure".into()));
        }
        let mut v = vec![0.0f32; 1536];
        for b in text.to_lowercase().bytes() {
            v[(b as usize) % 1536] += 1.0;
        }
        Ok(v)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_store() -> Arc<AnyStore> {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.keep();
    Arc::new(AnyStore::File(Box::new(FileStore::open_or_create(&path).unwrap())))
}

fn app(pool: PgPool, provider: StubProvider, fail_embed: bool) -> axum::Router {
    build_app_for_test(
        pool,
        make_store(),
        Arc::new(provider),
        Arc::new(StubEmbedder { fail: fail_embed }),
    )
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
    Request::builder().method("GET").uri(uri).body(Body::empty()).unwrap()
}

async fn json_body(resp: axum::response::Response) -> Value {
    let bytes = to_bytes(resp.into_body(), 1 << 20).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

/// Collect a full SSE response body into parsed event objects.
async fn sse_events(resp: axum::response::Response) -> Vec<Value> {
    let bytes = to_bytes(resp.into_body(), 1 << 22).await.unwrap();
    let text = String::from_utf8(bytes.to_vec()).unwrap();
    let mut out = Vec::new();
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("data:") {
            let payload = rest.trim();
            if !payload.is_empty() {
                out.push(serde_json::from_str(payload).unwrap());
            }
        }
    }
    out
}

async fn seed_agent(pool: &PgPool, provider: &str, model: &str) -> Uuid {
    sqlx::query_scalar(
        r#"INSERT INTO agents (name, system_prompt, provider, model)
           VALUES ($1, 'BASE SYSTEM PROMPT', $2, $3) RETURNING id"#,
    )
    .bind(format!("agent-{}", Uuid::new_v4()))
    .bind(provider)
    .bind(model)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn seed_conversation(pool: &PgPool, agent_id: Uuid) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO conversations (agent_id, title) VALUES ($1, 'New conversation') RETURNING id",
    )
    .bind(agent_id)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn attach_skill(pool: &PgPool, agent_id: Uuid, body: &str, position: i16) {
    let skill_id: Uuid = sqlx::query_scalar(
        "INSERT INTO skills (name, description, body) VALUES ($1, 'd', $2) RETURNING id",
    )
    .bind(format!("skill-{}", Uuid::new_v4()))
    .bind(body)
    .fetch_one(pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO agent_skills (agent_id, skill_id, position) VALUES ($1, $2, $3)")
        .bind(agent_id)
        .bind(skill_id)
        .bind(position)
        .execute(pool)
        .await
        .unwrap();
}

fn ev_type<'a>(events: &'a [Value], t: &str) -> Option<&'a Value> {
    events.iter().find(|e| e["type"] == t)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "./migrations")]
async fn test_chat_streams_and_persists(pool: PgPool) {
    let agent = seed_agent(&pool, "anthropic", "claude-haiku-4-5").await;
    let conv = seed_conversation(&pool, agent).await;
    let app = app(pool.clone(), StubProvider::completion("Hello there"), false);

    let resp = app
        .oneshot(req_json("POST", &format!("/api/conversations/{conv}/chat"), json!({"content": "hi"})))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let events = sse_events(resp).await;
    assert!(ev_type(&events, "meta").is_some(), "meta frame present");
    assert!(ev_type(&events, "chunk").is_some(), "chunk frame present");
    let done = ev_type(&events, "done").expect("done frame");
    assert_eq!(done["status"], "complete");
    assert_eq!(done["token_count"], 34);

    // user + assistant persisted.
    let rows: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT role, content, status FROM messages WHERE conversation_id = $1 ORDER BY created_at",
    )
    .bind(conv)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].0, "user");
    assert_eq!(rows[1], ("assistant".into(), "Hello there".into(), "complete".into()));
}

#[sqlx::test(migrations = "./migrations")]
async fn test_compose_order_system_skills_memory_tail(pool: PgPool) {
    let agent = seed_agent(&pool, "anthropic", "claude-haiku-4-5").await;
    attach_skill(&pool, agent, "SKILL ONE BODY", 0).await;
    attach_skill(&pool, agent, "SKILL TWO BODY", 1).await;
    let conv = seed_conversation(&pool, agent).await;
    let stub = StubProvider::completion("ok");
    let recorded = stub.recorded.clone();
    let app = app(pool.clone(), stub, false);

    let resp = app
        .oneshot(req_json("POST", &format!("/api/conversations/{conv}/chat"), json!({"content": "question?"})))
        .await
        .unwrap();
    let _ = sse_events(resp).await;

    let req = recorded.lock().unwrap().clone().expect("provider received a request");
    let system = req.system.unwrap();
    let one = system.find("SKILL ONE BODY").expect("skill one in system");
    let two = system.find("SKILL TWO BODY").expect("skill two in system");
    assert!(system.starts_with("BASE SYSTEM PROMPT"));
    assert!(one < two, "skills concatenated in attachment order");
    assert_eq!(req.messages.last().unwrap().content, "question?", "tail is the current user turn");
}

#[sqlx::test(migrations = "./migrations")]
async fn test_skill_edit_changes_next_prompt(pool: PgPool) {
    let agent = seed_agent(&pool, "anthropic", "claude-haiku-4-5").await;
    let skill_id: Uuid = sqlx::query_scalar(
        "INSERT INTO skills (name, description, body) VALUES ($1, 'd', 'ORIGINAL') RETURNING id",
    )
    .bind(format!("skill-{}", Uuid::new_v4()))
    .fetch_one(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO agent_skills (agent_id, skill_id, position) VALUES ($1, $2, 0)")
        .bind(agent)
        .bind(skill_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE skills SET body = 'UPDATED BODY' WHERE id = $1")
        .bind(skill_id)
        .execute(&pool)
        .await
        .unwrap();
    let conv = seed_conversation(&pool, agent).await;
    let stub = StubProvider::completion("ok");
    let recorded = stub.recorded.clone();
    let app = app(pool.clone(), stub, false);

    let resp = app
        .oneshot(req_json("POST", &format!("/api/conversations/{conv}/chat"), json!({"content": "go"})))
        .await
        .unwrap();
    let _ = sse_events(resp).await;
    let system = recorded.lock().unwrap().clone().unwrap().system.unwrap();
    assert!(system.contains("UPDATED BODY"));
    assert!(!system.contains("ORIGINAL"));
}

#[sqlx::test(migrations = "./migrations")]
async fn test_per_agent_provider_model_overrides_default(pool: PgPool) {
    let agent = seed_agent(&pool, "openai", "gpt-4o").await;
    let conv = seed_conversation(&pool, agent).await;
    let stub = StubProvider::completion("ok");
    let recorded = stub.recorded.clone();
    let app = app(pool.clone(), stub, false);

    let resp = app
        .oneshot(req_json("POST", &format!("/api/conversations/{conv}/chat"), json!({"content": "go"})))
        .await
        .unwrap();
    let _ = sse_events(resp).await;
    assert_eq!(recorded.lock().unwrap().clone().unwrap().model, "gpt-4o");
}

#[sqlx::test(migrations = "./migrations")]
async fn test_in_flight_conflict_blocked(pool: PgPool) {
    let agent = seed_agent(&pool, "anthropic", "claude-haiku-4-5").await;
    let conv = seed_conversation(&pool, agent).await;
    let gate = Arc::new(tokio::sync::Notify::new());
    let app = app(pool.clone(), StubProvider::gated(gate.clone()), false);

    // First request: stream is gated open (task parked, in-flight guard held).
    let first = app
        .clone()
        .oneshot(req_json("POST", &format!("/api/conversations/{conv}/chat"), json!({"content": "one"})))
        .await
        .unwrap();
    assert_eq!(first.status(), StatusCode::OK);

    // Second request for the same conversation is rejected.
    let second = app
        .oneshot(req_json("POST", &format!("/api/conversations/{conv}/chat"), json!({"content": "two"})))
        .await
        .unwrap();
    assert_eq!(second.status(), StatusCode::CONFLICT);
    let body = json_body(second).await;
    assert_eq!(body["error"]["code"], "conflict");

    // Release the first stream so its task can finish cleanly.
    gate.notify_one();
    let _ = sse_events(first).await;
}

#[sqlx::test(migrations = "./migrations")]
async fn test_retry_no_duplicate_user(pool: PgPool) {
    let agent = seed_agent(&pool, "anthropic", "claude-haiku-4-5").await;
    let conv = seed_conversation(&pool, agent).await;

    // First send produces a user + assistant turn.
    let app1 = app(pool.clone(), StubProvider::completion("first answer"), false);
    let r1 = app1
        .oneshot(req_json("POST", &format!("/api/conversations/{conv}/chat"), json!({"content": "the question"})))
        .await
        .unwrap();
    let _ = sse_events(r1).await;

    // Mark the assistant turn as errored to simulate a failed reply.
    sqlx::query("UPDATE messages SET status = 'error' WHERE conversation_id = $1 AND role = 'assistant'")
        .bind(conv)
        .execute(&pool)
        .await
        .unwrap();

    // Retry: no new user row, prior assistant superseded.
    let app2 = app(pool.clone(), StubProvider::completion("second answer"), false);
    let r2 = app2
        .oneshot(req_json("POST", &format!("/api/conversations/{conv}/chat"), json!({"retry": true})))
        .await
        .unwrap();
    assert_eq!(r2.status(), StatusCode::OK);
    let _ = sse_events(r2).await;

    let users: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM messages WHERE conversation_id = $1 AND role = 'user'",
    )
    .bind(conv)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(users, 1, "retry must not duplicate the user message");
    let answer: String = sqlx::query_scalar(
        "SELECT content FROM messages WHERE conversation_id = $1 AND role = 'assistant' AND status = 'complete'",
    )
    .bind(conv)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(answer, "second answer");
}

#[sqlx::test(migrations = "./migrations")]
async fn test_recalls_persisted_and_listed(pool: PgPool) {
    let agent = seed_agent(&pool, "anthropic", "claude-haiku-4-5").await;
    // Small recent window so older turns become retrievable.
    sqlx::query("UPDATE agents SET recent_n_override = 4, top_k_override = 5 WHERE id = $1")
        .bind(agent)
        .execute(&pool)
        .await
        .unwrap();
    let conv = seed_conversation(&pool, agent).await;

    // Distinctive older turn first, then filler turns.
    let seeded = [
        ("user", "the secret code is platypus and nothing else"),
        ("assistant", "understood, noted the code"),
        ("user", "let's talk about weather"),
        ("assistant", "it is sunny today"),
        ("user", "and about food"),
        ("assistant", "pizza is great"),
    ];
    for (i, (role, content)) in seeded.iter().enumerate() {
        sqlx::query(
            "INSERT INTO messages (conversation_id, role, content, created_at)
             VALUES ($1, $2, $3, now() + ($4 || ' seconds')::interval)",
        )
        .bind(conv)
        .bind(role)
        .bind(content)
        .bind(i as i32)
        .execute(&pool)
        .await
        .unwrap();
    }

    // Embed the older turns through the stub embedder.
    let embed_app = app(pool.clone(), StubProvider::completion("x"), false);
    let er = embed_app
        .oneshot(req_json("POST", &format!("/api/conversations/{conv}/memory/embed-pending"), json!({})))
        .await
        .unwrap();
    assert_eq!(er.status(), StatusCode::OK);

    // Chat with a query that matches the distinctive older turn.
    let chat_app = app(pool.clone(), StubProvider::completion("recall answer"), false);
    let resp = chat_app
        .oneshot(req_json(
            "POST",
            &format!("/api/conversations/{conv}/chat"),
            json!({"content": "what was the secret code platypus"}),
        ))
        .await
        .unwrap();
    let events = sse_events(resp).await;
    let meta = ev_type(&events, "meta").unwrap();
    let recalled = meta["recalled"].as_array().unwrap();
    assert!(!recalled.is_empty(), "meta should surface recalled turns");

    // GET messages should show the recalled[] on the assistant message after reload.
    let list_app = app(pool.clone(), StubProvider::completion("x"), false);
    let listed = list_app
        .oneshot(req_get(&format!("/api/conversations/{conv}/messages")))
        .await
        .unwrap();
    let body = json_body(listed).await;
    let assistant_with_recall = body["messages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["role"] == "assistant" && m.get("recalled").is_some());
    assert!(assistant_with_recall.is_some(), "assistant message exposes recalled[]");
}

#[sqlx::test(migrations = "./migrations")]
async fn test_memory_degraded_falls_back(pool: PgPool) {
    let agent = seed_agent(&pool, "anthropic", "claude-haiku-4-5").await;
    let conv = seed_conversation(&pool, agent).await;
    // Embedder fails → memory degrades but chat still completes.
    let app = app(pool.clone(), StubProvider::completion("still works"), true);

    let resp = app
        .oneshot(req_json("POST", &format!("/api/conversations/{conv}/chat"), json!({"content": "hello"})))
        .await
        .unwrap();
    let events = sse_events(resp).await;
    let meta = ev_type(&events, "meta").unwrap();
    assert_eq!(meta["degraded"], true);
    assert_eq!(ev_type(&events, "done").unwrap()["status"], "complete");
}

#[sqlx::test(migrations = "./migrations")]
async fn test_context_too_large(pool: PgPool) {
    // openai_compat → llama3 budget 128_000; an oversized skill body blows it.
    let agent = seed_agent(&pool, "openai_compat", "llama3").await;
    let huge = "x".repeat(200_000);
    attach_skill(&pool, agent, &huge, 0).await;
    let conv = seed_conversation(&pool, agent).await;
    let app = app(pool.clone(), StubProvider::completion("never"), false);

    let resp = app
        .oneshot(req_json("POST", &format!("/api/conversations/{conv}/chat"), json!({"content": "hi"})))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body = json_body(resp).await;
    assert_eq!(body["error"]["code"], "context_too_large");
}

#[sqlx::test(migrations = "./migrations")]
async fn test_provider_4xx_preflight(pool: PgPool) {
    let agent = seed_agent(&pool, "anthropic", "claude-haiku-4-5").await;
    let conv = seed_conversation(&pool, agent).await;
    let app = app(pool.clone(), StubProvider::preflight_error(401), false);

    let resp = app
        .oneshot(req_json("POST", &format!("/api/conversations/{conv}/chat"), json!({"content": "hi"})))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let body = json_body(resp).await;
    assert_eq!(body["error"]["code"], "provider_error");
}
