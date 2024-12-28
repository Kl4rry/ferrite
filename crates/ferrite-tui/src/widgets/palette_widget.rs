use ferrite_core::{
    config::editor::Editor,
    palette::{CommandPalette, PaletteState},
    theme::EditorTheme,
};
use tui::{layout::Rect, widgets::StatefulWidget};
use unicode_width::UnicodeWidthStr;

use super::{completer_widget::CompleterWidget, one_line_input_widget::OneLineInputWidget};
use crate::glue::convert_style;

pub struct CmdPaletteWidget<'a> {
    theme: &'a EditorTheme,
    config: &'a Editor,
    focused: bool,
    total_area: Rect,
}

impl<'a> CmdPaletteWidget<'a> {
    pub fn new(
        theme: &'a EditorTheme,
        config: &'a Editor,
        focused: bool,
        total_area: Rect,
    ) -> Self {
        Self {
            theme,
            config,
            focused,
            total_area,
        }
    }
}

impl StatefulWidget for CmdPaletteWidget<'_> {
    type State = CommandPalette;

    fn render(
        self,
        area: tui::layout::Rect,
        buf: &mut tui::buffer::Buffer,
        state: &mut Self::State,
    ) {
        match state.state() {
            PaletteState::Input {
                buffer,
                prompt,
                completer,
                mode,
                ..
            } => {
                let prompt_width = prompt.width() as u16 + 1;
                buf.set_stringn(
                    area.x,
                    area.y,
                    format!(" {}", prompt),
                    area.width.into(),
                    convert_style(&self.theme.text),
                );
                let input_area = Rect {
                    x: area.x + prompt_width,
                    y: area.y,
                    width: area.width.saturating_sub(prompt_width),
                    height: 1,
                };

                OneLineInputWidget::new(self.theme, self.config, self.focused)
                    .render(input_area, buf, buffer);

                if self.focused && (mode == "command" || mode == "shell") {
                    let completer_area = {
                        let mut completer_area = self.total_area;
                        completer_area.height = completer_area.height.saturating_sub(1);
                        completer_area
                    };

                    CompleterWidget::new(self.theme).render(completer_area, buf, completer);
                }
            }
            PaletteState::Message(msg) => {
                for (i, line) in msg.lines().enumerate() {
                    if i >= area.height.into() {
                        break;
                    }
                    buf.set_stringn(
                        area.x + 1,
                        area.y + i as u16,
                        line,
                        (area.width as usize).saturating_sub(1),
                        convert_style(&self.theme.text),
                    );
                }
            }
            PaletteState::Error(msg) => {
                for (i, line) in msg.lines().enumerate() {
                    if i >= area.height.into() {
                        break;
                    }
                    buf.set_stringn(
                        area.x + 1,
                        area.y + i as u16,
                        line,
                        (area.width as usize).saturating_sub(1),
                        convert_style(&self.theme.error_text),
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
                    if i >= area.height.into() {
                        break;
                    }
                    buf.set_stringn(
                        area.x + 1,
                        area.y + i as u16,
                        line,
                        (area.width as usize).saturating_sub(1),
                        convert_style(&self.theme.text),
                    );
                }
            }
        };
    }
}
