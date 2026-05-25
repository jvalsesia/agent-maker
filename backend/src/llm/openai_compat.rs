use super::openai::{openai_messages, parse_openai};
use super::provider::{
    ChatError, ChatRequest, ChatStream, LlmProvider, ProviderName, ProviderTestError, TestOutcome,
};
use super::streaming::sse_data_stream;
use crate::secrets::{AnyStore, SecretStore};
use async_trait::async_trait;
use futures::StreamExt;
use serde_json::json;
use std::{sync::Arc, time::Duration, time::Instant};

const DEFAULT_BASE: &str = "http://localhost:11434/v1";
const TEST_MODEL: &str = "llama3.2:1b";

pub struct OpenAiCompat {
    secrets: Arc<AnyStore>,
}

impl OpenAiCompat {
    pub fn new(secrets: Arc<AnyStore>) -> Self {
        Self { secrets }
    }
}

#[async_trait]
impl LlmProvider for OpenAiCompat {
    fn name(&self) -> ProviderName {
        ProviderName::OpenAiCompat
    }

    async fn test(&self, base_url: Option<&str>) -> Result<TestOutcome, ProviderTestError> {
        let key = self.secrets.get("openai_compat").await.unwrap_or_default();
        let base = base_url.unwrap_or(DEFAULT_BASE).trim_end_matches('/');
        let endpoint = format!("{base}/chat/completions");

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
        let mut req = client.post(&endpoint).json(&body);
        if !key.is_empty() {
            req = req.bearer_auth(key);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| ProviderTestError::Network(e.to_string()))?;

        let status = resp.status();
        let latency_ms = started.elapsed().as_millis();
        if status.is_success() {
            Ok(TestOutcome { ok: true, model_used: TEST_MODEL.to_string(), latency_ms })
        } else {
            let body = resp.text().await.unwrap_or_default();
            // For local endpoints, model-not-installed is a common failure mode; surface verbatim.
            Err(ProviderTestError::Http { status: status.as_u16(), body })
        }
    }

    async fn chat(&self, req: ChatRequest) -> Result<ChatStream, ChatError> {
        // Local endpoints often need no key; fall back to the configured one.
        let key = match req.api_key {
            Some(k) => k,
            None => self.secrets.get("openai_compat").await.unwrap_or_default(),
        };
        let base = req.base_url.as_deref().unwrap_or(DEFAULT_BASE);
        let endpoint = format!("{}/chat/completions", base.trim_end_matches('/'));

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

        let mut request = client.post(&endpoint).json(&body);
        if !key.is_empty() {
            request = request.bearer_auth(key);
        }
        let resp = request
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

