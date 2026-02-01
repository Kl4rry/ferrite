use encoding_rs::Encoding;
use ferrite_runtime::{Bounds, Painter, View, unique_id::UniqueId};
use ferrite_utility::tui_buf_ext::TuiBufExt;
use tui::style::Style;
use unicode_width::UnicodeWidthStr;

use crate::{byte_size::format_byte_size, config::editor::InfoLineConfig, theme::EditorTheme};

pub struct InfoLineView<'a> {
    pub theme: &'a EditorTheme,
    pub config: &'a InfoLineConfig,
    pub focus: bool,
    pub encoding: &'static Encoding,
    pub path: String,
    pub column: usize,
    pub line: usize,
    pub dirty: bool,
    pub branch: &'a Option<String>,
    pub language: String,
    pub size: usize,
    pub spinner: Option<char>,
    pub read_only: bool,
    pub matches: Option<(usize, usize)>,
    pub parent_unique_id: UniqueId,
}

impl InfoLineView<'_> {
    pub fn get_info_item(&self, item: &str) -> Option<String> {
        match item {
            "file" => {
                let prefix = std::env::current_dir()
                    .map(|d| d.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let mut file = self.path.clone();
                if file.starts_with(&prefix) {
                    file.drain(..prefix.len());
                    while file.starts_with(std::path::MAIN_SEPARATOR) {
                        file.remove(0);
                    }
                }
                if self.dirty {
                    file += " *";
                }
                Some(file)
            }
            "encoding" => Some(self.encoding.name().to_string()),
            "language" => Some(self.language.clone()),
            "position" => Some(format!("{}:{}", self.line, self.column)),
            "branch" => self.branch.clone(),
            "size" => Some(format_byte_size(self.size)),
            "spinner" => Some(self.spinner.unwrap_or(' ').to_string()),
            "read_only" if self.read_only => Some("🔒".into()),
            "matches" => {
                let (index, count) = self.matches?;
                if count > 0 {
                    Some(format!("Match {} of {}", (index + 1), count))
                } else {
                    Some(String::from("No matches"))
                }
            }
            _ => None,
        }
    }
}

impl View<()> for InfoLineView<'_> {
    fn render(&self, (): &mut (), bounds: Bounds, painter: &mut Painter) {
        let layer = painter.create_layer((self.parent_unique_id, "info line view"), bounds);
        let mut layer = layer.lock().unwrap();
        let area = bounds.grid_bounds();
        let buf = &mut layer.buf;

        let style = match self.focus {
            true => self.theme.info_line,
            false => self.theme.info_line_unfocused,
        };

        let mut left = String::from("    ");
        for item in &self.config.left {
            if let Some(item) = self.get_info_item(item) {
                left.push_str(&item);
                left.push_str(&" ".repeat(self.config.padding));
            }
        }

        let mut right = String::from(" ");
        for item in &self.config.right {
            if let Some(item) = self.get_info_item(item) {
                right.push_str(&item);
                right.push_str(&" ".repeat(self.config.padding));
            }
        }
        right.push(' ');
        let right_width = right.width();

        {
            let mut output_area = area;
            output_area.x = (output_area.x + output_area.width).saturating_sub(right_width);
            buf.draw_string(
                output_area.x as u16,
                output_area.y as u16,
                &right,
                output_area.into(),
                Style::default(),
            );
        }

        buf.draw_string(area.x as u16, area.y as u16, &left, area.into(), style);
        buf.set_style(area.into(), style);
    }
}
