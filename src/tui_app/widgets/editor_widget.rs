use ropey::RopeSlice;
use tui::{
    layout::Rect,
    style::Style,
    widgets::{StatefulWidget, Widget},
};
use unicode_width::UnicodeWidthStr;
use utility::graphemes::RopeGraphemeExt;

use super::info_line::InfoLine;
use crate::core::{
    buffer::{Buffer, BufferPos, Selection},
    theme::EditorTheme,
};

pub struct EditorWidget<'a> {
    theme: &'a EditorTheme,
    has_focus: bool,
}

impl<'a> EditorWidget<'a> {
    pub fn new(theme: &'a EditorTheme, has_focus: bool) -> Self {
        Self { theme, has_focus }
    }
}

impl StatefulWidget for EditorWidget<'_> {
    type State = Buffer;

    fn render(
        self,
        area: tui::layout::Rect,
        buf: &mut tui::buffer::Buffer,
        buffer: &mut Self::State,
    ) {
        let Self { theme, has_focus } = self;
        let line_number_max_width = buffer.len_lines().to_string().len();
        let width = area.width;
        let height = area.height - 1;

        buffer.set_view_lines(height.into());

        buf.set_style(area, theme.background);

        let current_line_number = buffer.cursor_line_idx() + 1;

        let mut left_offset = 0;
        {
            let mut line_buffer = String::with_capacity(width.into());
            let view = buffer.get_buffer_view();

            for (i, (line, line_number)) in view
                .lines
                .into_iter()
                .zip((buffer.line_pos() + 1)..=buffer.line_pos() + buffer.len_lines())
                .enumerate()
            {
                let line_number_str = line_number.to_string();
                let line_number_str = format!(
                    " {}{} │",
                    " ".repeat(line_number_max_width - line_number_str.len()),
                    line_number
                );
                if line_number == current_line_number {
                    buf.set_stringn(
                        0,
                        i as u16,
                        &line_number_str,
                        width.into(),
                        theme.current_line_nr,
                    );
                } else {
                    buf.set_stringn(0, i as u16, &line_number_str, width.into(), theme.line_nr);
                }

                // This is correct because there are no tabs in the line numbers.
                left_offset = line_number_str.width_cjk();

                let line = line.line_without_line_ending(0);
                for chunk in line.chunks() {
                    for c in chunk.chars() {
                        if c == '\t' {
                            line_buffer.push_str("    ");
                        } else {
                            line_buffer.push(c);
                        }
                    }
                }

                line_buffer.push(' ');

                buf.set_stringn(
                    left_offset as u16,
                    i as u16,
                    &line_buffer,
                    width as usize - left_offset,
                    theme.text,
                );

                line_buffer.clear();
            }

            if has_focus {
                'exit: {
                    if let Some((_, row)) = buffer.cursor_view_pos(height.into()) {
                        let column = buffer.cursor_grapheme_column();
                        if let Some(line) = buffer.get_line(row + buffer.line_pos()) {
                            let mut view_col = 0;
                            let mut last = RopeSlice::from("");
                            for grapheme in line.grapehemes() {
                                if view_col >= column {
                                    let x = area.x + view_col as u16 + left_offset as u16;
                                    if x <= area.width {
                                        buf.set_style(
                                            Rect {
                                                x,
                                                y: area.y + row as u16,
                                                width: 1,
                                                height: 1,
                                            },
                                            Style::default()
                                                .add_modifier(tui::style::Modifier::REVERSED),
                                        );
                                        break 'exit;
                                    }
                                }
                                view_col += grapheme.width();
                                last = grapheme;
                            }
                            // Edge case for last line
                            if last.get_line_ending().is_none() {
                                let x = area.x + view_col as u16 + left_offset as u16;
                                if x < area.right() {
                                    buf.set_style(
                                        Rect {
                                            x,
                                            y: area.y + row as u16,
                                            width: 1,
                                            height: 1,
                                        },
                                        Style::default()
                                            .add_modifier(tui::style::Modifier::REVERSED),
                                    );
                                }
                            }
                        }
                    }
                }
            }

            if let Some(bg) = theme.selection.bg {
                let Selection { start, end } = buffer.get_view_selection();

                for y in 0..buf.area.height - 2 {
                    for x in 0..(buf.area.width - (left_offset as u16)) {
                        let current = BufferPos {
                            column: x.into(),
                            line: y.into(),
                        };
                        if current >= start && current < end {
                            let cell = buf.get_mut(x + left_offset as u16, y);
                            cell.bg = bg;
                        }
                    }
                }
            }

            let info_line = InfoLine {
                theme,
                encoding: buffer.encoding,
                file: buffer.file(),
                line: buffer.cursor_pos().1 + 1,
                column: buffer.cursor_grapheme_column() + 1,
                dirty: buffer.is_dirty(),
            };
            info_line.render(Rect::new(0, height, width, 1), buf);
        }
    }
}
