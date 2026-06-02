//! Integration tests for F11 delegation over the chat SSE stream. A stub
//! `LlmProvider` records every dispatched request (child first, parent last) so
//! we can assert the child runs with its own model on the stripped task text and
//! the parent synthesizes with the "Sub-agent responses" block in its prompt.

use agent_maker::{
    build_app_for_test,
    llm::provider::{ProviderTestError, TestOutcome},
    llm::{ChatDelta, ChatError, ChatRequest, ChatStream, ChatUsage, LlmProvider, ProviderName},
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

/// Records every request it receives; optionally fails preflight (to exercise a
/// failing sub-agent). Returns a fixed reply otherwise.
#[derive(Clone)]
struct RecordingProvider {
    reply: String,
    fail_preflight: Option<u16>,
    recorded: Arc<Mutex<Vec<ChatRequest>>>,
}

impl RecordingProvider {
    fn completion(reply: &str) -> Self {
        Self { reply: reply.into(), fail_preflight: None, recorded: Arc::new(Mutex::new(Vec::new())) }
    }
    fn failing() -> Self {
        Self { reply: String::new(), fail_preflight: Some(401), recorded: Arc::new(Mutex::new(Vec::new())) }
    }
}

#[async_trait]
impl LlmProvider for RecordingProvider {
    fn name(&self) -> ProviderName {
        ProviderName::Anthropic
    }
    async fn test(&self, _base_url: Option<&str>) -> Result<TestOutcome, ProviderTestError> {
        Ok(TestOutcome { ok: true, model_used: "stub".into(), latency_ms: 0 })
    }
    async fn chat(&self, req: ChatRequest) -> Result<ChatStream, ChatError> {
        self.recorded.lock().unwrap().push(req);
        if let Some(status) = self.fail_preflight {
            return Err(ChatError::Http { status, body: "invalid api key".into() });
        }
        let reply = self.reply.clone();
        let items = vec![
            ChatDelta { content: reply, usage: None, finish_reason: None },
            ChatDelta {
                content: String::new(),
                usage: Some(ChatUsage { input_tokens: 5, output_tokens: 7 }),
                finish_reason: Some("stop".into()),
            },
        ];
        let stream = futures::stream::unfold((0usize, items), |(idx, items)| async move {
            if idx >= items.len() {
                return None;
            }
            Some((Ok(items[idx].clone()), (idx + 1, items)))
        });
        Ok(Box::pin(stream))
    }
}

struct NoopEmbedder;

#[async_trait]
impl EmbeddingProvider for NoopEmbedder {
    async fn embed_one(&self, _text: &str) -> Result<Vec<f32>, EmbeddingError> {
        Ok(vec![0.0f32; 1536])
    }
}

fn make_store() -> Arc<AnyStore> {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.keep();
    Arc::new(AnyStore::File(Box::new(FileStore::open_or_create(&path).unwrap())))
}

fn app(pool: PgPool, provider: RecordingProvider) -> axum::Router {
    build_app_for_test(pool, make_store(), Arc::new(provider), Arc::new(NoopEmbedder))
}

fn req_json(method: &str, uri: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

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

fn ev_type<'a>(events: &'a [Value], t: &str) -> Option<&'a Value> {
    events.iter().find(|e| e["type"] == t)
}

async fn seed_agent(pool: &PgPool, name: &str, provider: &str, model: &str) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO agents (name, system_prompt, provider, model) VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind(format!("{name}-{}", Uuid::new_v4()))
    .bind("BASE SYSTEM PROMPT")
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

async fn attach_subagent(pool: &PgPool, parent: Uuid, child: Uuid, alias: &str) {
    sqlx::query(
        "INSERT INTO agent_subagents (parent_id, child_id, alias, position) VALUES ($1, $2, $3, 0)",
    )
    .bind(parent)
    .bind(child)
    .bind(alias)
    .execute(pool)
    .await
    .unwrap();
}

