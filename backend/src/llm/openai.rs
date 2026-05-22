use super::provider::{LlmProvider, ProviderName, ProviderTestError, TestOutcome};
use crate::secrets::{AnyStore, SecretError, SecretStore};
use async_trait::async_trait;
use serde_json::json;
use std::{sync::Arc, time::Duration, time::Instant};

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
}
