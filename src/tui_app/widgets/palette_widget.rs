use tui::{layout::Rect, style::Style, widgets::StatefulWidget};
use unicode_width::UnicodeWidthStr;
use utility::graphemes::RopeGraphemeExt;

use crate::core::{
    palette::{CommandPalette, PaletteState, SelectedPrompt},
    theme::EditorTheme,
};

pub struct CmdPaletteWidget<'a> {
    theme: &'a EditorTheme,
}

impl<'a> CmdPaletteWidget<'a> {
    pub fn new(theme: &'a EditorTheme) -> Self {
        Self { theme }
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
                buf.set_stringn(
                    area.x + prompt_width,
                    area.y,
                    buffer.rope().last_n_columns(area.width.into()).to_string(),
                    area.width.into(),
                    self.theme.text,
                );
                let cursor = buffer.cursor_grapheme_column() as u16;
                let anchor = buffer.anchor_grapheme_column() as u16;
                let start = cursor.min(anchor);
                let end = cursor.max(anchor);
                buf.set_style(area, self.theme.text);
                let rect = Rect {
                    x: area.x + prompt_width + start,
                    y: area.y,
                    width: end - start,
                    height: 1,
                };
                buf.set_style(rect, self.theme.selection);

                buf.set_style(
                    Rect {
                        x: area.x + prompt_width + cursor,
                        y: area.y,
                        width: 1,
                        height: 1,
                    },
                    Style::default().add_modifier(tui::style::Modifier::REVERSED),
                );
            }
            PaletteState::Message(msg) => {
                buf.set_stringn(area.x, area.y, msg, area.width.into(), self.theme.text);
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
