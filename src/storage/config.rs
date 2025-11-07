use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    ReadError(#[from] std::io::Error),
    #[error("Failed to parse config: {0}")]
    ParseError(#[from] toml::de::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    pub google: GoogleConfig,
    pub sync: SyncConfig,
    pub ui: UiConfig,
    pub calendars: CalendarsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GoogleConfig {
    pub client_id: String,
    pub client_secret: String,
    pub token_cache: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SyncConfig {
    pub auto_sync_interval_minutes: u32,
    pub offline_mode: bool,
    pub sync_past_days: u32,
    pub sync_future_days: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UiConfig {
    pub first_day_of_week: String,
    pub time_format: String,
    pub date_format: String,
    pub show_week_numbers: bool,
    pub default_view: String,
    pub theme: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CalendarsConfig {
    pub default: String,
    pub visible: Vec<String>,
}

impl Config {
    pub fn from_toml(content: &str) -> Result<Self, ConfigError> {
        toml::from_str(content).map_err(ConfigError::from)
    }

    pub fn load_or_create() -> Result<Self, ConfigError> {
        let config_path = Self::config_path();

        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            Self::from_toml(&content)
        } else {
            let config = Self::default();
            config.save()?;
            Ok(config)
        }
    }

    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("gcal-imp")
            .join("config.toml")
    }

    pub fn save(&self) -> Result<(), ConfigError> {
        let config_path = Self::config_path();

        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(self)
            .expect("Failed to serialize config");
        std::fs::write(&config_path, content)?;

        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("gcal-imp");

        Self {
            google: GoogleConfig {
                client_id: String::new(),
                client_secret: String::new(),
                token_cache: config_dir.join("token.json"),
            },
            sync: SyncConfig {
                auto_sync_interval_minutes: 15,
                offline_mode: false,
                sync_past_days: 90,
                sync_future_days: 365,
            },
            ui: UiConfig {
                first_day_of_week: "Monday".to_string(),
                time_format: "24h".to_string(),
                date_format: "%Y-%m-%d".to_string(),
                show_week_numbers: true,
                default_view: "Month".to_string(),
                theme: "default".to_string(),
            },
            calendars: CalendarsConfig {
                default: "primary".to_string(),
                visible: vec!["primary".to_string()],
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_15_minute_sync_interval() {
        let config = Config::default();
        assert_eq!(config.sync.auto_sync_interval_minutes, 15);
    }

    #[test]
    fn default_config_syncs_90_days_past() {
        let config = Config::default();
        assert_eq!(config.sync.sync_past_days, 90);
    }

    #[test]
    fn default_config_syncs_365_days_future() {
        let config = Config::default();
        assert_eq!(config.sync.sync_future_days, 365);
    }

    #[test]
    fn parse_valid_toml_config() {
        let toml_content = r#"
            [google]
            client_id = "test_client_id"
            client_secret = "test_secret"
            token_cache = "/tmp/token.json"

            [sync]
            auto_sync_interval_minutes = 30
            offline_mode = false
            sync_past_days = 60
            sync_future_days = 180

            [ui]
            first_day_of_week = "Sunday"
            time_format = "12h"
            date_format = "%d/%m/%Y"
            show_week_numbers = false
            default_view = "Week"
            theme = "default"

            [calendars]
            default = "primary"
            visible = ["primary", "work"]
        "#;

        let config = Config::from_toml(toml_content).unwrap();

        assert_eq!(config.google.client_id, "test_client_id");
        assert_eq!(config.sync.auto_sync_interval_minutes, 30);
        assert_eq!(config.ui.first_day_of_week, "Sunday");
        assert_eq!(config.calendars.visible, vec!["primary", "work"]);
    }

    #[test]
    fn parse_invalid_toml_returns_error() {
        let invalid_toml = "this is not valid toml";
        let result = Config::from_toml(invalid_toml);
        assert!(result.is_err());
    }
}
