use std::path::Path;

use encoding_rs::Encoding;
use ferrite_core::{byte_size::format_byte_size, config::InfoLineConfig, theme::EditorTheme};
use tui::{style::Style, widgets::Widget};
use unicode_width::UnicodeWidthStr;

use crate::glue::convert_style;

pub struct InfoLine<'a> {
    pub theme: &'a EditorTheme,
    pub config: &'a InfoLineConfig,
    pub focus: bool,
    pub encoding: &'static Encoding,
    pub file: Option<&'a Path>,
    pub column: usize,
    pub line: usize,
    pub dirty: bool,
    pub branch: &'a Option<String>,
    pub language: String,
    pub size: usize,
    pub spinner: Option<char>,
    pub read_only: bool,
}

impl InfoLine<'_> {
    pub fn get_info_item(&self, item: &str) -> Option<String> {
        match item {
            "file" => {
                let mut file = self
                    .file
                    .map(|p| p.to_string_lossy())
                    .unwrap_or_else(|| "[scratch]".into())
                    .to_string();

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
            _ => None,
        }
    }
}

impl Widget for InfoLine<'_> {
    fn render(self, area: tui::layout::Rect, buf: &mut tui::buffer::Buffer) {
        let style = match self.focus {
            true => convert_style(&self.theme.info_line),
            false => convert_style(&self.theme.info_line_unfocused),
        };

        let mut left = String::from(" ");
        for item in &self.config.left {
            if let Some(item) = self.get_info_item(item) {
                left.push_str(&item);
                left.push_str(&" ".repeat(self.config.padding));
            }
        }
        let left_width = left.width();

        let mut center = String::from(" ");
        for item in &self.config.center {
            if let Some(item) = self.get_info_item(item) {
                center.push_str(&item);
                center.push_str(&" ".repeat(self.config.padding));
            }
        }
        let center_width = center.width();

        let mut right = String::from(" ");
        for item in &self.config.right {
            if let Some(item) = self.get_info_item(item) {
                right.push_str(&item);
                right.push_str(&" ".repeat(self.config.padding));
            }
        }
        let right_width = right.width();

        buf.set_stringn(area.x, area.y, &left, area.width.into(), style);

        if area.width as usize > left_width + right_width {
            let mut output_area = area;
            output_area.x = (output_area.x + output_area.width) - right_width as u16;
            buf.set_string(output_area.x, output_area.y, &right, Style::default());
        }

        if area.width as usize > left_width + center_width + right_width {
            let center_max_width = area.width as usize - left_width - right_width;
            let padding = (center_max_width - center_width / 2) / 2;
            buf.set_stringn(
                area.x + padding as u16,
                area.y,
                &center,
                area.width.into(),
                style,
            );
        }

        buf.set_style(area, style);
    }
}
