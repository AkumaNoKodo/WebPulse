use serde::Deserialize;
use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub database: DatabaseConfig,
    #[serde(default)]
    pub scheduler: SchedulerConfig,
    #[serde(default)]
    pub monitor: MonitorConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            database: DatabaseConfig::default(),
            scheduler: SchedulerConfig::default(),
            monitor: MonitorConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

impl Config {
    pub fn from_file(path: impl Into<PathBuf>) -> anyhow::Result<Self> {
        let path = path.into();
        let content = std::fs::read_to_string(&path)?;
        toml::from_str(&content).map_err(|e| anyhow::anyhow!("Failed to parse config: {}", e))
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

impl ServerConfig {
    pub fn address(&self) -> SocketAddr {
        format!("{}:{}", self.host, self.port)
            .parse()
            .unwrap_or_else(|_| SocketAddr::from(([0, 0, 0, 0], self.port)))
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
            path: "rustpulse.db".to_string(),
            max_connections: 10,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct SchedulerConfig {
    pub max_concurrent_checks: usize,
    pub check_batch_interval_ms: u64,
    pub heartbeat_check_interval_secs: u64,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            max_concurrent_checks: 100,
            check_batch_interval_ms: 1000,
            heartbeat_check_interval_secs: 30,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct MonitorConfig {
    pub default_timeout_secs: u64,
    pub default_interval_secs: u64,
    pub history_retention_count: usize,
}

impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            default_timeout_secs: 30,
            default_interval_secs: 60,
            history_retention_count: 20,
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
