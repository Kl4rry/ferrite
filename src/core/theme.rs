use std::{
    collections::HashMap,
    error::Error,
    fmt::{self},
    fs,
    path::{Path, PathBuf},
};

use anyhow::Result;
use serde::Deserialize;
use tui::style::{self, Color};

#[derive(Debug)]
pub enum StyleLoadError {
    InvalidColor,
    StyleNotFound(String),
}

impl fmt::Display for StyleLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StyleLoadError::InvalidColor => "invalid color".fmt(f),
            StyleLoadError::StyleNotFound(s) => write!(f, "style not found: {s}"),
        }
    }
}

impl Error for StyleLoadError {}

#[derive(Debug, Deserialize)]
struct Style {
    fg: Option<String>,
    bg: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Theme {
    palette: HashMap<String, String>,
    #[serde(flatten)]
    items: HashMap<String, Style>,
}

pub fn hex_str_to_color(string: &str) -> Result<Color> {
    if string.len() != 7 {
        return Err(StyleLoadError::InvalidColor)?;
    }

    if string.as_bytes()[0] != b'#' {
        return Err(StyleLoadError::InvalidColor)?;
    }

    let r = u8::from_str_radix(&string[1..3], 16)?;
    let g = u8::from_str_radix(&string[3..5], 16)?;
    let b = u8::from_str_radix(&string[5..7], 16)?;

    Ok(Color::Rgb(r, g, b))
}

impl Theme {
    pub fn get_style(&self, name: &str) -> Result<style::Style> {
        match self.items.get(name) {
            Some(s) => {
                let mut style = style::Style::default();

                if let Some(fg) = &s.fg {
                    if let Some(color) = self.palette.get(fg) {
                        style.fg = Some(hex_str_to_color(color)?);
                    }
                }

                if let Some(bg) = &s.bg {
                    if let Some(color) = self.palette.get(bg) {
                        style.bg = Some(hex_str_to_color(color)?);
                    }
                }

                Ok(style)
            }
            None => Err(StyleLoadError::StyleNotFound(name.to_string()))?,
        }
    }
}

pub struct EditorTheme {
    pub line_nr: style::Style,
    pub current_line_nr: style::Style,
    pub text: style::Style,
    pub info_line: style::Style,
    pub background: style::Style,
    pub selection: style::Style,
    pub border: style::Style,
    pub search_match: style::Style,
    pub error_text: style::Style,
    pub ruler: style::Style,
}

impl EditorTheme {
    pub fn from_str(s: &str) -> Result<Self> {
        let theme: Theme = toml::from_str(s)?;

        Ok(Self {
            line_nr: theme.get_style("editor.line_nr")?,
            current_line_nr: theme.get_style("editor.current_line_nr")?,
            text: theme.get_style("editor.text")?,
            info_line: theme.get_style("editor.info_line")?,
            background: theme.get_style("editor.background")?,
            selection: theme.get_style("editor.selection")?,
            border: theme.get_style("editor.border")?,
            search_match: theme.get_style("editor.search_match")?,
            error_text: theme.get_style("editor.error_text")?,
            ruler: theme.get_style("editor.ruler")?,
        })
    }

    pub fn load_theme(path: impl AsRef<Path>) -> Result<Self> {
        Self::from_str(&fs::read_to_string(path)?)
    }

    pub fn load_themes() -> HashMap<String, EditorTheme> {
        let mut theme_dirs = vec![PathBuf::from("themes")];
        if let Some(dirs) = directories::ProjectDirs::from("", "", "ferrite") {
            theme_dirs.push(dirs.config_dir().join("themes"));
        }

        log::info!("Loading themes from: {:#?}", theme_dirs);

        let mut themes = HashMap::new();
        for path in theme_dirs {
            let dir = match fs::read_dir(&path) {
                Ok(dir) => dir,
                Err(err) => {
                    log::error!("Error loading {} {err}", path.to_string_lossy());
                    continue;
                }
            };

            for entry in dir.filter_map(|entry| entry.ok()) {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_file() {
                        let path = entry.path();

                        match EditorTheme::load_theme(entry.path()) {
                            Ok(theme) => {
                                let name = path.file_stem().unwrap().to_string_lossy().into_owned();
                                themes.entry(name).or_insert(theme);
                            }
                            Err(err) => {
                                log::error!("Error loading {} {err}", path.to_string_lossy())
                            }
                        }
                    }
                }
            }
        }

        themes.insert("default".into(), EditorTheme::default());

        log::info!("{:#?}", themes.keys().collect::<Vec<&String>>());

        themes
    }
}

impl Default for EditorTheme {
    fn default() -> Self {
        EditorTheme::from_str(include_str!("../../themes/onedark.toml")).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn themes_config() {
        let _ = EditorTheme::default();
    }
}
