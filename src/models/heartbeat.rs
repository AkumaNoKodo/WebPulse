use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Row};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Heartbeat {
    pub id: i64,
    pub uuid: String,
    pub name: String,
    pub expected_interval_secs: i64,
    pub grace_period_secs: i64,
    pub status: HeartbeatStatus,
    pub last_ping_at: Option<DateTime<Utc>>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum HeartbeatStatus {
    Healthy,
    Late,
    Down,
    Unknown,
}

impl Default for HeartbeatStatus {
    fn default() -> Self {
        Self::Unknown
    }
}

impl std::fmt::Display for HeartbeatStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HeartbeatStatus::Healthy => write!(f, "healthy"),
            HeartbeatStatus::Late => write!(f, "late"),
            HeartbeatStatus::Down => write!(f, "down"),
            HeartbeatStatus::Unknown => write!(f, "unknown"),
        }
    }
}

impl From<&str> for HeartbeatStatus {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "healthy" => HeartbeatStatus::Healthy,
            "late" => HeartbeatStatus::Late,
            "down" => HeartbeatStatus::Down,
            _ => HeartbeatStatus::Unknown,
        }
    }
}

impl FromRow<'_, sqlx::sqlite::SqliteRow> for Heartbeat {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> sqlx::Result<Self> {
        Ok(Heartbeat {
            id: row.get("id"),
            uuid: row.get("uuid"),
            name: row.get("name"),
            expected_interval_secs: row.get("expected_interval_secs"),
            grace_period_secs: row.get("grace_period_secs"),
            status: HeartbeatStatus::from(row.get::<&str, _>("status")),
            last_ping_at: row.get("last_ping_at"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateHeartbeat {
    pub name: String,
    pub expected_interval_secs: Option<i64>,
    pub grace_period_secs: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatUpdate {
    pub name: Option<String>,
    pub expected_interval_secs: Option<i64>,
    pub grace_period_secs: Option<i64>,
}

impl Heartbeat {
    pub fn generate_uuid() -> String {
        Uuid::new_v4().to_string()
    }
}
