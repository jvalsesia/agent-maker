use tracing_subscriber::{EnvFilter, fmt, prelude::*};

pub fn init() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("agent_maker=info,tower_http=info,sqlx=warn"));

    let registry = tracing_subscriber::registry().with(filter);

    if cfg!(debug_assertions) {
        registry.with(fmt::layer().pretty()).init();
    } else {
        registry.with(fmt::layer().json()).init();
    }
}
