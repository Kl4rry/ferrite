use std::sync::Arc;

use ferrite_geom::rect::Rect;
use ferrite_runtime::{Bounds, View};
use unicode_width::UnicodeWidthStr;

use crate::{
    about::{git_hash_short, version},
    theme::EditorTheme,
};

pub struct SplashView {
    theme: Arc<EditorTheme>,
}

impl SplashView {
    pub fn new(theme: Arc<EditorTheme>) -> Self {
        Self { theme }
    }
}

impl View<()> for SplashView {
    fn render(&self, (): &mut (), bounds: Bounds, painter: &mut ferrite_runtime::Painter) {
        let layer = painter.create_layer("splash view", bounds);
        let mut layer = layer.lock().unwrap();
        let area: Rect = layer.buf.area.into();
        let buf = &mut layer.buf;

        let arena = ferrite_ctx::Ctx::arena();
        let splash = ferrite_ctx::format!(in &arena,
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
                    (area.left() + left) as u16,
                    (area.top() + top + i) as u16,
                    line,
                    self.theme.text,
                );
            }
        }
    }
}
