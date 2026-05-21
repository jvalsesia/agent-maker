use sqlx::{PgPool, postgres::PgPoolOptions};
use std::time::Duration;
use tokio::time::sleep;

pub async fn init(database_url: &str) -> anyhow::Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(8)
        .acquire_timeout(Duration::from_secs(5))
        .connect_lazy(database_url)?;

    wait_ready(&pool).await?;

    sqlx::migrate!("./migrations").run(&pool).await?;
    tracing::info!("database migrations applied");
    Ok(pool)
}

async fn wait_ready(pool: &PgPool) -> anyhow::Result<()> {
    let deadline = std::time::Instant::now() + Duration::from_secs(30);
    let mut attempt = 0u32;
    loop {
        attempt += 1;
        match sqlx::query("SELECT 1").execute(pool).await {
            Ok(_) => {
                tracing::info!(attempt, "database reachable");
                return Ok(());
            }
            Err(e) if std::time::Instant::now() < deadline => {
                tracing::debug!(attempt, error = %e, "waiting for database");
                sleep(Duration::from_secs(1)).await;
            }
            Err(e) => {
                anyhow::bail!("database unreachable after 30s: {e}");
            }
        }
    }
}
