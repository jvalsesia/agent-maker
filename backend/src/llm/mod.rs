pub mod anthropic;
pub mod openai;
pub mod openai_compat;
pub mod provider;
pub mod streaming;

pub use provider::{
    ChatDelta, ChatError, ChatMessage, ChatRequest, ChatRole, ChatStream, ChatUsage, LlmProvider,
    ProviderName,
};

use crate::secrets::AnyStore;
use std::sync::Arc;

#[derive(Clone)]
pub struct ProviderRegistry {
    pub secrets: Arc<AnyStore>,
    /// When set, `get` returns this provider regardless of name. Used by tests
    /// to inject a stub streaming provider without live network calls.
    override_provider: Option<Arc<dyn LlmProvider>>,
}

impl ProviderRegistry {
    pub fn new(secrets: Arc<AnyStore>) -> Self {
        Self { secrets, override_provider: None }
    }

    /// Build a registry that always hands out `provider` (test seam).
    pub fn with_override(secrets: Arc<AnyStore>, provider: Arc<dyn LlmProvider>) -> Self {
        Self { secrets, override_provider: Some(provider) }
    }

    pub fn get(&self, name: ProviderName) -> Arc<dyn LlmProvider> {
        if let Some(p) = &self.override_provider {
            return p.clone();
        }
        match name {
            ProviderName::Anthropic => Arc::new(anthropic::Anthropic::new(self.secrets.clone())),
            ProviderName::OpenAi => Arc::new(openai::OpenAi::new(self.secrets.clone())),
            ProviderName::OpenAiCompat => {
                Arc::new(openai_compat::OpenAiCompat::new(self.secrets.clone()))
            }
        }
    }
}
