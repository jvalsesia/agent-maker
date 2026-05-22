pub mod agents;
pub mod settings;

use crate::{agents::AgentsService, llm::ProviderRegistry, settings::SettingsService};
use axum::Router;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub settings: SettingsService,
    pub providers: ProviderRegistry,
    pub agents: AgentsService,
}

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .nest("/api", settings::routes().merge(agents::routes()))
        .with_state(state)
}
