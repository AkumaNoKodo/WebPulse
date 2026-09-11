use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Semaphore;
use tokio::time;

use crate::config::SchedulerConfig;
use crate::models::Monitor;
use crate::services::checker::probe;
use crate::store::{self, AppState};

pub async fn run(state: AppState, config: SchedulerConfig) {
    tracing::info!("Scheduler started");

    let permits = Arc::new(Semaphore::new(config.max_concurrent_checks));
    let mut ticker = time::interval(Duration::from_millis(config.check_batch_interval_ms));

    loop {
        ticker.tick().await;

        match store::claim_due_monitors(&state).await {
            Ok(monitors) => {
                for monitor in monitors {
                    tokio::spawn(check(state.clone(), permits.clone(), monitor));
                }
            }
            Err(error) => tracing::error!("Failed to claim due monitors: {error}"),
        }
    }
}

async fn check(state: AppState, permits: Arc<Semaphore>, monitor: Monitor) {
    let Ok(_permit) = permits.acquire().await else {
        return;
    };

    let timeout = Duration::from_secs(monitor.timeout_secs.max(1) as u64);
    let result = probe(&state.http, &monitor.url, timeout).await;

    if let Err(error) = store::record_check(&state, monitor.id, &result).await {
        tracing::error!("Failed to record check for monitor {}: {error}", monitor.id);
    }
}
