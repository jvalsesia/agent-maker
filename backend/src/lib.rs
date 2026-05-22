pub mod agents;
pub mod config;
pub mod db;
pub mod error;
pub mod llm;
pub mod routes;
pub mod secrets;
pub mod settings;
pub mod telemetry;

use crate::{llm::ProviderRegistry, routes::AppState, settings::SettingsService};
use axum::Router;
use sqlx::PgPool;
use std::{path::Path, sync::Arc};

/// Build the Axum router from a live `PgPool` and a directory to host the file-store fallback.
/// Used by `main.rs` and by integration tests.
pub fn build_app(pool: PgPool, secrets_home: &Path) -> Router {
    let secrets = Arc::new(secrets::auto(secrets_home));
    let providers = ProviderRegistry::new(secrets.clone());
    let settings_svc = SettingsService::new(pool, secrets);
    let state = Arc::new(AppState { settings: settings_svc, providers });
    routes::router(state)
}

/// Same as `build_app` but lets the caller inject a specific `AnyStore` (e.g., always a
/// `FileStore` under tempdir) so tests don't touch the developer's OS keychain.
pub fn build_app_with_store(pool: PgPool, secrets: Arc<secrets::AnyStore>) -> Router {
    let providers = ProviderRegistry::new(secrets.clone());
    let settings_svc = SettingsService::new(pool, secrets);
    let state = Arc::new(AppState { settings: settings_svc, providers });
    routes::router(state)
}
