//! Configuration manager

use crate::models::AppConfig;
use directories::ProjectDirs;
use std::fs;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[allow(dead_code)]
    #[error("Config directory not found")]
    NoDirs,
}

#[derive(Clone)]
pub struct ConfigManager {
    path: PathBuf,
}

impl ConfigManager {
    pub fn new() -> Self {
        let path = if let Some(dirs) = ProjectDirs::from("com", "anyrouter", "fast") {
            let config_dir = dirs.config_dir();
            fs::create_dir_all(config_dir).ok();
            config_dir.join("config.json")
        } else {
            PathBuf::from("config.json")
        };

        Self { path }
    }

    /// Create a ConfigManager with a custom path (for testing)
    #[cfg(test)]
    pub fn with_path(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn load(&self) -> Result<AppConfig, ConfigError> {
        if self.path.exists() {
            let content = fs::read_to_string(&self.path)?;
            let config: AppConfig = serde_json::from_str(&content)?;
            Ok(config)
        } else {
            Ok(AppConfig::default())
        }
    }

    pub fn save(&self, config: &AppConfig) -> Result<(), ConfigError> {
        let content = serde_json::to_string_pretty(config)?;
        fs::write(&self.path, content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_config_manager_load_default_when_missing() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");
        let manager = ConfigManager::with_path(config_path);

        let config = manager.load().unwrap();
        assert_eq!(config.check_interval, 30);
        assert_eq!(config.endpoints.len(), 15); // 15个默认站点
    }

    #[test]
    fn test_config_manager_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");
        let manager = ConfigManager::with_path(config_path);

        let config = AppConfig {
            check_interval: 60,
            ..Default::default()
        };

        manager.save(&config).unwrap();
        let loaded = manager.load().unwrap();

        assert_eq!(loaded.check_interval, 60);
    }

    #[test]
    fn test_config_manager_preserves_endpoints() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");
        let manager = ConfigManager::with_path(config_path);

        let mut config = AppConfig::default();
        config.endpoints.push(crate::models::Endpoint {
            name: "Custom".into(),
            url: "https://custom.com/api".into(),
            domain: "custom.com".into(),
            enabled: false,
        });

        manager.save(&config).unwrap();
        let loaded = manager.load().unwrap();

        assert_eq!(loaded.endpoints.len(), 16); // 15个默认 + 1个自定义
        assert_eq!(loaded.endpoints[15].name, "Custom");
        assert!(!loaded.endpoints[15].enabled);
    }

    #[test]
    fn test_config_error_invalid_json() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        fs::write(&config_path, "not valid json").unwrap();

        let manager = ConfigManager::with_path(config_path);
        let result = manager.load();

        assert!(result.is_err());
    }
}
