pub mod agents;
pub mod attachments;
pub mod settings;
pub mod skills;

use crate::{
    agents::AgentsService, llm::ProviderRegistry, settings::SettingsService,
    skill_attachments::AttachmentsService, skills::SkillsService, templates::TemplatesService,
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
}

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .nest(
            "/api",
            settings::routes()
                .merge(agents::routes())
                .merge(skills::routes())
                .merge(attachments::routes()),
        )
        .with_state(state)
}
