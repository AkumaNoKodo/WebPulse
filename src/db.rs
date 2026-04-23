use sqlx::sqlite::{Sqlite, SqlitePoolOptions};
use std::sync::Arc;

use crate::config::DatabaseConfig;

pub type DbPool = sqlx::Pool<Sqlite>;
pub type DbPoolRef = Arc<DbPool>;

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
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS monitors (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            url TEXT NOT NULL,
            check_interval_secs INTEGER NOT NULL DEFAULT 60,
            timeout_secs INTEGER NOT NULL DEFAULT 30,
            status TEXT NOT NULL DEFAULT 'unknown' CHECK(status IN ('up', 'down', 'unknown')),
            last_check_at TEXT,
            last_response_time_ms INTEGER,
            created_at TEXT DEFAULT (datetime('now')),
            updated_at TEXT DEFAULT (datetime('now'))
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_monitors_last_check_at ON monitors(last_check_at)
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_monitors_status ON monitors(status)
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS heartbeats (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            uuid TEXT NOT NULL UNIQUE,
            name TEXT NOT NULL,
            expected_interval_secs INTEGER NOT NULL DEFAULT 300,
            grace_period_secs INTEGER NOT NULL DEFAULT 60,
            status TEXT NOT NULL DEFAULT 'unknown' CHECK(status IN ('healthy', 'late', 'down', 'unknown')),
            last_ping_at TEXT,
            created_at TEXT DEFAULT (datetime('now')),
            updated_at TEXT DEFAULT (datetime('now'))
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_heartbeats_uuid ON heartbeats(uuid)
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_heartbeats_last_ping_at ON heartbeats(last_ping_at)
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_heartbeats_status ON heartbeats(status)
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            monitor_id INTEGER NOT NULL,
            status TEXT NOT NULL CHECK(status IN ('up', 'down')),
            response_time_ms INTEGER,
            error_message TEXT,
            checked_at TEXT DEFAULT (datetime('now')),
            FOREIGN KEY (monitor_id) REFERENCES monitors(id) ON DELETE CASCADE
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_history_monitor_id ON history(monitor_id)
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_history_checked_at ON history(checked_at)
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_history_monitor_checked ON history(monitor_id, checked_at)
        "#,
    )
    .execute(pool)
    .await?;

    tracing::info!("Database migrations completed");

    Ok(())
}

pub async fn close_pool(pool: &DbPool) -> anyhow::Result<()> {
    pool.close().await;
    Ok(())
}
