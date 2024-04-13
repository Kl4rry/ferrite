use tui::{layout::Rect, style::Style, widgets::StatefulWidget};

use crate::ferrite_core::{buffer::Buffer, theme::EditorTheme};

pub struct OneLineInputWidget<'a> {
    theme: &'a EditorTheme,
    focused: bool,
}

impl<'a> OneLineInputWidget<'a> {
    pub fn new(theme: &'a EditorTheme, focused: bool) -> Self {
        Self { theme, focused }
    }
}

impl StatefulWidget for OneLineInputWidget<'_> {
    type State = Buffer;

    fn render(self, area: Rect, buf: &mut tui::buffer::Buffer, buffer: &mut Self::State) {
        assert_eq!(area.height, 1);
        buffer.set_view_lines(1);
        buffer.set_view_columns(area.width.into());
        buffer.clamp_cursor = true;
        let view = buffer.get_buffer_view();
        buf.set_stringn(
            area.x,
            area.y,
            " ".repeat(area.width.into()),
            area.width.into(),
            self.theme.text,
        );
        buf.set_stringn(
            area.x,
            area.y,
            view.lines[0].text.to_string(),
            area.width.into(),
            self.theme.text,
        );
        let cursor = buffer.cursor_grapheme_column() as i64 - buffer.col_pos() as i64;
        let anchor = buffer.anchor_grapheme_column() as i64 - buffer.col_pos() as i64;
        let start = cursor.min(anchor).clamp(0, area.width as i64);
        let end = cursor.max(anchor).clamp(0, area.width as i64);
        let rect = Rect {
            x: area.x + start as u16,
            y: area.y,
            width: (end - start) as u16,
            height: 1,
        };
        buf.set_style(rect, self.theme.selection);

        let cursor_area = Rect {
            x: area.x + cursor as u16,
            y: area.y,
            width: 1,
            height: 1,
        };

        if cursor_area.intersects(area) && self.focused {
            buf.set_style(
                cursor_area,
                Style::default().add_modifier(tui::style::Modifier::REVERSED),
            );
        }
    }
}