#[sqlx::test(migrations = "./migrations")]
async fn delegation_emits_subagent_and_parent(pool: PgPool) {
    let parent = seed_agent(&pool, "parent", "anthropic", "claude-haiku-4-5").await;
    let child = seed_agent(&pool, "child", "openai", "gpt-4o").await;
    attach_subagent(&pool, parent, child, "reviewer").await;
    let conv = seed_conversation(&pool, parent).await;

    let provider = RecordingProvider::completion("the sub-agent reply");
    let recorded = provider.recorded.clone();
    let app = app(pool.clone(), provider);

    let resp = app
        .oneshot(req_json(
            "POST",
            &format!("/api/conversations/{conv}/chat"),
            json!({ "content": "@reviewer please review this" }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let events = sse_events(resp).await;

    // A labeled sub-agent frame precedes the parent's done frame.
    let sub = ev_type(&events, "subagent").expect("subagent frame present");
    assert_eq!(sub["alias"], "reviewer");
    assert_eq!(sub["agent_id"], child.to_string());
    assert_eq!(sub["status"], "complete");
    assert_eq!(sub["content"], "the sub-agent reply");
    assert_eq!(ev_type(&events, "done").unwrap()["status"], "complete");

    // The child ran first, on its own model, with exactly the stripped task.
    let reqs = recorded.lock().unwrap().clone();
    assert_eq!(reqs.len(), 2, "one child call + one parent call");
    assert_eq!(reqs[0].model, "gpt-4o", "child dispatches with its own model");
    assert_eq!(reqs[0].messages.len(), 1, "child sees only the task (stateless)");
    assert_eq!(reqs[0].messages[0].content, "please review this");

    // The parent ran last, on its own model, with the responses block in context.
    let parent_req = reqs.last().unwrap();
    assert_eq!(parent_req.model, "claude-haiku-4-5");
    let system = parent_req.system.as_ref().unwrap();
    assert!(system.contains("# Sub-agent responses"), "parent prompt has the block");
    assert!(system.contains("@reviewer"));
    assert!(system.contains("the sub-agent reply"));

    // Persistence: one delegated turn (alias set) + one parent assistant turn.
    let sub_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM messages WHERE conversation_id = $1 AND subagent_alias = 'reviewer' AND status = 'complete'",
    )
    .bind(conv)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(sub_rows, 1);
    let parent_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM messages WHERE conversation_id = $1 AND role = 'assistant' AND subagent_alias IS NULL AND status = 'complete'",
    )
    .bind(conv)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(parent_rows, 1, "parent synthesized its own turn");

    // The parent's own user turn keeps the original (un-stripped) text.
    let user_text: String = sqlx::query_scalar(
        "SELECT content FROM messages WHERE conversation_id = $1 AND role = 'user'",
    )
    .bind(conv)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(user_text, "@reviewer please review this");
}

#[sqlx::test(migrations = "./migrations")]
async fn delegation_child_error_holds_parent(pool: PgPool) {
    let parent = seed_agent(&pool, "parent", "anthropic", "claude-haiku-4-5").await;
    let child = seed_agent(&pool, "child", "openai", "gpt-4o").await;
    attach_subagent(&pool, parent, child, "reviewer").await;
    let conv = seed_conversation(&pool, parent).await;

    // The provider fails preflight, so the (first) child call errors.
    let provider = RecordingProvider::failing();
    let recorded = provider.recorded.clone();
    let app = app(pool.clone(), provider);

    let resp = app
        .oneshot(req_json(
            "POST",
            &format!("/api/conversations/{conv}/chat"),
            json!({ "content": "@reviewer do it" }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let events = sse_events(resp).await;

    let sub = ev_type(&events, "subagent").expect("subagent error frame present");
    assert_eq!(sub["status"], "error");
    assert!(ev_type(&events, "error").is_some(), "an error frame holds the turn");
    assert!(ev_type(&events, "done").is_none(), "parent never synthesizes");

    // Only the child was ever dispatched.
    assert_eq!(recorded.lock().unwrap().len(), 1);

    // The delegated turn is persisted as an error; no complete parent turn exists.
    let err_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM messages WHERE conversation_id = $1 AND subagent_alias = 'reviewer' AND status = 'error'",
    )
    .bind(conv)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(err_rows, 1);
    let complete_parent: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM messages WHERE conversation_id = $1 AND role = 'assistant' AND subagent_alias IS NULL AND status = 'complete'",
    )
    .bind(conv)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(complete_parent, 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn unmatched_mention_completes_normally(pool: PgPool) {
    let parent = seed_agent(&pool, "parent", "anthropic", "claude-haiku-4-5").await;
    let child = seed_agent(&pool, "child", "openai", "gpt-4o").await;
    attach_subagent(&pool, parent, child, "reviewer").await;
    let conv = seed_conversation(&pool, parent).await;

    let provider = RecordingProvider::completion("normal answer");
    let recorded = provider.recorded.clone();
    let app = app(pool.clone(), provider);

    let resp = app
        .oneshot(req_json(
            "POST",
            &format!("/api/conversations/{conv}/chat"),
            json!({ "content": "@nobody hello there" }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let events = sse_events(resp).await;

    assert!(ev_type(&events, "subagent").is_none(), "no delegation occurs");
    assert_eq!(ev_type(&events, "done").unwrap()["status"], "complete");

    // Only the parent was dispatched, with the original text intact as the tail.
    let reqs = recorded.lock().unwrap().clone();
    assert_eq!(reqs.len(), 1);
    assert_eq!(reqs[0].messages.last().unwrap().content, "@nobody hello there");

    // The meta frame carries an "unknown" notice for the unmatched handle.
    let meta = ev_type(&events, "meta").unwrap();
    let notices = meta["notices"].as_array().expect("notices present");
    assert!(notices.iter().any(|n| n["code"] == "unknown"
        && n["aliases"].as_array().unwrap().iter().any(|a| a == "nobody")));
}
