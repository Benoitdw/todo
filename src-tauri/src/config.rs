use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Config {
    pub server_url: String,
    pub token: String,
}

fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("todo")
        .join("config.toml")
}

pub fn load() -> Option<Config> {
    let path = config_path();
    let content = std::fs::read_to_string(&path).ok()?;
    let config: Config = toml::from_str(&content).ok()?;
    if config.server_url.is_empty() || config.token.is_empty() {
        None
    } else {
        Some(config)
    }
}

pub fn save(config: &Config) -> Result<(), String> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let content = toml::to_string(config).map_err(|e| e.to_string())?;
    std::fs::write(&path, content).map_err(|e| e.to_string())
}
