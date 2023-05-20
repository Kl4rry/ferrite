use std::path::Path;

use encoding_rs::Encoding;
use tui::widgets::Widget;
use unicode_width::UnicodeWidthStr;

use crate::core::theme::EditorTheme;

pub struct InfoLine<'a> {
    pub theme: &'a EditorTheme,
    pub encoding: &'static Encoding,
    pub file: Option<&'a Path>,
    pub column: usize,
    pub line: usize,
    pub dirty: bool,
    pub branch: &'a Option<String>,
    pub language: String,
}

impl Widget for InfoLine<'_> {
    fn render(self, area: tui::layout::Rect, buf: &mut tui::buffer::Buffer) {
        let mut file = self
            .file
            .map(|p| p.to_string_lossy())
            .unwrap_or_else(|| "[scratch]".into())
            .to_string();

        if self.dirty {
            file += " *";
        }

        file.insert(0, ' ');

        buf.set_stringn(
            area.x,
            area.y,
            &file,
            area.width.into(),
            self.theme.info_line,
        );

        let left_width = file.width_cjk();

        {
            let output = format!(
                "{}:{} {} {} ",
                self.line,
                self.column,
                self.encoding.name(),
                self.language
            );
            let output = match self.branch {
                Some(branch) => format!(" {} {}", branch, output),
                None => output,
            };

            let output_width = output.width();
            if output_width + left_width < area.width.into() {
                let mut output_area = area;
                output_area.x = (output_area.x + output_area.width) - output_width as u16;
                buf.set_string(output_area.x, output_area.y, &output, Default::default());
            }
        }

        buf.set_style(area, self.theme.info_line);
    }
}
