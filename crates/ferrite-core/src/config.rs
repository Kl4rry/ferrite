use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::watcher::FromTomlFile;

pub fn default_theme() -> String {
    "default".into()
}

pub fn default_rulers() -> Vec<u16> {
    vec![80]
}

pub fn get_false() -> bool {
    false
}

pub fn get_true() -> bool {
    false
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_rulers")]
    pub rulers: Vec<u16>,
    #[serde(default = "get_false")]
    pub local_clipboard: bool,
    #[serde(default = "get_true")]
    pub show_splash: bool,
    #[serde(default = "get_true")]
    pub watch_recursive: bool,
    #[serde(default = "get_true")]
    pub watch_workspace: bool,
    #[serde(default = "get_true")]
    pub show_indent_rulers: bool,
    #[serde(default = "get_false")]
    pub always_prompt_on_exit: bool,
    #[serde(default = "get_true")]
    pub case_insensitive_search: bool,
    #[serde(default)]
    pub line_number: LineNumber,
    #[serde(default)]
    pub render_whitespace: RenderWhitespace,
    #[serde(default)]
    pub picker: PickerConfig,
    #[serde(default)]
    pub info_line: InfoLineConfig,
    #[serde(default)]
    pub language: Vec<Language>,
}

#[derive(Serialize, Deserialize, Default, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RenderWhitespace {
    All,
    #[default]
    None,
    Trailing,
}

#[derive(Serialize, Deserialize, Default, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LineNumber {
    #[default]
    Absolute,
    None,
    Relative,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct PickerConfig {
    pub show_hidden: bool,
    pub follow_gitignore: bool,
    pub follow_git_exclude: bool,
    pub follow_ignore: bool,
    pub follow_git_global: bool,
    pub show_only_text_files: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InfoLineConfig {
    pub left: Vec<String>,
    pub center: Vec<String>,
    pub right: Vec<String>,
    pub padding: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Language {
    pub name: String,
    pub format: Option<String>,
    pub format_selection: Option<String>,
}

impl Default for InfoLineConfig {
    fn default() -> Self {
        Self {
            left: ["size"].iter().map(|s| s.to_string()).collect(),
            center: ["file"].iter().map(|s| s.to_string()).collect(),
            right: ["branch", "position", "encoding", "language", "spinner"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            padding: 1,
        }
    }
}

impl Default for PickerConfig {
    fn default() -> Self {
        Self {
            show_hidden: false,
            follow_gitignore: true,
            follow_git_exclude: true,
            follow_ignore: true,
            follow_git_global: true,
            show_only_text_files: true,
        }
    }
}

pub const DEFAULT_CONFIG: &str = include_str!("../../../config/default.toml");

impl Config {
    pub fn create_default_config(overwrite: bool) -> Result<()> {
        let config = Self::get_default_location()?;

        let mut config_folder = config.clone();
        config_folder.pop();

        if !config_folder.exists() {
            fs::create_dir_all(config_folder)?;
        }

        if !config.exists() || overwrite {
            fs::write(config, DEFAULT_CONFIG)?;
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

    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        Ok(toml::from_str(&fs::read_to_string(path)?)?)
    }

    pub fn get_default_location() -> Result<PathBuf> {
        let Some(directories) = directories::ProjectDirs::from("", "", "ferrite") else {
            return Err(anyhow::Error::msg("Unable to find project directory"));
        };
        Ok(directories.config_dir().join("config.toml"))
    }
}

impl Default for Config {
    fn default() -> Self {
        toml::from_str(DEFAULT_CONFIG).unwrap()
    }
}

impl FromTomlFile for Config {
    fn from_toml_file(path: impl AsRef<Path>) -> Result<Self>
    where
        Self: Sized,
    {
        Self::load(path)
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
