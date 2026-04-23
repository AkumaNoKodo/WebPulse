use sqlx::sqlite::{Sqlite, SqlitePoolOptions};

use crate::config::DatabaseConfig;

pub type DbPool = sqlx::Pool<Sqlite>;

pub async fn create_pool(config: &DatabaseConfig) -> anyhow::Result<DbPool> {
    let db_url = format!("sqlite:{}?mode=rwc", config.path);

    let pool = SqlitePoolOptions::new()
        .max_connections(config.max_connections)
        .connect(&db_url)
        .await
        .map_err(|e| {
            anyhow::anyhow!(
                "Failed to create database pool: {} - path: {}",
                e,
                config.path
            )
        })?;

    Ok(pool)
}

pub async fn run_migrations(pool: &DbPool) -> anyhow::Result<()> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .map_err(|e| anyhow::anyhow!("Migration failed: {}", e))?;

    tracing::info!("Database migrations completed");

    Ok(())
}
