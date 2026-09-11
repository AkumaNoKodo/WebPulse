use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct History {
    pub id: i64,
    pub monitor_id: i64,
    pub status: HistoryStatus,
    pub response_time_ms: Option<i64>,
    pub error_message: Option<String>,
    pub checked_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(rename_all = "lowercase")]
pub enum HistoryStatus {
    Up,
    Down,
}

impl HistoryStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            HistoryStatus::Up => "up",
            HistoryStatus::Down => "down",
        }
    }
}

impl From<HistoryStatus> for super::MonitorStatus {
    fn from(status: HistoryStatus) -> Self {
        match status {
            HistoryStatus::Up => super::MonitorStatus::Up,
            HistoryStatus::Down => super::MonitorStatus::Down,
        }
    }
}
