mod config;
mod db;
mod error;
mod llm;
mod routes;
mod secrets;
mod settings;
mod telemetry;

use crate::{
    config::Config,
    llm::ProviderRegistry,
    routes::AppState,
    settings::SettingsService,
};
use std::sync::Arc;
use tower_http::trace::TraceLayer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    telemetry::init();
    let cfg = Config::from_env()?;
    tracing::info!(?cfg.bind_addr, "starting agent-maker backend");

    let pool = db::init(&cfg.database_url).await?;
    let secrets = Arc::new(secrets::auto(&cfg.agent_maker_home));
    let providers = ProviderRegistry::new(secrets.clone());
    let settings_svc = SettingsService::new(pool, secrets);
    let state = Arc::new(AppState { settings: settings_svc, providers });

    let mut app = routes::router(state).layer(TraceLayer::new_for_http());

    if let Some(dist) = cfg.serve_frontend_dist.as_ref() {
        if dist.exists() {
            use tower_http::services::{ServeDir, ServeFile};
            let index = dist.join("index.html");
            let serve = ServeDir::new(dist).not_found_service(ServeFile::new(index));
            app = app.fallback_service(serve);
            tracing::info!(path = %dist.display(), "serving frontend dist");
        }
    }

    let listener = tokio::net::TcpListener::bind(cfg.bind_addr).await?;
    tracing::info!(addr = %cfg.bind_addr, "listening");
    axum::serve(listener, app).await?;
    Ok(())
}
