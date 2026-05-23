use crate::secrets::{AnyStore, SecretError, SecretStore};
use async_trait::async_trait;
use serde_json::json;
use std::{sync::Arc, time::Duration};

use super::model::EMBEDDING_MODEL;

const ENDPOINT_DEFAULT: &str = "https://api.openai.com/v1/embeddings";

#[derive(Debug, thiserror::Error)]
pub enum EmbeddingError {
    #[error("no openai key configured")]
    NoKey,
    #[error("provider error {status}: {body}")]
    Http { status: u16, body: String },
    #[error("network error: {0}")]
    Network(String),
    #[error("backend error: {0}")]
    Backend(String),
}

#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    async fn embed_one(&self, text: &str) -> Result<Vec<f32>, EmbeddingError>;
}

pub struct OpenAiEmbedding {
    secrets: Arc<AnyStore>,
    endpoint: String,
}

impl OpenAiEmbedding {
    pub fn new(secrets: Arc<AnyStore>) -> Self {
        let endpoint = std::env::var("OPENAI_API_BASE")
            .map(|b| format!("{}/v1/embeddings", b.trim_end_matches('/')))
            .unwrap_or_else(|_| ENDPOINT_DEFAULT.to_string());
        Self { secrets, endpoint }
    }
}

#[async_trait]
impl EmbeddingProvider for OpenAiEmbedding {
    async fn embed_one(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        let key = match self.secrets.get("openai").await {
            Ok(k) => k,
            Err(SecretError::NotFound) => return Err(EmbeddingError::NoKey),
            Err(e) => return Err(EmbeddingError::Backend(e.to_string())),
        };
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(20))
            .build()
            .map_err(|e| EmbeddingError::Backend(e.to_string()))?;

        let body = json!({ "model": EMBEDDING_MODEL, "input": text });
        let resp = client
            .post(&self.endpoint)
            .bearer_auth(key)
            .json(&body)
            .send()
            .await
            .map_err(|e| EmbeddingError::Network(e.to_string()))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(EmbeddingError::Http { status: status.as_u16(), body });
        }
        let parsed: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| EmbeddingError::Backend(e.to_string()))?;
        let arr = parsed
            .get("data")
            .and_then(|d| d.get(0))
            .and_then(|d| d.get("embedding"))
            .and_then(|e| e.as_array())
            .ok_or_else(|| {
                EmbeddingError::Backend("missing data[0].embedding in response".into())
            })?;
        let mut out = Vec::with_capacity(arr.len());
        for v in arr {
            out.push(v.as_f64().unwrap_or(0.0) as f32);
        }
        Ok(out)
    }
}
