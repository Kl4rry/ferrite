use tui::{layout::Rect, widgets::StatefulWidget};
use unicode_width::UnicodeWidthStr;

use super::one_line_input_widget::OneLineInputWidget;
use crate::core::{
    palette::{CommandPalette, PaletteState, SelectedPrompt},
    theme::EditorTheme,
};

pub struct CmdPaletteWidget<'a> {
    theme: &'a EditorTheme,
    focused: bool,
}

impl<'a> CmdPaletteWidget<'a> {
    pub fn new(theme: &'a EditorTheme, focused: bool) -> Self {
        Self { theme, focused }
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
            PaletteState::Input { buffer, prompt, .. } => {
                let prompt_width = prompt.width_cjk() as u16;
                buf.set_stringn(area.x, area.y, prompt, area.width.into(), self.theme.text);
                let input_area = Rect {
                    x: area.x + prompt_width,
                    y: area.y,
                    width: area.width.saturating_sub(prompt_width),
                    height: 1,
                };

                OneLineInputWidget::new(self.theme, self.focused).render(input_area, buf, buffer);
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
                let prompt_width = prompt.width_cjk() as u16;
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
