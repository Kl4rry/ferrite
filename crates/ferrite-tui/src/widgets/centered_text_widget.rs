use ferrite_core::theme::EditorTheme;
use ferrite_utility::graphemes::RopeGraphemeExt;
use ropey::Rope;
use tui::widgets::Widget;

use crate::glue::convert_style;

pub struct CenteredTextWidget<'a> {
    theme: &'a EditorTheme,
    text: &'a str,
}

impl<'a> CenteredTextWidget<'a> {
    pub fn new(theme: &'a EditorTheme, text: &'a str) -> Self {
        Self { theme, text }
    }
}

impl Widget for CenteredTextWidget<'_> {
    fn render(self, area: tui::layout::Rect, buf: &mut tui::buffer::Buffer) {
        if area.area() == 0 {
            return;
        }
        let rope = Rope::from_str(self.text);
        let lines = rope.len_lines();
        // Will panic if text is more then u16::MAX lines
        let top_padding = (area.height / 2).saturating_sub(lines as u16 / 2);
        for (i, y) in ((area.y + top_padding)..(area.y + area.height)).enumerate() {
            let Some(line) = rope.get_line(i) else {
                break;
            };
            let text_width = rope.width(0);
            let left_padding = (area.width / 2).saturating_sub(text_width as u16 / 2);
            let x = area.x + left_padding;
            buf.set_stringn(
                x,
                y,
                line.as_str().unwrap(),
                (area.width - left_padding) as usize,
                convert_style(&self.theme.text),
            );
        }
    }
}
