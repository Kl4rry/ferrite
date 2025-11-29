use std::sync::Arc;

use ferrite_geom::rect::Rect;
use ferrite_runtime::{Bounds, Painter, View};
use tui::{
    layout,
    widgets::{Block, BorderType, Borders, Widget},
};
use unicode_width::UnicodeWidthStr;

use crate::{cmd::Cmd, config::keymap::Keymapping, engine::Engine, theme::EditorTheme};

pub struct ChordView {
    theme: Arc<EditorTheme>,
}

impl ChordView {
    pub fn new(theme: Arc<EditorTheme>) -> Self {
        Self { theme }
    }
}

impl View<Engine> for ChordView {
    fn render(&self, engine: &mut Engine, bounds: Bounds, painter: &mut Painter) {
        let key_mappings = engine.get_current_keymappings();
        let total_area = bounds.grid_bounds();

        let height = total_area.height.min(
            key_mappings
                .iter()
                .filter(|Keymapping { cmd, .. }| {
                    *cmd != Cmd::Escape
                        && *cmd
                            != Cmd::InputMode {
                                name: String::from("normal"),
                            }
                })
                .count()
                + 2,
        );

        let mut lines = Vec::new();
        let mut longest = 0;
        let mut left_col_width = 0;
        for Keymapping { key, cmd, .. } in key_mappings
            .iter()
            .filter(|Keymapping { cmd, .. }| {
                *cmd != Cmd::Escape
                    && *cmd
                        != Cmd::InputMode {
                            name: String::from("normal"),
                        }
            })
            .take(height)
        {
            let mapping = format!("{}{} ", key.keycode.to_string(), key.modifiers);
            let cmd = cmd.to_string();
            longest = longest.max(mapping.width() + cmd.width() + 1);
            left_col_width = left_col_width.max(mapping.width());
            lines.push((mapping, cmd));
        }

        let width = total_area.width.min(longest + 4);

        if width < 3 || height < 3 {
            return;
        }

        if width >= total_area.width || height >= total_area.height {
            return;
        }

        let left = bounds.view_bounds().width as f32 - (width as f32 * bounds.cell_size().x);
        let top = bounds.view_bounds().height as f32 - (height as f32 * bounds.cell_size().y);
        let bounds = Bounds::new(
            Rect::new(
                left as usize,
                top as usize,
                (width as f32 * bounds.cell_size().x) as usize,
                (height as f32 * bounds.cell_size().y) as usize,
            ),
            bounds.cell_size(),
            bounds.rounding,
        );
        let layer = painter.create_layer("chord view", bounds);
        let mut layer = layer.lock().unwrap();
        let area: Rect = layer.buf.area.into();
        let buf = &mut layer.buf;

        Block::default()
            .title("Chords")
            .borders(Borders::ALL)
            .border_style(self.theme.border)
            .border_type(BorderType::Plain)
            .style(self.theme.background)
            .render(area.into(), buf);

        let inner_area = layout::Rect::from(area).inner(layout::Margin::new(1, 1));
        for (i, (mapping, cmd)) in lines.into_iter().enumerate() {
            let mut line = format!(" {mapping}");
            line.push_str(&" ".repeat(left_col_width.saturating_sub(mapping.width()) + 1));
            line.push_str(&cmd);
            line.push_str(
                &" ".repeat((inner_area.width as usize).saturating_sub(line.width() as usize)),
            );

            buf.set_stringn(
                inner_area.left(),
                inner_area.top() + i as u16,
                line,
                inner_area.width.into(),
                self.theme.text,
            );
        }
    }
}
