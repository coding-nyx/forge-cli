use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use anyhow::Result;
use directories::ProjectDirs;
use tracing::{info, warn};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    pub default_provider: Option<String>,
    pub default_model: Option<String>,
    pub providers: ProvidersConfig,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct ProvidersConfig {
    pub openrouter: Option<ApiConfig>,
    pub google: Option<ApiConfig>,
    pub grok: Option<ApiConfig>,
    pub local: Option<LocalConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiConfig {
    pub api_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LocalConfig {
    pub base_url: String,
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = fs::read_to_string(path)?;
        let config = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        fs::write(path, content)?;
        info!("Configuration saved to disk.");
        Ok(())
    }

    pub fn config_path() -> Result<PathBuf> {
        let proj_dirs = ProjectDirs::from("com", "forge", "forge-cli")
            .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;
        Ok(proj_dirs.config_dir().join("config.toml"))
    }
}
