pub mod settings;

use crate::{llm::ProviderRegistry, settings::SettingsService};
use axum::Router;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub settings: SettingsService,
    pub providers: ProviderRegistry,
}

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .nest("/api", settings::routes())
        .with_state(state)
}
