use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Config {
    #[serde(default = "default_archive_after_days")]
    pub archive_after_days: i64,
}

fn default_archive_after_days() -> i64 {
    1
}

impl Default for Config {
    fn default() -> Self {
        Self {
            archive_after_days: 1,
        }
    }
}

pub fn get_config_path() -> PathBuf {
    if let Ok(val) = std::env::var("YK_CONFIG_PATH") {
        return PathBuf::from(val);
    }

    if let Some(mut dir) = dirs::config_dir() {
        dir.push("yk");
        dir.push("config.toml");
        return dir;
    }

    PathBuf::from("config.toml")
}

impl Config {
    pub fn load() -> Self {
        let path = get_config_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(config) = toml::from_str::<Config>(&content) {
                    return config;
                }
            }
        }

        let default_config = Config::default();
        let _ = Self::save_default(&path, &default_config);
        default_config
    }

    fn save_default(path: &PathBuf, config: &Config) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let serialized = toml::to_string_pretty(config).unwrap_or_default();
        fs::write(path, serialized)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.archive_after_days, 1);
    }

    #[test]
    fn test_config_deserialize() {
        let toml_str = "archive_after_days = 3\n";
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.archive_after_days, 3);
    }
}
