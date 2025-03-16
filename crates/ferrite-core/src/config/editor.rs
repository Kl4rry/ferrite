use std::{fs, path::PathBuf};

use anyhow::Result;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{
    cmd::Cmd,
    keymap::{InputContext, Key},
};

pub fn default_theme() -> String {
    "default".into()
}

pub fn default_keymap_mode() -> String {
    "normal".into()
}

pub fn default_font() -> String {
    "Fira Code".into()
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
pub struct Editor {
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_rulers")]
    pub rulers: Vec<u16>,
    #[serde(default = "get_false")]
    pub local_clipboard: bool,
    #[serde(default = "get_true")]
    pub show_splash: bool,
    #[serde(default = "get_true")]
    pub watch_open_files: bool,
    #[serde(default = "get_false")]
    pub show_indent_rulers: bool,
    #[serde(default = "get_false")]
    pub always_prompt_on_exit: bool,
    #[serde(default = "get_true")]
    pub case_insensitive_search: bool,
    #[serde(default = "get_true")]
    pub pipe_shell_palette: bool,
    #[serde(default = "get_true")]
    pub auto_trim_whitespace: bool,
    #[serde(default = "get_false")]
    pub auto_format: bool,
    #[serde(default = "get_true")]
    pub highlight_cursor_line: bool,
    #[serde(default)]
    pub line_number: LineNumber,
    #[serde(default)]
    pub render_whitespace: RenderWhitespace,
    #[serde(default)]
    pub picker: PickerConfig,
    #[serde(default)]
    pub info_line: InfoLineConfig,
    #[serde(default)]
    pub gui: Gui,
    #[serde(default)]
    pub keymap: IndexMap<Key, KeymapAndMetadata>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct KeymapAndMetadata {
    #[serde(flatten)]
    pub cmd: Cmd,
    #[serde(default)]
    pub ignore_modifiers: bool,
    #[serde(default = "default_keymap_mode")]
    pub mode: String,
    #[serde(default)]
    pub ctx: InputContext,
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

#[derive(Serialize, Deserialize, Default, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FontWeight {
    Thin = 100,
    ExtraLight = 200,
    Light = 300,
    #[default]
    Normal = 400,
    Medium = 500,
    Semibold = 600,
    Bold = 700,
    ExtraBold = 800,
    Black = 900,
}

#[derive(Serialize, Deserialize, Default, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CursorType {
    Block,
    #[default]
    Line,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Gui {
    #[serde(default = "default_font")]
    pub font_family: String,
    #[serde(default)]
    pub font_weight: FontWeight,
    #[serde(default)]
    pub cursor_type: CursorType,
}

impl Default for Gui {
    fn default() -> Self {
        Self {
            font_family: default_font(),
            font_weight: FontWeight::default(),
            cursor_type: CursorType::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct PickerConfig {
    #[serde(default = "get_true")]
    pub show_hidden: bool,
    #[serde(default = "get_true")]
    pub follow_gitignore: bool,
    #[serde(default = "get_true")]
    pub follow_git_exclude: bool,
    #[serde(default = "get_true")]
    pub follow_ignore: bool,
    #[serde(default = "get_true")]
    pub follow_git_global: bool,
    #[serde(default = "get_true")]
    pub show_only_text_files: bool,
    #[serde(default = "get_true")]
    pub file_picker_auto_reload: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InfoLineConfig {
    pub left: Vec<String>,
    pub center: Vec<String>,
    pub right: Vec<String>,
    pub padding: usize,
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
            file_picker_auto_reload: true,
        }
    }
}

impl Editor {
    pub const DEFAULT: &str = include_str!("../../../../config/editor.toml");

    pub fn create_default_config(overwrite: bool) -> Result<()> {
        let config = Self::get_default_location()?;

        let mut config_folder = config.clone();
        config_folder.pop();

        if !config_folder.exists() {
            fs::create_dir_all(config_folder)?;
        }

        if !config.exists() || overwrite {
            fs::write(config, Self::DEFAULT)?;
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
        Ok(directories.config_dir().join("editor.toml"))
    }
}

impl Default for Editor {
    fn default() -> Self {
        toml::from_str(Self::DEFAULT).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_config() {
        let _ = Editor::default();
    }
}
