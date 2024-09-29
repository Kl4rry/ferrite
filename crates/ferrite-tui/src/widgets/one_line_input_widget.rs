use ferrite_core::{buffer::Buffer, theme::EditorTheme};
use tui::{layout::Rect, style::Style, widgets::StatefulWidget};

use crate::glue::convert_style;

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
        let view_id = buffer.get_first_view_or_create();
        buffer.set_view_lines(view_id, 1);
        buffer.set_view_columns(view_id, area.width.into());
        buffer.views[view_id].clamp_cursor = true;
        let view = buffer.get_buffer_view(view_id);
        buf.set_stringn(
            area.x,
            area.y,
            " ".repeat(area.width.into()),
            area.width.into(),
            convert_style(&self.theme.text),
        );
        buf.set_stringn(
            area.x,
            area.y,
            view.lines[0].text.to_string(),
            area.width.into(),
            convert_style(&self.theme.text),
        );
        let cursor = buffer.cursor_grapheme_column(view_id) as i64 - buffer.col_pos(view_id) as i64;
        let anchor = buffer.anchor_grapheme_column(view_id) as i64 - buffer.col_pos(view_id) as i64;
        let start = cursor.min(anchor).clamp(0, area.width as i64);
        let end = cursor.max(anchor).clamp(0, area.width as i64);
        let rect = Rect {
            x: area.x + start as u16,
            y: area.y,
            width: (end - start) as u16,
            height: 1,
        };
        buf.set_style(rect, convert_style(&self.theme.selection));

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
