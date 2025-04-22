use ferrite_core::{
    about::{git_hash_short, version},
    theme::EditorTheme,
};
use tui::widgets::Widget;
use unicode_width::UnicodeWidthStr;

use crate::glue::convert_style;

pub struct SplashWidget<'a> {
    theme: &'a EditorTheme,
}

impl<'a> SplashWidget<'a> {
    pub fn new(theme: &'a EditorTheme) -> Self {
        Self { theme }
    }
}

impl Widget for SplashWidget<'_> {
    fn render(self, area: tui::layout::Rect, buf: &mut tui::buffer::Buffer) {
        let arena = ferrite_ctx::Ctx::arena();
        let splash = bumpalo::format!(in &arena,
            r#"
  Ferrite v{} {}

CTRL + P    Command palette
CTRL + O    Browse files
CTRL + Q    Quit
"#,
            version(),
            git_hash_short()
        );
        let lines = splash.lines().count();
        let width = splash
            .lines()
            .map(|line| line.width())
            .max()
            .unwrap_or_default();
        let left = (area.width as usize).saturating_sub(width) / 2;
        let top = (area.height as usize).saturating_sub(lines) / 2;
        if area.width as usize >= width {
            for (i, line) in splash.lines().enumerate() {
                buf.set_string(
                    area.left() + left as u16,
                    area.top() + top as u16 + i as u16,
                    line,
                    convert_style(&self.theme.text),
                );
            }
        }
    }
}
