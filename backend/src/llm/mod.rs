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
}

impl ProviderRegistry {
    pub fn new(secrets: Arc<AnyStore>) -> Self {
        Self { secrets }
    }

    pub fn get(&self, name: ProviderName) -> Box<dyn LlmProvider> {
        match name {
            ProviderName::Anthropic => Box::new(anthropic::Anthropic::new(self.secrets.clone())),
            ProviderName::OpenAi => Box::new(openai::OpenAi::new(self.secrets.clone())),
            ProviderName::OpenAiCompat => {
                Box::new(openai_compat::OpenAiCompat::new(self.secrets.clone()))
            }
        }
    }
}
