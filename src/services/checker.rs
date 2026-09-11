use reqwest::Client;
use std::time::{Duration, Instant};

use crate::models::HistoryStatus;

#[derive(Debug, Clone)]
pub struct CheckResult {
    pub status: HistoryStatus,
    pub response_time_ms: Option<i64>,
    pub error_message: Option<String>,
}

pub async fn probe(client: &Client, url: &str, timeout: Duration) -> CheckResult {
    let start = Instant::now();
    let outcome = client.get(url).timeout(timeout).send().await;
    let response_time_ms = Some(start.elapsed().as_millis() as i64);

    match outcome {
        Ok(response) if response.status().is_success() => CheckResult {
            status: HistoryStatus::Up,
            response_time_ms,
            error_message: None,
        },
        Ok(response) => CheckResult {
            status: HistoryStatus::Down,
            response_time_ms,
            error_message: Some(format!("HTTP {}", response.status())),
        },
        Err(error) => CheckResult {
            status: HistoryStatus::Down,
            response_time_ms,
            error_message: Some(error.to_string()),
        },
    }
}
