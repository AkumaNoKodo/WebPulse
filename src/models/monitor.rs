use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Row};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Monitor {
    pub id: i64,
    pub name: String,
    pub url: String,
    pub check_interval_secs: i64,
    pub timeout_secs: i64,
    pub status: MonitorStatus,
    pub last_check_at: Option<DateTime<Utc>>,
    pub last_response_time_ms: Option<i64>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MonitorStatus {
    Up,
    Down,
    Unknown,
}

impl Default for MonitorStatus {
    fn default() -> Self {
        Self::Unknown
    }
}

impl std::fmt::Display for MonitorStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MonitorStatus::Up => write!(f, "up"),
            MonitorStatus::Down => write!(f, "down"),
            MonitorStatus::Unknown => write!(f, "unknown"),
        }
    }
}

impl From<&str> for MonitorStatus {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "up" => MonitorStatus::Up,
            "down" => MonitorStatus::Down,
            _ => MonitorStatus::Unknown,
        }
    }
}

impl FromRow<'_, sqlx::sqlite::SqliteRow> for Monitor {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> sqlx::Result<Self> {
        Ok(Monitor {
            id: row.get("id"),
            name: row.get("name"),
            url: row.get("url"),
            check_interval_secs: row.get("check_interval_secs"),
            timeout_secs: row.get("timeout_secs"),
            status: MonitorStatus::from(row.get::<&str, _>("status")),
            last_check_at: row.get("last_check_at"),
            last_response_time_ms: row.get("last_response_time_ms"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
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
