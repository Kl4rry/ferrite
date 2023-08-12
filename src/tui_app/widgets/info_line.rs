use std::path::Path;

use encoding_rs::Encoding;
use tui::widgets::Widget;
use unicode_width::UnicodeWidthStr;

use crate::core::{config::InfoLineConfig, theme::EditorTheme};

pub struct InfoLine<'a> {
    pub theme: &'a EditorTheme,
    pub config: &'a InfoLineConfig,
    pub encoding: &'static Encoding,
    pub file: Option<&'a Path>,
    pub column: usize,
    pub line: usize,
    pub dirty: bool,
    pub branch: &'a Option<String>,
    pub language: String,
    pub size: usize,
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
            "size" => Some(humansize::format_size(self.size, humansize::BINARY)),
            _ => None,
        }
    }
}

impl Widget for InfoLine<'_> {
    fn render(self, area: tui::layout::Rect, buf: &mut tui::buffer::Buffer) {
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

        buf.set_stringn(
            area.x,
            area.y,
            &left,
            area.width.into(),
            self.theme.info_line,
        );

        if area.width as usize > left_width + right_width {
            let mut output_area = area;
            output_area.x = (output_area.x + output_area.width) - right_width as u16;
            buf.set_string(output_area.x, output_area.y, &right, Default::default());
        }

        if area.width as usize > left_width + center_width + right_width {
            let center_max_width = area.width as usize - left_width - right_width;
            let padding = (center_max_width - center_width / 2) / 2;
            buf.set_stringn(
                area.x + padding as u16,
                area.y,
                &center,
                area.width.into(),
                self.theme.info_line,
            );
        }

        buf.set_style(area, self.theme.info_line);
    }
}
