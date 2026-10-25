use std::time::Duration;

use sqlx::postgres::{PgPool, PgPoolOptions};

use crate::config::Config;

pub async fn connect(cfg: &Config) -> anyhow::Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(10))
        .connect(&cfg.database_url)
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}
