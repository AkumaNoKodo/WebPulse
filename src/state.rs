use std::sync::Arc;
use tokio::sync::broadcast;

use crate::config::Config;
use crate::db::DbPool;
use crate::services::scheduler::{EventSender, MonitorEvent, Scheduler};

pub struct AppState {
    pub pool: DbPool,
    pub config: Config,
    pub event_sender: EventSender,
    pub scheduler: Arc<Scheduler>,
}

impl AppState {
    pub fn new(pool: DbPool, config: Config) -> Self {
        let (event_sender, _) = broadcast::channel::<MonitorEvent>(100);

        let scheduler = Arc::new(Scheduler::new(
            pool.clone(),
            config.scheduler.clone(),
            event_sender.clone(),
        ));

        Self {
            pool,
            config,
            event_sender,
            scheduler,
        }
    }
}
