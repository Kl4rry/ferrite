use tui::{
    layout::Rect,
    widgets::{StatefulWidget, Widget},
};
use utility::graphemes::{tab_width_at, RopeGraphemeExt, TAB_WIDTH};

use super::info_line::InfoLine;
use crate::core::{
    buffer::{Buffer, BufferPos, Selection},
    config::Config,
    theme::EditorTheme,
};

pub struct EditorWidget<'a> {
    theme: &'a EditorTheme,
    config: &'a Config,
    has_focus: bool,
    branch: Option<String>,
}

impl<'a> EditorWidget<'a> {
    pub fn new(
        theme: &'a EditorTheme,
        config: &'a Config,
        has_focus: bool,
        branch: Option<String>,
    ) -> Self {
        Self {
            theme,
            config,
            has_focus,
            branch,
        }
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
        let Self {
            theme,
            config,
            has_focus,
            branch,
        } = self;
        let line_number_max_width = buffer.len_lines().to_string().len();
        let width = area.width;
        let height = area.height - 1;

        buffer.set_view_lines(height.into());
        let left_offset = 4 + line_number_max_width;
        buffer.set_view_columns((width as usize).saturating_sub(left_offset));

        buf.set_style(area, theme.background);

        let current_line_number = buffer.cursor_line_idx() + 1;

        let view = buffer.get_buffer_view();
        {
            let mut line_buffer = String::with_capacity(width.into());

            for (i, (line, line_number)) in view
                .lines
                .iter()
                .zip((buffer.line_pos() + 1)..=buffer.line_pos() + buffer.len_lines())
                .enumerate()
            {
                let line_number_str = line_number.to_string();
                let line_number_str = format!(
                    " {}{} │",
                    " ".repeat(line_number_max_width - line_number_str.len()),
                    line_number
                );
                let line_nr_theme = if line_number == current_line_number {
                    theme.current_line_nr
                } else {
                    theme.line_nr
                };

                buf.set_stringn(
                    area.x,
                    area.y + i as u16,
                    &line_number_str,
                    width.into(),
                    line_nr_theme,
                );

                let text = line.text.line_without_line_ending(0);
                line_buffer.push_str(&" ".repeat(line.col_start_offset));
                let mut current_width = 0;
                for grapheme in text.grapehemes() {
                    if grapheme.starts_width_char('\t') {
                        let tab_width = tab_width_at(current_width, TAB_WIDTH);
                        line_buffer.push_str(&" ".repeat(tab_width));
                        current_width += tab_width;
                        continue;
                    }

                    for ch in grapheme.chars() {
                        if ch.is_ascii_control() {
                            line_buffer.push('�');
                        } else {
                            line_buffer.push(ch);
                        }
                    }

                    current_width += grapheme.width(current_width);
                }

                line_buffer.push(' ');

                buf.set_stringn(
                    area.x + left_offset as u16,
                    area.y + i as u16,
                    &line_buffer,
                    width as usize - left_offset,
                    theme.text,
                );

                line_buffer.clear();
            }

            for ruler in config.rulers.iter().copied() {
                let real_col =
                    ruler as i64 - buffer.col_pos() as i64 + area.x as i64 + left_offset as i64 + 1;
                if (area.left().into()..area.right().into()).contains(&real_col) {
                    for y in area.top()..(area.bottom() - 1) {
                        let cell = buf.get_mut(real_col as u16, y);
                        if cell.symbol.chars().all(|ch| ch.is_whitespace()) {
                            cell.set_symbol("│");
                            cell.set_style(theme.ruler);
                        }
                    }
                }
            }

            if has_focus {
                'exit: {
                    if let Some((_, row)) = buffer.cursor_view_pos(height.into()) {
                        let column =
                            buffer.cursor_grapheme_column() as i64 - buffer.col_pos() as i64;

                        if view.lines.get(row).is_some() {
                            let x = area.x as i64 + column + left_offset as i64;
                            if x < area.width as i64 && column >= 0 {
                                buf.set_style(
                                    Rect {
                                        x: x as u16,
                                        y: area.y + row as u16,
                                        width: 1,
                                        height: 1,
                                    },
                                    theme.text.add_modifier(tui::style::Modifier::REVERSED),
                                );
                                break 'exit;
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
                            let cell = buf.get_mut(x + left_offset as u16 + area.x, y + area.y);
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
                branch: &branch,
            };
            info_line.render(Rect::new(0, height, width, 1), buf);
        }
    }
}
