use std::sync::Arc;
use std::time::Duration;

use tokio::sync::RwLock;
use tokio::time;

use crate::config::SchedulerConfig;
use crate::db::DbPool;
use crate::error::AppResult;
use crate::models::{Heartbeat, Monitor};
use crate::services::checker::Checkable;
use crate::services::heartbeat_checker::check_heartbeat_status;

pub struct Scheduler {
    pool: DbPool,
    config: SchedulerConfig,
    shutdown_signal: Arc<RwLock<bool>>,
}

impl Scheduler {
    pub fn new(pool: DbPool, config: SchedulerConfig) -> Self {
        Self {
            pool,
            config,
            shutdown_signal: Arc::new(RwLock::new(false)),
        }
    }

    pub async fn start(&self) {
        tracing::info!("Scheduler started");

        let mut interval =
            time::interval(Duration::from_millis(self.config.check_batch_interval_ms));

        loop {
            if *self.shutdown_signal.read().await {
                tracing::info!("Scheduler shutting down");
                break;
            }

            interval.tick().await;

            if let Err(e) = self.check_due_monitors().await {
                tracing::error!("Error checking monitors: {}", e);
            }

            if let Err(e) = self.check_heartbeats().await {
                tracing::error!("Error checking heartbeats: {}", e);
            }
        }
    }

    pub async fn check_due_monitors(&self) -> AppResult<()> {
        let monitors = sqlx::query_as::<_, Monitor>(
            r#"
            SELECT * FROM monitors 
            WHERE last_check_at IS NULL 
            OR datetime(last_check_at, '+' || check_interval_secs || ' seconds') <= datetime('now')
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let pool = self.pool.clone();

        for monitor in monitors {
            let checker = crate::services::http_checker::HttpChecker::new(
                monitor.id,
                monitor.name.clone(),
                monitor.url.clone(),
                monitor.timeout_secs as u64,
            );

            let pool_clone = pool.clone();
            let monitor_clone = monitor.clone();

            tokio::spawn(async move {
                let result = checker.check(&pool_clone).await;

                match result {
                    Ok(check_result) => {
                        if let Err(e) = crate::services::checker::record_check_result(
                            &pool_clone,
                            monitor_clone.id,
                            &check_result,
                        )
                        .await
                        {
                            tracing::error!("Error recording check result: {}", e);
                        }
                    }
                    Err(e) => {
                        tracing::error!("Error checking monitor {}: {}", monitor_clone.id, e);
                    }
                }
            });
        }

        Ok(())
    }

    pub async fn check_heartbeats(&self) -> AppResult<()> {
        let heartbeats = sqlx::query_as::<_, Heartbeat>("SELECT * FROM heartbeats")
            .fetch_all(&self.pool)
            .await?;

        for heartbeat in heartbeats {
            if let Err(e) = check_heartbeat_status(
                &self.pool,
                heartbeat.id,
                heartbeat.expected_interval_secs,
                heartbeat.grace_period_secs,
                heartbeat.last_ping_at,
            )
            .await
            {
                tracing::error!("Error checking heartbeat {}: {}", heartbeat.id, e);
            }
        }

        Ok(())
    }

    pub async fn shutdown(&self) {
        *self.shutdown_signal.write().await = true;
    }
}
