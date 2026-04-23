use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Row};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct History {
    pub id: i64,
    pub monitor_id: i64,
    pub status: HistoryStatus,
    pub response_time_ms: Option<i64>,
    pub error_message: Option<String>,
    pub checked_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HistoryStatus {
    Up,
    Down,
}

impl std::fmt::Display for HistoryStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HistoryStatus::Up => write!(f, "up"),
            HistoryStatus::Down => write!(f, "down"),
        }
    }
}

impl From<&str> for HistoryStatus {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "up" => HistoryStatus::Up,
            _ => HistoryStatus::Down,
        }
    }
}

impl FromRow<'_, sqlx::sqlite::SqliteRow> for History {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> sqlx::Result<Self> {
        Ok(History {
            id: row.get("id"),
            monitor_id: row.get("monitor_id"),
            status: HistoryStatus::from(row.get::<&str, _>("status")),
            response_time_ms: row.get("response_time_ms"),
            error_message: row.get("error_message"),
            checked_at: row.get("checked_at"),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateHistory {
    pub monitor_id: i64,
    pub status: HistoryStatus,
    pub response_time_ms: Option<i64>,
    pub error_message: Option<String>,
}

impl CreateHistory {
    pub fn new(monitor_id: i64, status: HistoryStatus) -> Self {
        Self {
            monitor_id,
            status,
            response_time_ms: None,
            error_message: None,
        }
    }

    pub fn with_response_time(mut self, ms: i64) -> Self {
        self.response_time_ms = Some(ms);
        self
    }

    pub fn with_error(mut self, error: String) -> Self {
        self.error_message = Some(error);
        self
    }
}
