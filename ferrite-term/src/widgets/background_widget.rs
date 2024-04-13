use ferrite_core::theme::EditorTheme;
use tui::widgets::Widget;

use crate::glue::convert_style;

pub struct BackgroundWidget<'a> {
    theme: &'a EditorTheme,
}

impl<'a> BackgroundWidget<'a> {
    pub fn new(theme: &'a EditorTheme) -> Self {
        Self { theme }
    }
}

impl Widget for BackgroundWidget<'_> {
    fn render(self, _: tui::layout::Rect, buf: &mut tui::buffer::Buffer) {
        for cell in &mut buf.content {
            cell.set_style(convert_style(&self.theme.background));
        }
    }
}
