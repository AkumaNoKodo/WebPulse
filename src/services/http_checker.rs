use async_trait::async_trait;
use reqwest::Client;
use std::time::{Duration, Instant};

use crate::db::DbPool;
use crate::error::AppResult;
use crate::models::MonitorStatus;
use crate::services::checker::{CheckResult, Checkable};

pub struct HttpChecker {
    id: i64,
    name: String,
    url: String,
    timeout: Duration,
}

impl HttpChecker {
    pub fn new(id: i64, name: String, url: String, timeout_secs: u64) -> Self {
        Self {
            id,
            name,
            url,
            timeout: Duration::from_secs(timeout_secs),
        }
    }
}

#[async_trait]
impl Checkable for HttpChecker {
    async fn check(&self, _pool: &DbPool) -> AppResult<CheckResult> {
        let client = Client::builder()
            .timeout(self.timeout)
            .connect_timeout(Duration::from_secs(5))
            .build()?;

        let start = Instant::now();

        match client.get(&self.url).send().await {
            Ok(response) => {
                let elapsed = start.elapsed().as_millis() as i64;
                if response.status().is_success() {
                    Ok(CheckResult {
                        status: MonitorStatus::Up,
                        response_time_ms: Some(elapsed),
                        error_message: None,
                    })
                } else {
                    Ok(CheckResult {
                        status: MonitorStatus::Down,
                        response_time_ms: Some(elapsed),
                        error_message: Some(format!("HTTP {}", response.status())),
                    })
                }
            }
            Err(e) => {
                let elapsed = start.elapsed().as_millis() as i64;
                Ok(CheckResult {
                    status: MonitorStatus::Down,
                    response_time_ms: Some(elapsed),
                    error_message: Some(e.to_string()),
                })
            }
        }
    }

    fn id(&self) -> i64 {
        self.id
    }

    fn name(&self) -> &str {
        &self.name
    }
}
