use std::{hash::Hash, sync::Arc};

use ferrite_runtime::{Bounds, Painter, View};
use ferrite_utility::{graphemes::RopeGraphemeExt, tui_buf_ext::TuiBufExt};
use ropey::Rope;

use crate::theme::EditorTheme;

pub struct CenteredTextView<I> {
    theme: Arc<EditorTheme>,
    text: String,
    id: I,
}

impl<I> CenteredTextView<I> {
    pub fn new(theme: Arc<EditorTheme>, text: String, id: I) -> Self {
        Self { theme, text, id }
    }
}

impl<I> View<()> for CenteredTextView<I>
where
    I: Hash + Copy + 'static,
{
    fn render(&self, (): &mut (), bounds: Bounds, painter: &mut Painter) {
        if bounds.grid_bounds().area() == 0 {
            return;
        }

        let area = bounds.grid_bounds();
        let layer = painter.create_layer(self.id, bounds);
        let mut layer = layer.lock().unwrap();
        let buf = &mut layer.buf;

        let rope = Rope::from_str(&self.text);
        let lines = rope.len_lines();
        // Will panic if text is more then u16::MAX lines
        let top_padding = (area.height / 2).saturating_sub(lines / 2);
        for (i, y) in ((area.y + top_padding)..(area.y + area.height)).enumerate() {
            let Some(line) = rope.get_line(i) else {
                break;
            };
            let text_width = rope.width(0);
            let left_padding = (area.width / 2).saturating_sub(text_width / 2);
            let x = area.x + left_padding;
            buf.draw_string(
                x as u16,
                y as u16,
                line.as_str().unwrap(),
                area.into(),
                self.theme.text,
            );
        }
    }
}
