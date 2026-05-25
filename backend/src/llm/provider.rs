use async_trait::async_trait;
use futures::Stream;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderName {
    Anthropic,
    #[serde(rename = "openai")]
    OpenAi,
    #[serde(rename = "openai_compat")]
    OpenAiCompat,
}

impl ProviderName {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProviderName::Anthropic => "anthropic",
            ProviderName::OpenAi => "openai",
            ProviderName::OpenAiCompat => "openai_compat",
        }
    }
}

impl FromStr for ProviderName {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "anthropic" => Ok(Self::Anthropic),
            "openai" => Ok(Self::OpenAi),
            "openai_compat" => Ok(Self::OpenAiCompat),
            other => Err(format!("unknown provider {other}")),
        }
    }
}

pub const ALL_PROVIDERS: [ProviderName; 3] = [
    ProviderName::Anthropic,
    ProviderName::OpenAi,
    ProviderName::OpenAiCompat,
];

#[derive(Debug, Clone, Serialize)]
pub struct TestOutcome {
    pub ok: bool,
    pub model_used: String,
    pub latency_ms: u128,
}

/// One message in a chat request. `System` content is folded into the
/// provider's system parameter by each implementation, so callers typically
/// pass only `User`/`Assistant` turns plus a separate `system` string.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChatRole {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
}

/// A fully composed chat request ready to dispatch. The composer (F07) builds
/// `system` and `messages`; `api_key` overrides the provider's default key
/// (per-agent key fallback), and `base_url` targets a local endpoint.
#[derive(Debug, Clone)]
pub struct ChatRequest {
    pub model: String,
    pub system: Option<String>,
    pub messages: Vec<ChatMessage>,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub max_tokens: u32,
}

/// Token usage as reported by a provider. Fields are `0` when unknown; the
/// caller merges successive deltas by keeping the last non-zero value.
#[derive(Debug, Clone, Copy, Default)]
pub struct ChatUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

/// One streamed increment: a text delta plus optional usage / finish metadata
/// that providers attach to their final frames.
#[derive(Debug, Clone, Default)]
pub struct ChatDelta {
    pub content: String,
    pub usage: Option<ChatUsage>,
    pub finish_reason: Option<String>,
}

pub type ChatStream = Pin<Box<dyn Stream<Item = Result<ChatDelta, ChatError>> + Send>>;

#[derive(Debug, thiserror::Error)]
pub enum ChatError {
    #[error("no key configured for {0}")]
    NoKey(&'static str),
    #[error("provider returned {status}: {body}")]
    Http { status: u16, body: String },
    #[error("network error: {0}")]
    Network(String),
    #[error("backend error: {0}")]
    Backend(String),
    #[error("stream error: {0}")]
    Stream(String),
}

#[async_trait]
pub trait LlmProvider: Send + Sync {
    fn name(&self) -> ProviderName;

    /// Cheap probe call: a single-token completion against the cheapest model.
    /// `base_url` is honored when present (mainly for `openai_compat`).
    async fn test(&self, base_url: Option<&str>) -> Result<TestOutcome, ProviderTestError>;

    /// Open a streaming completion. The outer `Result` reports preflight
    /// failures (auth, unknown model, network) before any token streams; the
    /// returned stream then yields text deltas until the response completes.
    async fn chat(&self, req: ChatRequest) -> Result<ChatStream, ChatError>;
}

#[derive(Debug, thiserror::Error)]
pub enum ProviderTestError {
    #[error("no key configured for {0}")]
    NoKey(&'static str),
    #[error("provider returned {status}: {body}")]
    Http { status: u16, body: String },
    #[error("network error: {0}")]
    Network(String),
    #[error("backend error: {0}")]
    Backend(String),
}

/// Masks an API key as `<first7>***...<last4>`, matching the spec's mask rule.
pub fn mask_key(key: &str) -> String {
    let chars: Vec<char> = key.chars().collect();
    if chars.len() <= 11 {
        return "***".to_string();
    }
    let prefix: String = chars[..7].iter().collect();
    let suffix: String = chars[chars.len() - 4..].iter().collect();
    format!("{prefix}***...{suffix}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masking_first7_last4() {
        assert_eq!(mask_key("sk-ant-api03-XYZ123ABC"), "sk-ant-***...3ABC");
    }
}
