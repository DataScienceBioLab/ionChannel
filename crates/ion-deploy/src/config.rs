//! Configuration management

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::discovery::VmInfo;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub last_vm: Option<VmInfo>,
    pub discovered_vms: Vec<VmInfo>,
    pub preferences: Preferences,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preferences {
    pub auto_restart: bool,
    pub monitor_logs: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            last_vm: None,
            discovered_vms: Vec::new(),
            preferences: Preferences {
                auto_restart: false,
                monitor_logs: false,
            },
        }
    }
}

impl Config {
    /// Load config from file or create default
    pub fn load(path: Option<&str>) -> Result<Self> {
        let config_path = if let Some(p) = path {
            PathBuf::from(p)
        } else {
            Self::default_path()?
        };

        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            let config: Config = ::toml::from_str(&content)?;
            Ok(config)
        } else {
            Ok(Self::default())
        }
    }

    /// Save config to file
    pub fn save(&self) -> Result<()> {
        let config_path = Self::default_path()?;

        // Create parent directory if needed
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = ::toml::to_string_pretty(self)?;
        std::fs::write(&config_path, content)?;

        Ok(())
    }

    /// Reset config to defaults
    pub fn reset(&mut self) -> Result<()> {
        *self = Self::default();
        let config_path = Self::default_path()?;
        if config_path.exists() {
            std::fs::remove_file(&config_path)?;
        }
        Ok(())
    }

    /// Get default config path
    fn default_path() -> Result<PathBuf> {
        let home = home::home_dir().ok_or_else(|| anyhow::anyhow!("Cannot find home directory"))?;
        Ok(home.join(".config/ionChannel/deploy.toml"))
    }
}
