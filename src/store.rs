use sqlx::FromRow;

use crate::config::MonitorConfig;
use crate::db::DbPool;
use crate::error::{AppError, AppResult};
use crate::models::{CreateMonitor, History, Monitor, UpdateMonitor};
use crate::services::checker::CheckResult;

#[derive(Debug, Clone)]
pub struct AppState {
    pub pool: DbPool,
    pub monitor: MonitorConfig,
    pub http: reqwest::Client,
}

#[derive(Debug, FromRow)]
pub struct Stats {
    pub total: i64,
    pub online: i64,
    pub offline: i64,
    pub unknown: i64,
}

pub async fn list_monitors(state: &AppState) -> AppResult<Vec<Monitor>> {
    let monitors = sqlx::query_as::<_, Monitor>("SELECT * FROM monitors ORDER BY id")
        .fetch_all(&state.pool)
        .await?;

    Ok(monitors)
}

pub async fn get_monitor(state: &AppState, id: i64) -> AppResult<Monitor> {
    sqlx::query_as::<_, Monitor>("SELECT * FROM monitors WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Monitor {id} not found")))
}

pub async fn create_monitor(state: &AppState, input: CreateMonitor) -> AppResult<Monitor> {
    let interval = input
        .check_interval_secs
        .unwrap_or(state.monitor.default_interval_secs);
    let timeout = input
        .timeout_secs
        .unwrap_or(state.monitor.default_timeout_secs);

    validate(&input.url, interval, timeout)?;

    let monitor = sqlx::query_as::<_, Monitor>(
        "INSERT INTO monitors (name, url, check_interval_secs, timeout_secs) \
         VALUES (?, ?, ?, ?) RETURNING *",
    )
    .bind(&input.name)
    .bind(&input.url)
    .bind(interval)
    .bind(timeout)
    .fetch_one(&state.pool)
    .await?;

    Ok(monitor)
}

pub async fn update_monitor(state: &AppState, id: i64, input: UpdateMonitor) -> AppResult<Monitor> {
    let existing = get_monitor(state, id).await?;

    let name = input.name.unwrap_or(existing.name);
    let url = input.url.unwrap_or(existing.url);
    let interval = input
        .check_interval_secs
        .unwrap_or(existing.check_interval_secs);
    let timeout = input.timeout_secs.unwrap_or(existing.timeout_secs);

    validate(&url, interval, timeout)?;

    let monitor = sqlx::query_as::<_, Monitor>(
        "UPDATE monitors \
         SET name = ?, url = ?, check_interval_secs = ?, timeout_secs = ?, updated_at = datetime('now') \
         WHERE id = ? RETURNING *",
    )
    .bind(&name)
    .bind(&url)
    .bind(interval)
    .bind(timeout)
    .bind(id)
    .fetch_one(&state.pool)
    .await?;

    Ok(monitor)
}

pub async fn delete_monitor(state: &AppState, id: i64) -> AppResult<()> {
    let result = sqlx::query("DELETE FROM monitors WHERE id = ?")
        .bind(id)
        .execute(&state.pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("Monitor {id} not found")));
    }

    Ok(())
}

/// Oldest first, so the caller can render it left to right.
pub async fn history(state: &AppState, monitor_id: i64) -> AppResult<Vec<History>> {
    let history = sqlx::query_as::<_, History>(
        "SELECT * FROM (SELECT * FROM history WHERE monitor_id = ? ORDER BY id DESC LIMIT ?) \
         ORDER BY id ASC",
    )
    .bind(monitor_id)
    .bind(state.monitor.history_retention_count)
    .fetch_all(&state.pool)
    .await?;

    Ok(history)
}

pub async fn clear_history(state: &AppState, monitor_id: i64) -> AppResult<()> {
    let mut tx = state.pool.begin().await?;

    sqlx::query("DELETE FROM history WHERE monitor_id = ?")
        .bind(monitor_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query(
        "UPDATE monitors \
         SET status = 'unknown', last_check_at = NULL, last_response_time_ms = NULL, \
             updated_at = datetime('now') \
         WHERE id = ?",
    )
    .bind(monitor_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(())
}

pub async fn record_check(
    state: &AppState,
    monitor_id: i64,
    result: &CheckResult,
) -> AppResult<()> {
    let mut tx = state.pool.begin().await?;

    sqlx::query(
        "INSERT INTO history (monitor_id, status, response_time_ms, error_message) \
         VALUES (?, ?, ?, ?)",
    )
    .bind(monitor_id)
    .bind(result.status)
    .bind(result.response_time_ms)
    .bind(result.error_message.as_deref())
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "UPDATE monitors \
         SET status = ?, last_check_at = datetime('now'), last_response_time_ms = ?, \
             updated_at = datetime('now') \
         WHERE id = ?",
    )
    .bind(crate::models::MonitorStatus::from(result.status))
    .bind(result.response_time_ms)
    .bind(monitor_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "DELETE FROM history WHERE monitor_id = ?1 AND id NOT IN \
         (SELECT id FROM history WHERE monitor_id = ?1 ORDER BY id DESC LIMIT ?2)",
    )
    .bind(monitor_id)
    .bind(state.monitor.history_retention_count)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(())
}

/// Marks every due monitor as checked and returns it in one statement, so a probe
/// that outlives its own interval cannot be started a second time by the next tick.
pub async fn claim_due_monitors(state: &AppState) -> AppResult<Vec<Monitor>> {
    let monitors = sqlx::query_as::<_, Monitor>(
        "UPDATE monitors SET last_check_at = datetime('now'), updated_at = datetime('now') \
         WHERE last_check_at IS NULL \
            OR datetime(last_check_at, '+' || check_interval_secs || ' seconds') <= datetime('now') \
         RETURNING *",
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(monitors)
}

pub async fn stats(state: &AppState) -> AppResult<Stats> {
    let stats = sqlx::query_as::<_, Stats>(
        "SELECT COUNT(*) AS total, \
                COUNT(*) FILTER (WHERE status = 'up') AS online, \
                COUNT(*) FILTER (WHERE status = 'down') AS offline, \
                COUNT(*) FILTER (WHERE status = 'unknown') AS unknown \
         FROM monitors",
    )
    .fetch_one(&state.pool)
    .await?;

    Ok(stats)
}

fn validate(url: &str, check_interval_secs: i64, timeout_secs: i64) -> AppResult<()> {
    let parsed = reqwest::Url::parse(url)
        .map_err(|e| AppError::BadRequest(format!("Invalid URL '{url}': {e}")))?;

    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(AppError::BadRequest(format!(
            "URL '{url}' must use http or https"
        )));
    }

    if check_interval_secs < 1 {
        return Err(AppError::BadRequest(
            "Check interval must be at least 1 second".to_string(),
        ));
    }

    if timeout_secs < 1 {
        return Err(AppError::BadRequest(
            "Timeout must be at least 1 second".to_string(),
        ));
    }

    Ok(())
}
