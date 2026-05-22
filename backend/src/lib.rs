pub mod agents;
pub mod config;
pub mod db;
pub mod error;
pub mod llm;
pub mod routes;
pub mod secrets;
pub mod settings;
pub mod skill_attachments;
pub mod skills;
pub mod telemetry;

use crate::{
    agents::AgentsService, llm::ProviderRegistry, routes::AppState, settings::SettingsService,
    skill_attachments::AttachmentsService, skills::SkillsService,
};
use axum::Router;
use sqlx::PgPool;
use std::{path::Path, sync::Arc};

/// Build the Axum router from a live `PgPool` and a directory to host the file-store fallback.
/// Used by `main.rs` and by integration tests.
pub fn build_app(pool: PgPool, secrets_home: &Path) -> Router {
    let secrets = Arc::new(secrets::auto(secrets_home));
    let providers = ProviderRegistry::new(secrets.clone());
    let settings_svc = SettingsService::new(pool.clone(), secrets.clone());
    let agents_svc = AgentsService::new(pool.clone(), secrets);
    let skills_svc = SkillsService::new(pool.clone());
    let attachments_svc = AttachmentsService::new(pool);
    let state = Arc::new(AppState {
        settings: settings_svc,
        providers,
        agents: agents_svc,
        skills: skills_svc,
        attachments: attachments_svc,
    });
    routes::router(state)
}

/// Same as `build_app` but lets the caller inject a specific `AnyStore` (e.g., always a
/// `FileStore` under tempdir) so tests don't touch the developer's OS keychain.
pub fn build_app_with_store(pool: PgPool, secrets: Arc<secrets::AnyStore>) -> Router {
    let providers = ProviderRegistry::new(secrets.clone());
    let settings_svc = SettingsService::new(pool.clone(), secrets.clone());
    let agents_svc = AgentsService::new(pool.clone(), secrets);
    let skills_svc = SkillsService::new(pool.clone());
    let attachments_svc = AttachmentsService::new(pool);
    let state = Arc::new(AppState {
        settings: settings_svc,
        providers,
        agents: agents_svc,
        skills: skills_svc,
        attachments: attachments_svc,
    });
    routes::router(state)
}
