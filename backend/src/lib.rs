pub mod agents;
pub mod chat;
pub mod config;
pub mod conversations;
pub mod db;
pub mod error;
pub mod llm;
pub mod memory;
pub mod routes;
pub mod secrets;
pub mod settings;
pub mod skill_attachments;
pub mod skills;
pub mod telemetry;
pub mod templates;

use crate::{
    agents::AgentsService, conversations::ConversationsService, llm::ProviderRegistry,
    memory::{MemoryService, OpenAiEmbedding}, routes::AppState, settings::SettingsService,
    skill_attachments::AttachmentsService, skills::SkillsService, templates::TemplatesService,
};
use axum::Router;
use sqlx::PgPool;
use std::{path::Path, sync::Arc};

/// Build the Axum router from a live `PgPool` and a directory to host the file-store fallback.
/// Used by `main.rs` and by integration tests.
pub fn build_app(pool: PgPool, secrets_home: &Path) -> Router {
    templates::catalog::validate().expect("starter template catalog must validate");
    let secrets = Arc::new(secrets::auto(secrets_home));
    let providers = ProviderRegistry::new(secrets.clone());
    let settings_svc = SettingsService::new(pool.clone(), secrets.clone());
    let agents_svc = AgentsService::new(pool.clone(), secrets.clone());
    let skills_svc = SkillsService::new(pool.clone());
    let attachments_svc = AttachmentsService::new(pool.clone());
    let templates_svc = TemplatesService::new(pool.clone());
    let conversations_svc = ConversationsService::new(pool.clone());
    let memory_svc =
        MemoryService::new(pool, Arc::new(OpenAiEmbedding::new(secrets)));
    let state = Arc::new(AppState {
        settings: settings_svc,
        providers,
        agents: agents_svc,
        skills: skills_svc,
        attachments: attachments_svc,
        templates: templates_svc,
        conversations: conversations_svc,
        memory: memory_svc,
    });
    routes::router(state)
}

/// Same as `build_app` but lets the caller inject a specific `AnyStore` (e.g., always a
/// `FileStore` under tempdir) so tests don't touch the developer's OS keychain.
pub fn build_app_with_store(pool: PgPool, secrets: Arc<secrets::AnyStore>) -> Router {
    templates::catalog::validate().expect("starter template catalog must validate");
    let providers = ProviderRegistry::new(secrets.clone());
    let settings_svc = SettingsService::new(pool.clone(), secrets.clone());
    let agents_svc = AgentsService::new(pool.clone(), secrets.clone());
    let skills_svc = SkillsService::new(pool.clone());
    let attachments_svc = AttachmentsService::new(pool.clone());
    let templates_svc = TemplatesService::new(pool.clone());
    let conversations_svc = ConversationsService::new(pool.clone());
    let memory_svc =
        MemoryService::new(pool, Arc::new(OpenAiEmbedding::new(secrets)));
    let state = Arc::new(AppState {
        settings: settings_svc,
        providers,
        agents: agents_svc,
        skills: skills_svc,
        attachments: attachments_svc,
        templates: templates_svc,
        conversations: conversations_svc,
        memory: memory_svc,
    });
    routes::router(state)
}
