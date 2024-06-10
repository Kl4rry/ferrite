use std::{
    collections::HashMap,
    error::Error,
    fmt::{self},
    fs,
    path::{Path, PathBuf},
};

use anyhow::Result;
use csscolorparser::ParseColorError;
use memchr::memrchr;
use serde::Deserialize;

pub mod style;

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
        match palette.get(fg) {
            Some(color) => style.fg = Some(csscolorparser::parse(color)?),
            None => tracing::error!("Color `{fg}` not found"),
        }
    }

    if let Some(bg) = &s.bg {
        match palette.get(bg) {
            Some(color) => style.bg = Some(csscolorparser::parse(color)?),
            None => tracing::error!("Color `{bg}` not found"),
        }
    }

    Ok(style)
}

pub struct EditorTheme {
    pub line_nr: style::Style,
    pub current_line_nr: style::Style,
    pub text: style::Style,
    pub dim_text: style::Style,
    pub info_line: style::Style,
    pub info_line_unfocused: style::Style,
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
    pub fn parse_theme(s: &str) -> Result<Self> {
        let theme: Theme = toml::from_str(s)?;

        Ok(Self {
            line_nr: theme.get_style("editor.line_nr")?,
            current_line_nr: theme.get_style("editor.current_line_nr")?,
            text: theme.get_style("editor.text")?,
            dim_text: theme.get_style("editor.dim_text")?,
            info_line: theme.get_style("editor.info_line")?,
            info_line_unfocused: theme.get_style("editor.info_line.unfocused")?,
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
                Some(style) => return style.clone(),
                None => match memrchr(b'.', name.as_bytes()) {
                    Some(i) => name = &name[..i],
                    None => break,
                },
            }
        }
        tracing::warn!("missing in theme: {}", name);
        self.text.clone()
    }

    pub fn load_theme(path: impl AsRef<Path>) -> Result<Self> {
        Self::parse_theme(&fs::read_to_string(path)?)
    }

    pub fn load_themes() -> HashMap<String, EditorTheme> {
        let mut theme_dirs = vec![PathBuf::from("themes")];
        if let Some(dirs) = directories::ProjectDirs::from("", "", "ferrite") {
            theme_dirs.push(dirs.config_dir().join("themes"));
        }

        tracing::info!("Loading themes from: {:#?}", theme_dirs);

        let mut themes = HashMap::new();
        for path in theme_dirs {
            let dir = match fs::read_dir(&path) {
                Ok(dir) => dir,
                Err(err) => {
                    tracing::error!("Error loading {} {err}", path.to_string_lossy());
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
                                tracing::error!("Error loading {} {err}", path.to_string_lossy())
                            }
                        }
                    }
                }
            }
        }

        #[cfg(feature = "embed-themes")]
        {
            for (name, theme) in get_embedded_themes() {
                themes.entry(name).or_insert(theme);
            }
        }

        themes.insert("default".into(), EditorTheme::default());

        tracing::info!("{:#?}", themes.keys().collect::<Vec<&String>>());

        themes
    }
}

impl Default for EditorTheme {
    fn default() -> Self {
        EditorTheme::parse_theme(include_str!("../../../themes/catppuccin_mocha.toml")).unwrap()
    }
}

#[cfg(feature = "embed-themes")]
static THEMES: include_dir::Dir<'_> = include_dir::include_dir!("$CARGO_MANIFEST_DIR/../../themes");

#[cfg(feature = "embed-themes")]
fn get_embedded_themes() -> Vec<(String, EditorTheme)> {
    THEMES
        .files()
        .map(|file| {
            (
                file.path()
                    .file_stem()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned(),
                EditorTheme::parse_theme(file.contents_utf8().unwrap()).unwrap(),
            )
        })
        .collect()
}

pub fn init_themes() -> Result<()> {
    #[cfg(feature = "embed-themes")]
    {
        let Some(project_dirs) = directories::ProjectDirs::from("", "", "ferrite") else {
            anyhow::bail!("Config directory could not be located");
        };
        let theme_dir = project_dirs.config_dir().join("themes");
        fs::create_dir_all(&theme_dir)?;
        for (name, theme) in THEMES.files().map(|file| {
            (
                file.path()
                    .file_stem()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned(),
                file.contents_utf8().unwrap(),
            )
        }) {
            let path = theme_dir.join(format!("{name}.toml"));
            if !path.exists() {
                fs::write(&path, theme)?;
            }
        }

        println!("Wrote bundled themes to `{}`", theme_dir.to_string_lossy());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn themes_config() {
        let _ = EditorTheme::default();
    }

    #[cfg(feature = "embed-themes")]
    #[test]
    fn parse_embedded_themes() {
        for file in THEMES.files() {
            let content = file.contents_utf8();
            assert!(content.is_some());
            assert!(EditorTheme::parse_theme(content.unwrap()).is_ok());
        }
    }
}
