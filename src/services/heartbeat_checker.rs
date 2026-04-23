use chrono::{DateTime, Utc};

use crate::db::DbPool;
use crate::error::AppResult;
use crate::models::HeartbeatStatus;

pub async fn check_heartbeat_status(
    pool: &DbPool,
    heartbeat_id: i64,
    expected_interval_secs: i64,
    grace_period_secs: i64,
    last_ping_at: Option<DateTime<Utc>>,
) -> AppResult<HeartbeatStatus> {
    let now = Utc::now();

    let Some(last_ping) = last_ping_at else {
        return Ok(HeartbeatStatus::Unknown);
    };

    let elapsed = (now - last_ping).num_seconds();
    let deadline = expected_interval_secs + grace_period_secs;

    let status = if elapsed > deadline {
        HeartbeatStatus::Down
    } else if elapsed > expected_interval_secs {
        HeartbeatStatus::Late
    } else {
        HeartbeatStatus::Healthy
    };

    sqlx::query(
        r#"
        UPDATE heartbeats 
        SET status = ?, updated_at = datetime('now')
        WHERE id = ?
        "#,
    )
    .bind(status.to_string())
    .bind(heartbeat_id)
    .execute(pool)
    .await?;

    Ok(status)
}

pub async fn update_heartbeat_ping(pool: &DbPool, heartbeat_id: i64) -> AppResult<()> {
    sqlx::query(
        r#"
        UPDATE heartbeats 
        SET status = 'healthy', last_ping_at = datetime('now'), updated_at = datetime('now')
        WHERE id = ?
        "#,
    )
    .bind(heartbeat_id)
    .execute(pool)
    .await?;

    Ok(())
}
