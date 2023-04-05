use std::{fs, io};

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub theme: String,
}

const DEFAULT_CONFIG: &str = include_str!("../../config/default.toml");

impl Config {
    pub fn load() -> Result<Self> {
        let Some(directories) = directories::ProjectDirs::from("", "", "ferrite") else {
            return Err(anyhow::Error::msg("Unable to find project directory"));
        };

        let path = directories.config_dir().join("config.toml");

        match fs::read_to_string(&path) {
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                if let Err(err) = fs::write(path, DEFAULT_CONFIG) {
                    log::debug!("Error creating config: {}", err);
                }
                Ok(Config::default())
            }
            err => Ok(toml::from_str(&err?)?),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        toml::from_str(DEFAULT_CONFIG).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_config() {
        let _ = Config::default();
    }
}
