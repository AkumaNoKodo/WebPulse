use serde::Deserialize;
use std::path::Path;

use anyhow::Context;

#[derive(Debug, Default, Deserialize, Clone)]
#[serde(default)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub scheduler: SchedulerConfig,
    pub monitor: MonitorConfig,
    pub logging: LoggingConfig,
}

impl Config {
    pub fn from_file(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read config file {}", path.display()))?;

        toml::from_str(&content)
            .with_context(|| format!("failed to parse config file {}", path.display()))
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 3000,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct DatabaseConfig {
    pub path: String,
    pub max_connections: u32,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            path: "webpulse.db".to_string(),
            max_connections: 10,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct SchedulerConfig {
    pub max_concurrent_checks: usize,
    pub check_batch_interval_ms: u64,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            max_concurrent_checks: 100,
            check_batch_interval_ms: 1000,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct MonitorConfig {
    pub default_timeout_secs: i64,
    pub default_interval_secs: i64,
    pub history_retention_count: i64,
}

impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            default_timeout_secs: 30,
            default_interval_secs: 60,
            history_retention_count: 100,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct LoggingConfig {
    pub level: String,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
        }
    }
}
