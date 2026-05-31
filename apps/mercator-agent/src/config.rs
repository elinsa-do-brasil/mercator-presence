use std::fs;
use std::path::{Path, PathBuf};

#[cfg(not(windows))]
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::security::{SecurityError, mask_secret, validate_server_url};

pub const DEFAULT_HEARTBEAT_INTERVAL_SECONDS: u64 = 900;
pub const DEFAULT_AGENT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AgentConfig {
    pub server_url: String,
    pub device_id: Option<String>,
    pub device_token: Option<String>,
    #[serde(default = "default_heartbeat_interval_seconds")]
    pub heartbeat_interval_seconds: u64,
    #[serde(default = "default_agent_version")]
    pub agent_version: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DisplayConfig {
    pub server_url: String,
    pub device_id: Option<String>,
    pub device_token: Option<String>,
    pub heartbeat_interval_seconds: u64,
    pub agent_version: String,
    pub config_path: String,
    pub log_path: String,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("config file does not exist at {0}")]
    Missing(PathBuf),
    #[error("invalid config: {0}")]
    Invalid(String),
    #[error(transparent)]
    Security(#[from] SecurityError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

impl AgentConfig {
    pub fn new_enrolled(
        server_url: String,
        device_id: String,
        device_token: String,
        heartbeat_interval_seconds: Option<u64>,
    ) -> Result<Self, ConfigError> {
        let normalized_url = normalize_server_url(&server_url)?;
        let config = Self {
            server_url: normalized_url,
            device_id: Some(non_empty(device_id, "deviceId")?),
            device_token: Some(non_empty(device_token, "deviceToken")?),
            heartbeat_interval_seconds: heartbeat_interval_seconds
                .unwrap_or(DEFAULT_HEARTBEAT_INTERVAL_SECONDS),
            agent_version: DEFAULT_AGENT_VERSION.to_string(),
        };
        config.validate_enrolled()?;
        Ok(config)
    }

    pub fn validate_base(&self) -> Result<(), ConfigError> {
        validate_server_url(&self.server_url)?;
        if self.heartbeat_interval_seconds == 0 {
            return Err(ConfigError::Invalid(
                "heartbeatIntervalSeconds must be greater than zero".to_string(),
            ));
        }
        Ok(())
    }

    pub fn validate_enrolled(&self) -> Result<(), ConfigError> {
        self.validate_base()?;
        require_present(&self.device_id, "deviceId")?;
        require_present(&self.device_token, "deviceToken")?;
        Ok(())
    }

    pub fn display(&self) -> DisplayConfig {
        DisplayConfig {
            server_url: self.server_url.clone(),
            device_id: self.device_id.as_ref().map(|id| mask_secret(id)),
            device_token: self.device_token.as_ref().map(|token| mask_secret(token)),
            heartbeat_interval_seconds: self.heartbeat_interval_seconds,
            agent_version: self.agent_version.clone(),
            config_path: config_path().display().to_string(),
            log_path: log_path().display().to_string(),
        }
    }
}

pub fn load_config() -> Result<AgentConfig, ConfigError> {
    load_config_from_path(&config_path())
}

pub fn load_config_from_path(path: &Path) -> Result<AgentConfig, ConfigError> {
    if !path.exists() {
        return Err(ConfigError::Missing(path.to_path_buf()));
    }
    let raw = fs::read_to_string(path)?;
    let mut config = serde_json::from_str::<AgentConfig>(&raw)?;
    config.server_url = normalize_server_url(&config.server_url)?;
    config.validate_base()?;
    Ok(config)
}

pub fn save_config(config: &AgentConfig) -> Result<(), ConfigError> {
    ensure_config_dir()?;
    save_config_to_path(config, &config_path())
}

pub fn save_config_to_path(config: &AgentConfig, path: &Path) -> Result<(), ConfigError> {
    config.validate_enrolled()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let raw = serde_json::to_string_pretty(config)?;
    fs::write(path, raw)?;
    Ok(())
}

pub fn ensure_config_dir() -> Result<PathBuf, ConfigError> {
    let path = base_dir();
    fs::create_dir_all(&path)?;
    Ok(path)
}

pub fn ensure_log_dir() -> Result<PathBuf, ConfigError> {
    let path = log_dir();
    fs::create_dir_all(&path)?;
    Ok(path)
}

pub fn config_path() -> PathBuf {
    base_dir().join("config.json")
}

pub fn log_dir() -> PathBuf {
    base_dir().join("logs")
}

pub fn log_path() -> PathBuf {
    log_dir().join("agent.log")
}

pub fn normalize_server_url(server_url: &str) -> Result<String, ConfigError> {
    let normalized = server_url.trim().trim_end_matches('/').to_string();
    validate_server_url(&normalized)?;
    Ok(normalized)
}

fn default_heartbeat_interval_seconds() -> u64 {
    DEFAULT_HEARTBEAT_INTERVAL_SECONDS
}

fn default_agent_version() -> String {
    DEFAULT_AGENT_VERSION.to_string()
}

fn require_present(value: &Option<String>, field: &str) -> Result<(), ConfigError> {
    let value = value
        .as_ref()
        .ok_or_else(|| ConfigError::Invalid(format!("{field} is required after enrollment")))?;
    non_empty(value.clone(), field).map(|_| ())
}

fn non_empty(value: String, field: &str) -> Result<String, ConfigError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(ConfigError::Invalid(format!("{field} cannot be empty")));
    }
    Ok(trimmed.to_string())
}

fn base_dir() -> PathBuf {
    if let Ok(path) = std::env::var("MERCATOR_AGENT_HOME") {
        return PathBuf::from(path);
    }

    #[cfg(windows)]
    {
        PathBuf::from(r"C:\ProgramData\Mercator\Agent")
    }

    #[cfg(not(windows))]
    {
        ProjectDirs::from("br.com", "Mercator", "MercatorAgent")
            .map(|dirs| dirs.config_dir().to_path_buf())
            .unwrap_or_else(|| std::env::temp_dir().join("mercator-agent"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_server_url_should_trim_trailing_slash() {
        let normalized = normalize_server_url(" https://mercator.example.com/ ").unwrap();
        assert_eq!(normalized, "https://mercator.example.com");
    }

    #[test]
    fn normalize_server_url_should_reject_http() {
        let error = normalize_server_url("http://mercator.example.com").unwrap_err();
        assert!(matches!(error, ConfigError::Security(_)));
    }

    #[test]
    fn config_round_trip_should_preserve_values() {
        let dir =
            std::env::temp_dir().join(format!("mercator-agent-config-test-{}", std::process::id()));
        let path = dir.join("config.json");
        let config = AgentConfig::new_enrolled(
            "https://mercator.example.com".to_string(),
            "dev_123456789".to_string(),
            "tok_123456789".to_string(),
            Some(60),
        )
        .unwrap();

        save_config_to_path(&config, &path).unwrap();
        let loaded = load_config_from_path(&path).unwrap();

        assert_eq!(loaded, config);
        let _ = fs::remove_dir_all(dir);
    }
}
