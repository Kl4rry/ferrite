use std::{
    collections::HashMap,
    error::Error,
    fmt::{self},
    fs,
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::Result;
use memchr::memrchr;
use serde::Deserialize;
use tui::style::{self, Color, ParseColorError};

#[derive(Debug)]
pub enum StyleLoadError {
    InvalidColor(ParseColorError),
    StyleNotFound(String),
}

impl fmt::Display for StyleLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StyleLoadError::InvalidColor(err) => err.fmt(f),
            StyleLoadError::StyleNotFound(s) => write!(f, "style not found: {s}"),
        }
    }
}

impl From<ParseColorError> for StyleLoadError {
    fn from(value: ParseColorError) -> Self {
        Self::InvalidColor(value)
    }
}

impl Error for StyleLoadError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidColor(e) => Some(e),
            _ => None,
        }
    }
}

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
    syntax: HashMap<String, Style>,
}

impl Theme {
    pub fn get_style(&self, name: &str) -> Result<style::Style> {
        match self.items.get(name) {
            Some(s) => raw_style_to_style(s, &self.palette),
            None => Err(StyleLoadError::StyleNotFound(name.to_string()))?,
        }
    }
}

fn raw_style_to_style(s: &Style, palette: &HashMap<String, String>) -> Result<style::Style> {
    let mut style = style::Style::default();

    if let Some(fg) = &s.fg {
        if let Some(color) = palette.get(fg) {
            style.fg = Some(Color::from_str(color)?);
        }
    }

    if let Some(bg) = &s.bg {
        if let Some(color) = palette.get(bg) {
            style.bg = Some(Color::from_str(color)?);
        }
    }

    Ok(style)
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
    pub fuzzy_match: style::Style,
    pub completer: style::Style,
    pub completer_selected: style::Style,
    // syntax styles
    syntax: HashMap<String, style::Style>,
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
            search_match: theme.get_style("editor.search.match")?,
            error_text: theme.get_style("editor.error_text")?,
            ruler: theme.get_style("editor.ruler")?,
            fuzzy_match: theme.get_style("editor.fuzzy.match")?,
            completer: theme.get_style("editor.completer")?,
            completer_selected: theme.get_style("editor.completer.selected")?,

            syntax: {
                let mut syntax = HashMap::new();
                for (key, style) in theme.syntax.into_iter() {
                    syntax.insert(key, raw_style_to_style(&style, &theme.palette)?);
                }
                syntax
            },
        })
    }

    pub fn get_syntax(&self, name: &str) -> style::Style {
        let mut name = name;
        loop {
            match self.syntax.get(name) {
                Some(style) => return *style,
                None => match memrchr(b'.', name.as_bytes()) {
                    Some(i) => name = &name[..i],
                    None => break,
                },
            }
        }
        log::warn!("missing in theme: {}", name);
        self.text
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