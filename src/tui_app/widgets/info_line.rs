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

        buf.set_stringn(
            area.x + 1,
            area.y,
            &file,
            area.width.into(),
            self.theme.info_line,
        );

        {
            let output = format!(" {}:{} {} ", self.line, self.column, self.encoding.name());
            let len = output.width_cjk();
            let mut output_area = area;
            output_area.x = (output_area.x + output_area.width) - len as u16;
            buf.set_stringn(
                output_area.x,
                output_area.y,
                &output,
                output_area.width.into(),
                Default::default(),
            );
        }

        buf.set_style(area, self.theme.info_line);
    }
}
