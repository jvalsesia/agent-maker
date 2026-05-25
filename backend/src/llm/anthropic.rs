use super::provider::{
    ChatDelta, ChatError, ChatRequest, ChatRole, ChatStream, ChatUsage, LlmProvider, ProviderName,
    ProviderTestError, TestOutcome,
};
use super::streaming::sse_data_stream;
use crate::secrets::{AnyStore, SecretError, SecretStore};
use async_trait::async_trait;
use futures::StreamExt;
use serde_json::{Value, json};
use std::{sync::Arc, time::Duration, time::Instant};

const TEST_MODEL: &str = "claude-haiku-4-5";
const ENDPOINT_DEFAULT: &str = "https://api.anthropic.com/v1/messages";

pub struct Anthropic {
    secrets: Arc<AnyStore>,
    endpoint: String,
}

impl Anthropic {
    pub fn new(secrets: Arc<AnyStore>) -> Self {
        let endpoint = std::env::var("ANTHROPIC_API_BASE")
            .map(|b| format!("{}/v1/messages", b.trim_end_matches('/')))
            .unwrap_or_else(|_| ENDPOINT_DEFAULT.to_string());
        Self { secrets, endpoint }
    }
}

#[async_trait]
impl LlmProvider for Anthropic {
    fn name(&self) -> ProviderName {
        ProviderName::Anthropic
    }

    async fn test(&self, _base_url: Option<&str>) -> Result<TestOutcome, ProviderTestError> {
        let key = match self.secrets.get("anthropic").await {
            Ok(k) => k,
            Err(SecretError::NotFound) => return Err(ProviderTestError::NoKey("anthropic")),
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
            .header("x-api-key", key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
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
            None => match self.secrets.get("anthropic").await {
                Ok(k) => k,
                Err(SecretError::NotFound) => return Err(ChatError::NoKey("anthropic")),
                Err(e) => return Err(ChatError::Backend(e.to_string())),
            },
        };

        // Fold any System turns into the top-level system param; the rest map
        // directly to Anthropic's user/assistant messages.
        let mut system = req.system.unwrap_or_default();
        let mut messages = Vec::new();
        for m in req.messages {
            match m.role {
                ChatRole::System => {
                    if !system.is_empty() {
                        system.push_str("\n\n");
                    }
                    system.push_str(&m.content);
                }
                ChatRole::User => messages.push(json!({"role": "user", "content": m.content})),
                ChatRole::Assistant => {
                    messages.push(json!({"role": "assistant", "content": m.content}))
                }
            }
        }

        let mut body = json!({
            "model": req.model,
            "max_tokens": req.max_tokens,
            "stream": true,
            "messages": messages,
        });
        if !system.is_empty() {
            body["system"] = Value::String(system);
        }

        let client = reqwest::Client::builder()
            .build()
            .map_err(|e| ChatError::Backend(e.to_string()))?;

        let resp = client
            .post(&self.endpoint)
            .header("x-api-key", key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| ChatError::Network(e.to_string()))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(ChatError::Http { status: status.as_u16(), body });
        }

        let stream = sse_data_stream(resp).map(|res| res.map(|payload| parse_anthropic(&payload)));
        Ok(Box::pin(stream))
    }
}

/// Maps one Anthropic SSE payload to a `ChatDelta`. Irrelevant frame types
/// yield an empty delta (harmless to append).
fn parse_anthropic(payload: &str) -> ChatDelta {
    let mut delta = ChatDelta::default();
    let v: Value = match serde_json::from_str(payload) {
        Ok(v) => v,
        Err(_) => return delta,
    };
    match v["type"].as_str() {
        Some("content_block_delta") => {
            if let Some(t) = v["delta"]["text"].as_str() {
                delta.content.push_str(t);
            }
        }
        Some("message_start") => {
            if let Some(it) = v["message"]["usage"]["input_tokens"].as_u64() {
                delta.usage = Some(ChatUsage { input_tokens: it as u32, output_tokens: 0 });
            }
        }
        Some("message_delta") => {
            if let Some(reason) = v["delta"]["stop_reason"].as_str() {
                delta.finish_reason = Some(reason.to_string());
            }
            if let Some(ot) = v["usage"]["output_tokens"].as_u64() {
                delta.usage = Some(ChatUsage { input_tokens: 0, output_tokens: ot as u32 });
            }
        }
        _ => {}
    }
    delta
}
