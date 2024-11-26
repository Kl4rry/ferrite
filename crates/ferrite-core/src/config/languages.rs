use std::{fs, path::PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Languages {
    #[serde(rename = "language")]
    pub languages: Vec<Language>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Language {
    pub name: String,
    pub format: Option<String>,
    pub format_selection: Option<String>,
    pub auto_trim_whitespace: Option<bool>,
    pub auto_format: Option<bool>,
}

impl Languages {
    pub const DEFAULT: &str = include_str!("../../../../config/languages.toml");

    pub fn create_default_config(overwrite: bool) -> Result<()> {
        let config = Self::get_default_location()?;

        let mut config_folder = config.clone();
        config_folder.pop();

        if !config_folder.exists() {
            fs::create_dir_all(config_folder)?;
        }

        if !config.exists() || overwrite {
            fs::write(config, toml::to_string(&Self::default()).unwrap())?;
        }

        Ok(())
    }

    pub fn load_from_default_location() -> Result<Self> {
        let path = Self::get_default_location()?;

        let mut config_folder = path.clone();
        config_folder.pop();

        if !config_folder.exists() {
            fs::create_dir_all(config_folder)?;
        }

        Ok(toml::from_str(&fs::read_to_string(&path)?)?)
    }

    pub fn get_default_location() -> Result<PathBuf> {
        let Some(directories) = directories::ProjectDirs::from("", "", "ferrite") else {
            return Err(anyhow::Error::msg("Unable to find project directory"));
        };
        Ok(directories.config_dir().join("languages.toml"))
    }

    pub fn from_name(&self, name: &str) -> Option<&Language> {
        self.languages.iter().find(|language| language.name == name)
    }
}

impl Default for Languages {
    fn default() -> Self {
        toml::from_str(Self::DEFAULT).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_config() {
        let _ = Languages::default();
    }
}
