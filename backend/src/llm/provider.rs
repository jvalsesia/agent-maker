use async_trait::async_trait;
use serde::{Deserialize, Serialize};
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

#[async_trait]
pub trait LlmProvider: Send + Sync {
    fn name(&self) -> ProviderName;

    /// Cheap probe call: a single-token completion against the cheapest model.
    /// `base_url` is honored when present (mainly for `openai_compat`).
    async fn test(&self, base_url: Option<&str>) -> Result<TestOutcome, ProviderTestError>;
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
        return "***".repeat(1);
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
