use tui::{layout::Rect, widgets::StatefulWidget};
use unicode_width::UnicodeWidthStr;

use super::{completer_widget::CompleterWidget, one_line_input_widget::OneLineInputWidget};
use crate::core::{
    palette::{CommandPalette, PaletteState, SelectedPrompt},
    theme::EditorTheme,
};

pub struct CmdPaletteWidget<'a> {
    theme: &'a EditorTheme,
    focused: bool,
    total_area: Rect,
}

impl<'a> CmdPaletteWidget<'a> {
    pub fn new(theme: &'a EditorTheme, focused: bool, total_area: Rect) -> Self {
        Self {
            theme,
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
                let prompt_width = prompt.width() as u16;
                buf.set_stringn(area.x, area.y, prompt, area.width.into(), self.theme.text);
                let input_area = Rect {
                    x: area.x + prompt_width,
                    y: area.y,
                    width: area.width.saturating_sub(prompt_width),
                    height: 1,
                };

                OneLineInputWidget::new(self.theme, self.focused).render(input_area, buf, buffer);

                if mode == "command" {
                    let completer_area = {
                        let mut completer_area = self.total_area;
                        completer_area.height = completer_area.height.saturating_sub(1);
                        completer_area
                    };

                    CompleterWidget::new(self.theme).render(completer_area, buf, completer);
                }
            }
            PaletteState::Message(msg) => {
                buf.set_stringn(area.x, area.y, msg, area.width.into(), self.theme.text);
            }
            PaletteState::Error(msg) => {
                buf.set_stringn(
                    area.x,
                    area.y,
                    msg,
                    area.width.into(),
                    self.theme.error_text,
                );
            }
            PaletteState::Nothing => (),
            PaletteState::Prompt {
                selected,
                prompt,
                alt1_char,
                alt2_char,
                ..
            } => {
                let prompt = format!("{prompt}: ");
                let prompt_width = prompt.width() as u16;
                buf.set_stringn(area.x, area.y, &prompt, area.width.into(), self.theme.text);
                let alt1 = if *selected == SelectedPrompt::Alt1 {
                    alt1_char.to_ascii_uppercase()
                } else {
                    *alt1_char
                };

                let alt2 = if *selected == SelectedPrompt::Alt2 {
                    alt2_char.to_ascii_uppercase()
                } else {
                    *alt2_char
                };

                buf.set_stringn(
                    area.x + prompt_width,
                    area.y,
                    format!("{alt1} / {alt2}"),
                    area.width.into(),
                    self.theme.text,
                );
            }
        };
    }
}
