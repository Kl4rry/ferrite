use tui::widgets::Widget;

use crate::ferrite_core::theme::EditorTheme;

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
            cell.set_style(self.theme.background);
        }
    }
}
