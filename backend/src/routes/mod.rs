pub mod agents;
pub mod attachments;
pub mod auth;
pub mod chat;
pub mod conversations;
pub mod memory;
pub mod settings;
pub mod skills;
pub mod templates;

use crate::{
    agents::AgentsService, auth::AuthService, chat::ChatService,
    conversations::ConversationsService, llm::ProviderRegistry, memory::MemoryService,
    settings::SettingsService, skill_attachments::AttachmentsService, skills::SkillsService,
    templates::TemplatesService,
};
use axum::Router;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub settings: SettingsService,
    pub providers: ProviderRegistry,
    pub agents: AgentsService,
    pub skills: SkillsService,
    pub attachments: AttachmentsService,
    pub templates: TemplatesService,
    pub conversations: ConversationsService,
    pub memory: MemoryService,
    pub chat: ChatService,
    pub auth: AuthService,
}

pub fn router(state: Arc<AppState>) -> Router {
    let api = settings::routes()
        .merge(agents::routes())
        .merge(skills::routes())
        .merge(attachments::routes())
        .merge(templates::routes())
        .merge(conversations::routes())
        .merge(memory::routes())
        .merge(chat::routes())
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::auth::require_auth,
        ));

    Router::new()
        .nest("/api", api)
        .nest("/auth", auth::routes())
        .with_state(state)
}
