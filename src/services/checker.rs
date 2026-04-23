use async_trait::async_trait;

use crate::db::DbPool;
use crate::error::AppResult;
use crate::models::{CreateHistory, HistoryStatus, MonitorStatus};

#[async_trait]
pub trait Checkable: Send + Sync {
    async fn check(&self, pool: &DbPool) -> AppResult<CheckResult>;

    fn id(&self) -> i64;
    fn name(&self) -> &str;
}

#[derive(Debug, Clone)]
pub struct CheckResult {
    pub status: MonitorStatus,
    pub response_time_ms: Option<i64>,
    pub error_message: Option<String>,
}

pub async fn record_check_result(
    pool: &DbPool,
    monitor_id: i64,
    result: &CheckResult,
) -> AppResult<()> {
    let history = CreateHistory::new(
        monitor_id,
        match result.status {
            MonitorStatus::Up => HistoryStatus::Up,
            MonitorStatus::Down => HistoryStatus::Down,
            _ => HistoryStatus::Down,
        },
    )
    .with_response_time(result.response_time_ms.unwrap_or(0))
    .with_error(result.error_message.clone().unwrap_or_default());

    sqlx::query(
        r#"
        INSERT INTO history (monitor_id, status, response_time_ms, error_message, checked_at)
        VALUES (?, ?, ?, ?, datetime('now'))
        "#,
    )
    .bind(history.monitor_id)
    .bind(history.status.to_string())
    .bind(history.response_time_ms)
    .bind(history.error_message)
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        UPDATE monitors 
        SET status = ?, last_check_at = datetime('now'), last_response_time_ms = ?, updated_at = datetime('now')
        WHERE id = ?
        "#,
    )
    .bind(result.status.to_string())
    .bind(result.response_time_ms)
    .bind(monitor_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn cleanup_old_history(
    pool: &DbPool,
    monitor_id: i64,
    keep_count: usize,
) -> AppResult<()> {
    sqlx::query(
        r#"
        DELETE FROM history 
        WHERE monitor_id = ? 
        AND id NOT IN (
            SELECT id FROM history 
            WHERE monitor_id = ? 
            ORDER BY checked_at DESC 
            LIMIT ?
        )
        "#,
    )
    .bind(monitor_id)
    .bind(monitor_id)
    .bind(keep_count as i64)
    .execute(pool)
    .await?;

    Ok(())
}
