use agent_maker::{
    auth::{AuthConfig, AuthState},
    build_app,
    config::Config,
    db, telemetry,
};
use tower_http::trace::TraceLayer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    telemetry::init();
    let cfg = Config::from_env()?;
    tracing::info!(?cfg.bind_addr, "starting agent-maker backend");

    let auth = AuthState::new(AuthConfig {
        jwks_url: cfg.clerk_jwks_url.clone(),
        issuer: cfg.clerk_issuer.clone(),
    });
    tracing::info!(auth_enabled = auth.enabled(), "authentication mode resolved");

    let pool = db::init(&cfg.database_url).await?;
    let mut app = build_app(pool, &cfg.agent_maker_home, auth, cfg.cors_allowed_origin.clone())
        .layer(TraceLayer::new_for_http());

    if let Some(dist) = cfg.serve_frontend_dist.as_ref()
        && dist.exists() {
            use tower_http::services::{ServeDir, ServeFile};
            let index = dist.join("index.html");
            let serve = ServeDir::new(dist).not_found_service(ServeFile::new(index));
            app = app.fallback_service(serve);
            tracing::info!(path = %dist.display(), "serving frontend dist");
        }

    let listener = tokio::net::TcpListener::bind(cfg.bind_addr).await?;
    tracing::info!(addr = %cfg.bind_addr, "listening");
    axum::serve(listener, app).await?;
    Ok(())
}
