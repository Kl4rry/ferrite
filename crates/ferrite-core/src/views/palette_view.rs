use std::sync::Arc;

use ferrite_geom::rect::Rect;
use ferrite_runtime::{Bounds, Painter, View};

use super::one_line_input_view::OneLineInputView;
use crate::{
    config::editor::Editor,
    palette::{CommandPalette, PaletteMode, PaletteState},
    theme::EditorTheme,
    views::completer_view::CompleterView,
};

pub struct PaletteView {
    theme: Arc<EditorTheme>,
    config: Arc<Editor>,
    focused: bool,
}

impl PaletteView {
    pub fn new(theme: Arc<EditorTheme>, config: Arc<Editor>, focused: bool) -> Self {
        Self {
            theme,
            config,
            focused,
        }
    }
}

impl View<CommandPalette> for PaletteView {
    fn render(&self, palette: &mut CommandPalette, bounds: Bounds, painter: &mut Painter) {
        let cell_size = bounds.cell_size();
        let total_area = bounds.grid_bounds();
        let total_view_bounds = bounds.view_bounds();
        // Calculate size of palette
        let palette_bounds = Bounds::new(
            Rect::new(
                total_view_bounds.left(),
                total_view_bounds
                    .bottom()
                    .saturating_sub((palette.height() as f32 * cell_size.y).round() as usize),
                total_view_bounds.width,
                (palette.height().min(total_area.height) as f32 * cell_size.y).round() as usize,
            ),
            cell_size,
            bounds.rounding,
        );
        let area = palette_bounds.grid_bounds();

        let layer = painter.create_layer("command palette view", palette_bounds);
        let mut layer = layer.lock().unwrap();
        let buf = &mut layer.buf;

        buf.set_style(area.into(), self.theme.background);

        match palette.state() {
            PaletteState::Input {
                buffer,
                prompt,
                completer,
                mode,
                ..
            } => {
                OneLineInputView::new(
                    self.theme.clone(),
                    self.config.clone(),
                    self.focused,
                    "palette input field",
                )
                .set_left_prompt(format!(" {}", prompt))
                .render(buffer, palette_bounds, painter);

                if self.focused && (*mode == PaletteMode::Command || *mode == PaletteMode::Shell) {
                    let mut completer_bounds = total_view_bounds;
                    completer_bounds.height =
                        completer_bounds.height.saturating_sub(cell_size.y as usize);

                    CompleterView::new(self.theme.clone()).render(
                        completer,
                        Bounds::new(completer_bounds, cell_size, bounds.rounding),
                        painter,
                    );
                }
            }
            PaletteState::Message(msg) => {
                for (i, line) in msg.lines().enumerate() {
                    if i >= area.height {
                        break;
                    }
                    buf.set_stringn(
                        (area.x + 1) as u16,
                        (area.y + i) as u16,
                        line,
                        area.width.saturating_sub(1),
                        self.theme.text,
                    );
                }
            }
            PaletteState::Error(msg) => {
                for (i, line) in msg.lines().enumerate() {
                    if i >= area.height {
                        break;
                    }
                    buf.set_stringn(
                        (area.x + 1) as u16,
                        (area.y + i) as u16,
                        line,
                        area.width.saturating_sub(1),
                        self.theme.error_text,
                    );
                }
            }
            PaletteState::Nothing => (),
            PaletteState::Prompt {
                selected,
                prompt,
                alt1_char,
                alt2_char,
                ..
            } => {
                let msg = CommandPalette::get_prompt(*selected, prompt, *alt1_char, *alt2_char);
                for (i, line) in msg.lines().enumerate() {
                    if i >= area.height {
                        break;
                    }
                    buf.set_stringn(
                        (area.x + 1) as u16,
                        (area.y + i) as u16,
                        line,
                        area.width.saturating_sub(1),
                        self.theme.text,
                    );
                }
            }
        };
    }
}
