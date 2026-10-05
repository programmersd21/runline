use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub repos: Vec<String>,
    pub polling: PollingConfig,
    pub ui: UiConfig,
    pub cache: CacheConfig,
    pub keys: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PollingConfig {
    pub active_interval: String,
    pub idle_max_backoff: String,
    pub run_limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UiConfig {
    pub theme: String,
    pub reduce_motion: bool,
    pub ascii: bool,
    pub timestamps: bool,
    pub wrap: bool,
    pub graphics: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CacheConfig {
    pub enabled: bool,
    pub max_size_mb: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            repos: Vec::new(),
            polling: PollingConfig::default(),
            ui: UiConfig::default(),
            cache: CacheConfig::default(),
            keys: std::collections::HashMap::new(),
        }
    }
}

impl Default for PollingConfig {
    fn default() -> Self {
        Self {
            active_interval: "3s".to_string(),
            idle_max_backoff: "60s".to_string(),
            run_limit: 30,
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            theme: "terminal".to_string(),
            reduce_motion: false,
            ascii: false,
            timestamps: false,
            wrap: false,
            graphics: "auto".to_string(),
        }
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_size_mb: 200,
        }
    }
}

pub fn default_config_path() -> PathBuf {
    let base = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join(".config")
        });
    base.join("runline").join("config.toml")
}

pub fn save_config(config: &Config, path: Option<&PathBuf>) -> Result<(), String> {
    let config_path = match path {
        Some(p) => p.clone(),
        None => default_config_path(),
    };

    if let Some(parent) = config_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let serialized =
        toml::to_string_pretty(config).map_err(|e| format!("cannot serialize config: {}", e))?;

    std::fs::write(&config_path, serialized)
        .map_err(|e| format!("cannot write config to {:?}: {}", config_path, e))?;

    Ok(())
}

pub fn load_config(path: Option<&PathBuf>) -> Result<Config, String> {
    let config_path = match path {
        Some(p) => p.clone(),
        None => default_config_path(),
    };

    if !config_path.exists() {
        return Ok(Config::default());
    }

    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("cannot read config at {:?}: {}", config_path, e))?;

    let cfg: Config = toml::from_str(&content)
        .map_err(|e| format!("invalid config at {:?}: {}", config_path, e))?;

    Ok(cfg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = Config::default();
        assert_eq!(cfg.polling.run_limit, 30);
        assert_eq!(cfg.ui.theme, "terminal");
    }
}

impl Config {
    pub fn validate(&self) -> Vec<String> {
        vec![]
    }
}
pub fn load(path: Option<&str>, ignore: bool) -> Config {
    use std::path::PathBuf;
    let pb = path.map(PathBuf::from);
    match load_config(pb.as_ref()) {
        Ok(c) => c,
        Err(e) => {
            if ignore {
                Config::default()
            } else {
                eprintln!("{e}");
                Config::default()
            }
        }
    }
}
