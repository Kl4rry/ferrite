use tui::widgets::Widget;
use unicode_width::UnicodeWidthStr;

use crate::core::theme::EditorTheme;

pub struct SplashWidget<'a> {
    theme: &'a EditorTheme,
}

impl<'a> SplashWidget<'a> {
    pub fn new(theme: &'a EditorTheme) -> Self {
        Self { theme }
    }
}

const SPLASH: &str = r#"
╭────────────────────────────────────╮
│     ______               _ __      │
│    / ____/__  __________(_) /____  │
│   / /_  / _ \/ ___/ ___/ / __/ _ \ │
│  / __/ /  __/ /  / /  / / /_/  __/ │
│ /_/    \___/_/  /_/  /_/\__/\___/  │
│                                    │
│      Command palette CTRL + P      │
│       Browse files CTRL + O        │
│           Quit CTRL + Q            │
╰────────────────────────────────────╯
"#;

impl Widget for SplashWidget<'_> {
    fn render(self, area: tui::layout::Rect, buf: &mut tui::buffer::Buffer) {
        let lines = SPLASH.lines().count();
        let width = SPLASH
            .lines()
            .map(|line| line.width())
            .max()
            .unwrap_or_default();
        let left = (area.width as usize).saturating_sub(width) / 2;
        let top = (area.height as usize).saturating_sub(lines) / 2;
        if area.width as usize >= width {
            for (i, line) in SPLASH.lines().enumerate() {
                buf.set_string(
                    area.left() + left as u16,
                    area.top() + top as u16 + i as u16,
                    line,
                    self.theme.text,
                );
            }
        }
    }
}
