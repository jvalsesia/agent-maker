use super::provider::{
    ChatDelta, ChatError, ChatRequest, ChatStream, ChatUsage, LlmProvider, ProviderName,
    ProviderTestError, TestOutcome,
};
use super::streaming::sse_data_stream;
use crate::secrets::{AnyStore, SecretError, SecretStore};
use async_trait::async_trait;
use futures::StreamExt;
use serde_json::{Value, json};
use std::{sync::Arc, time::Duration, time::Instant};

/// Builds the OpenAI-style `messages` array, prepending `system` as a system
/// message when present. Shared by the OpenAI and OpenAI-compatible providers.
pub(super) fn openai_messages(
    system: Option<String>,
    messages: Vec<super::provider::ChatMessage>,
) -> Vec<Value> {
    use super::provider::ChatRole;
    let mut out = Vec::new();
    if let Some(sys) = system.filter(|s| !s.is_empty()) {
        out.push(json!({"role": "system", "content": sys}));
    }
    for m in messages {
        let role = match m.role {
            ChatRole::System => "system",
            ChatRole::User => "user",
            ChatRole::Assistant => "assistant",
        };
        out.push(json!({"role": role, "content": m.content}));
    }
    out
}

/// Maps one OpenAI-style SSE payload to a `ChatDelta`. Shared by both
/// OpenAI-compatible providers; usage-only frames carry empty `choices`.
pub(super) fn parse_openai(payload: &str) -> ChatDelta {
    let mut delta = ChatDelta::default();
    let v: Value = match serde_json::from_str(payload) {
        Ok(v) => v,
        Err(_) => return delta,
    };
    if let Some(choice) = v["choices"].as_array().and_then(|c| c.first()) {
        if let Some(t) = choice["delta"]["content"].as_str() {
            delta.content.push_str(t);
        }
        if let Some(reason) = choice["finish_reason"].as_str() {
            delta.finish_reason = Some(reason.to_string());
        }
    }
    if let Some(usage) = v.get("usage").filter(|u| !u.is_null()) {
        delta.usage = Some(ChatUsage {
            input_tokens: usage["prompt_tokens"].as_u64().unwrap_or(0) as u32,
            output_tokens: usage["completion_tokens"].as_u64().unwrap_or(0) as u32,
        });
    }
    delta
}

const TEST_MODEL: &str = "gpt-4o-mini";
const ENDPOINT_DEFAULT: &str = "https://api.openai.com/v1/chat/completions";

pub struct OpenAi {
    secrets: Arc<AnyStore>,
    endpoint: String,
}

impl OpenAi {
    pub fn new(secrets: Arc<AnyStore>) -> Self {
        let endpoint = std::env::var("OPENAI_API_BASE")
            .map(|b| format!("{}/v1/chat/completions", b.trim_end_matches('/')))
            .unwrap_or_else(|_| ENDPOINT_DEFAULT.to_string());
        Self { secrets, endpoint }
    }
}

#[async_trait]
impl LlmProvider for OpenAi {
    fn name(&self) -> ProviderName {
        ProviderName::OpenAi
    }

    async fn test(&self, _base_url: Option<&str>) -> Result<TestOutcome, ProviderTestError> {
        let key = match self.secrets.get("openai").await {
            Ok(k) => k,
            Err(SecretError::NotFound) => return Err(ProviderTestError::NoKey("openai")),
            Err(e) => return Err(ProviderTestError::Backend(e.to_string())),
        };

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| ProviderTestError::Backend(e.to_string()))?;

        let body = json!({
            "model": TEST_MODEL,
            "max_tokens": 1,
            "messages": [{"role": "user", "content": "."}]
        });

        let started = Instant::now();
        let resp = client
            .post(&self.endpoint)
            .bearer_auth(key)
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderTestError::Network(e.to_string()))?;

        let status = resp.status();
        let latency_ms = started.elapsed().as_millis();
        if status.is_success() {
            Ok(TestOutcome { ok: true, model_used: TEST_MODEL.to_string(), latency_ms })
        } else {
            let body = resp.text().await.unwrap_or_default();
            Err(ProviderTestError::Http { status: status.as_u16(), body })
        }
    }

    async fn chat(&self, req: ChatRequest) -> Result<ChatStream, ChatError> {
        let key = match req.api_key {
            Some(k) => k,
            None => match self.secrets.get("openai").await {
                Ok(k) => k,
                Err(SecretError::NotFound) => return Err(ChatError::NoKey("openai")),
                Err(e) => return Err(ChatError::Backend(e.to_string())),
            },
        };

        let body = json!({
            "model": req.model,
            "max_tokens": req.max_tokens,
            "stream": true,
            "stream_options": {"include_usage": true},
            "messages": openai_messages(req.system, req.messages),
        });

        let client = reqwest::Client::builder()
            .build()
            .map_err(|e| ChatError::Backend(e.to_string()))?;

        let resp = client
            .post(&self.endpoint)
            .bearer_auth(key)
            .json(&body)
            .send()
            .await
            .map_err(|e| ChatError::Network(e.to_string()))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(ChatError::Http { status: status.as_u16(), body });
        }

        let stream = sse_data_stream(resp).map(|res| res.map(|payload| parse_openai(&payload)));
        Ok(Box::pin(stream))
    }
}
