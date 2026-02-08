use std::sync::Arc;

use ferrite_runtime::{Bounds, Painter, View, any_view::AnyView};
use ferrite_utility::tui_buf_ext::TuiBufExt;

use super::one_line_input_view::OneLineInputView;
use crate::{
    config::{editor::Editor, keymap::Keymap},
    palette::{CommandPalette, PaletteMode, PaletteState},
    theme::EditorTheme,
    views::{
        completer_view::CompleterView, nullview::NullView, search_palette_view::SearchPaletteView,
    },
};

pub struct PaletteView {
    theme: Arc<EditorTheme>,
    keymap: Arc<Keymap>,
    focused: bool,
    input_view: OneLineInputView<&'static str>,
}

impl PaletteView {
    pub fn new(
        theme: Arc<EditorTheme>,
        config: Arc<Editor>,
        keymap: Arc<Keymap>,
        focused: bool,
    ) -> Self {
        Self {
            input_view: OneLineInputView::new(
                theme.clone(),
                config.clone(),
                focused,
                "palette input field",
            ),
            theme,
            keymap,
            focused,
        }
    }
}

impl View<CommandPalette> for PaletteView {
    fn render(&self, palette: &mut CommandPalette, bounds: Bounds, painter: &mut Painter) {
        let cell_size = bounds.cell_size();
        let total_view_bounds = bounds.view_bounds();
        let palette_bounds = bounds.bottom_lines(palette.height());
        let area = palette_bounds.grid_bounds();

        let layer = painter.create_layer("command palette view", palette_bounds);
        let mut layer = layer.lock().unwrap();
        let buf = &mut layer.buf;

        buf.set_style(area.into(), self.theme.background);

        match palette.state() {
            PaletteState::Input {
                input_state,
                prompt,
                completer,
                mode,
                ..
            } => {
                let view = match mode {
                    PaletteMode::Search => AnyView::new(SearchPaletteView::new(
                        self.theme.clone(),
                        self.keymap.clone(),
                        false,
                    )),
                    PaletteMode::Replace => AnyView::new(SearchPaletteView::new(
                        self.theme.clone(),
                        self.keymap.clone(),
                        true,
                    )),
                    _ => NullView::any(),
                };

                view.render(&mut (), palette_bounds.top_lines(1), painter);
                input_state.set_left_prompt(format!(" {}", prompt));
                self.input_view
                    .render(input_state, palette_bounds.bottom_lines(1), painter);

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
                    buf.draw_string(
                        (area.x + 1) as u16,
                        (area.y + i) as u16,
                        line,
                        area.into(),
                        self.theme.text,
                    );
                }
            }
            PaletteState::Error(msg) => {
                for (i, line) in msg.lines().enumerate() {
                    if i >= area.height {
                        break;
                    }
                    buf.draw_string(
                        (area.x + 1) as u16,
                        (area.y + i) as u16,
                        line,
                        area.into(),
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
                    buf.draw_string(
                        (area.x + 1) as u16,
                        (area.y + i) as u16,
                        line,
                        area.into(),
                        self.theme.text,
                    );
                }
            }
        };
    }
}
