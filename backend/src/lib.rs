pub mod agents;
pub mod auth;
pub mod chat;
pub mod config;
pub mod conversations;
pub mod db;
pub mod error;
pub mod i18n;
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
    agents::AgentsService, auth::AuthService, chat::ChatService,
    config::AuthConfig, conversations::ConversationsService,
    llm::{LlmProvider, ProviderRegistry},
    memory::{EmbeddingProvider, MemoryService, OpenAiEmbedding}, routes::AppState,
    settings::SettingsService, skill_attachments::AttachmentsService, skills::SkillsService,
    templates::TemplatesService,
};
use axum::Router;
use sqlx::PgPool;
use std::{path::Path, sync::Arc};

/// Assemble `AppState` from a pool, secret store, provider registry, and
/// embedding provider. Shared by all builders so the wiring stays in one place.
fn build_state(
    pool: PgPool,
    secrets: Arc<secrets::AnyStore>,
    providers: ProviderRegistry,
    embedder: Arc<dyn EmbeddingProvider>,
    auth: AuthService,
) -> Arc<AppState> {
    templates::catalog::validate().expect("starter template catalog must validate");
    let settings_svc = SettingsService::new(pool.clone(), secrets.clone());
    let agents_svc = AgentsService::new(pool.clone(), secrets.clone());
    let skills_svc = SkillsService::new(pool.clone());
    let attachments_svc = AttachmentsService::new(pool.clone());
    let templates_svc = TemplatesService::new(pool.clone());
    let conversations_svc = ConversationsService::new(pool.clone());
    let memory_svc = MemoryService::new(pool.clone(), embedder);
    let chat_svc = ChatService::new(
        pool,
        providers.clone(),
        agents_svc.clone(),
        conversations_svc.clone(),
        memory_svc.clone(),
        attachments_svc.clone(),
        secrets,
    );
    Arc::new(AppState {
        settings: settings_svc,
        providers,
        agents: agents_svc,
        skills: skills_svc,
        attachments: attachments_svc,
        templates: templates_svc,
        conversations: conversations_svc,
        memory: memory_svc,
        chat: chat_svc,
        auth,
    })
}

/// Build the Axum router from a live `PgPool` and a directory to host the file-store fallback.
/// Used by `main.rs` and by integration tests.
pub fn build_app(pool: PgPool, secrets_home: &Path) -> Router {
    let secrets = Arc::new(secrets::auto(secrets_home));
    let providers = ProviderRegistry::new(secrets.clone());
    let embedder = Arc::new(OpenAiEmbedding::new(secrets.clone()));
    let auth_cfg = AuthConfig::from_env().expect("AuthConfig::from_env");
    let auth = AuthService::new(auth_cfg, reqwest::Client::new());
    routes::router(build_state(pool, secrets, providers, embedder, auth))
}

/// Same as `build_app` but lets the caller inject a specific `AnyStore` (e.g., always a
/// `FileStore` under tempdir) so tests don't touch the developer's OS keychain.
pub fn build_app_with_store(pool: PgPool, secrets: Arc<secrets::AnyStore>) -> Router {
    let providers = ProviderRegistry::new(secrets.clone());
    let embedder = Arc::new(OpenAiEmbedding::new(secrets.clone()));
    let auth = AuthService::for_tests();
    routes::router(build_state(pool, secrets, providers, embedder, auth))
}

/// Build the app with a stub streaming provider and embedder (chat integration
/// tests), so no live LLM/embedding network calls are made.
pub fn build_app_for_test(
    pool: PgPool,
    secrets: Arc<secrets::AnyStore>,
    provider: Arc<dyn LlmProvider>,
    embedder: Arc<dyn EmbeddingProvider>,
) -> Router {
    let providers = ProviderRegistry::with_override(secrets.clone(), provider);
    let auth = AuthService::for_tests();
    routes::router(build_state(pool, secrets, providers, embedder, auth))
}
