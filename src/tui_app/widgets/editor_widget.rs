use tui::{
    layout::Rect,
    style::Style,
    widgets::{StatefulWidget, Widget},
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;
use utility::line_ending::line_without_line_ending;

use super::info_line::InfoLine;
use crate::core::{editor::Editor, theme::EditorTheme};

pub struct EditorWidget<'a> {
    theme: &'a EditorTheme,
}

impl<'a> EditorWidget<'a> {
    pub fn new(theme: &'a EditorTheme) -> Self {
        Self { theme }
    }
}

impl<'a> StatefulWidget for EditorWidget<'a> {
    type State = Editor;

    fn render(
        self,
        area: tui::layout::Rect,
        buf: &mut tui::buffer::Buffer,
        editor: &mut Self::State,
    ) {
        let Self { theme } = self;
        let line_number_max_width = editor.buffer.len_lines().to_string().len();
        let width = area.width;
        let height = area.height - 1;

        buf.set_style(area, theme.background);

        let mut left_offset = 0;
        {
            let mut line_buffer = String::with_capacity(width.into());
            let view = editor.buffer.get_buffer_view(height.into());

            for (i, (line, line_number)) in view
                .lines
                .into_iter()
                .zip(
                    (editor.buffer.line_pos() + 1)
                        ..=editor.buffer.line_pos() + editor.buffer.len_lines(),
                )
                .enumerate()
            {
                let line_number = line_number.to_string();
                let line_number = format!(
                    "{}{} │",
                    " ".repeat(line_number_max_width - line_number.len()),
                    line_number
                );
                buf.set_stringn(0, i as u16, &line_number, width.into(), theme.line_nr);
                left_offset = line_number.width_cjk();

                let line = line_without_line_ending(&line, 0);
                for chunk in line.chunks() {
                    line_buffer.push_str(chunk);
                }

                buf.set_stringn(
                    left_offset as u16,
                    i as u16,
                    &line_buffer,
                    width as usize - left_offset,
                    theme.text,
                );

                line_buffer.clear();
            }

            {
                if let Some((column, row)) = editor.buffer.cursor_view_pos(height.into()) {
                    if let Some(line) = editor.buffer.get_line(row + editor.buffer.line_pos()) {
                        let mut view_col = 0;
                        for chunk in line.chunks() {
                            for (i, grapheme) in chunk.grapheme_indices(true) {
                                if i >= column {
                                    break;
                                }
                                view_col += grapheme.width_cjk();
                            }
                            buf.set_style(
                                Rect {
                                    x: area.x + view_col as u16 + left_offset as u16,
                                    y: area.y + row as u16,
                                    width: 1,
                                    height: 1,
                                },
                                Style::default().bg(tui::style::Color::Green),
                            )
                        }
                    }
                }
            }

            let info_line = InfoLine {
                theme,
                encoding: editor.buffer.encoding,
                file: editor.buffer.file(),
                column: editor.buffer.cursor_pos().1 + 1,
                line: editor.buffer.cursor_grapheme_column(),
            };
            info_line.render(Rect::new(0, height, width, 1), buf);
        }
    }
}
