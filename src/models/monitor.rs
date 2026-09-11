use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Monitor {
    pub id: i64,
    pub name: String,
    pub url: String,
    pub check_interval_secs: i64,
    pub timeout_secs: i64,
    pub status: MonitorStatus,
    pub last_check_at: Option<DateTime<Utc>>,
    pub last_response_time_ms: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(rename_all = "lowercase")]
pub enum MonitorStatus {
    Up,
    Down,
    #[default]
    Unknown,
}

impl MonitorStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            MonitorStatus::Up => "up",
            MonitorStatus::Down => "down",
            MonitorStatus::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMonitor {
    pub name: String,
    pub url: String,
    pub check_interval_secs: Option<i64>,
    pub timeout_secs: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateMonitor {
    pub name: Option<String>,
    pub url: Option<String>,
    pub check_interval_secs: Option<i64>,
    pub timeout_secs: Option<i64>,
}
