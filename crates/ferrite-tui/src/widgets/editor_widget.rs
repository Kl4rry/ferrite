use std::ops::Add;

use ferrite_core::{
    buffer::{search::SearchMatch, Buffer, Selection},
    config::{self, Config, LineNumber},
    language::syntax::{Highlight, HighlightEvent},
    theme::EditorTheme,
};
use ferrite_utility::{
    graphemes::{tab_width_at, RopeGraphemeExt, TAB_WIDTH},
    point::Point,
};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use ropey::RopeSlice;
use tui::{
    layout::Rect,
    widgets::{Clear, StatefulWidget, Widget},
};
use unicode_width::UnicodeWidthStr;

use super::info_line::InfoLine;
use crate::{glue::convert_style, rect_ext::RectExt};

pub fn lines_to_left_offset(lines: usize) -> (usize, usize) {
    let line_number_max_width = lines.to_string().len().add(1).max(4);
    const MAGIC_NUMBER: usize = 4;
    let left_offset = MAGIC_NUMBER + line_number_max_width;
    (line_number_max_width, left_offset)
}

pub struct EditorWidget<'a> {
    theme: &'a EditorTheme,
    config: &'a Config,
    has_focus: bool,
    branch: Option<String>,
    spinner: Option<char>,
    pub line_nr: bool,
    pub info_line: bool,
}

impl<'a> EditorWidget<'a> {
    pub fn new(
        theme: &'a EditorTheme,
        config: &'a Config,
        has_focus: bool,
        branch: Option<String>,
        spinner: Option<char>,
    ) -> Self {
        Self {
            theme,
            config,
            has_focus,
            branch,
            spinner,
            line_nr: true,
            info_line: true,
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
        if area.area() == 0 {
            return;
        }

        Clear.render(area, buf);

        let Self {
            theme,
            config,
            has_focus,
            branch,
            spinner,
            line_nr,
            info_line,
        } = self;

        let (line_number_max_width, left_offset) =
            if line_nr && config.line_number != LineNumber::None {
                lines_to_left_offset(buffer.len_lines())
            } else {
                (0, 0)
            };

        let text_area = Rect {
            x: area.x + left_offset as u16,
            y: area.y,
            width: area.width.saturating_sub(left_offset as u16),
            height: area.height - info_line as u16,
        };

        buffer.set_view_lines(text_area.height.into());

        buffer.set_view_columns((text_area.width as usize).saturating_sub(left_offset));
        buf.set_style(area, convert_style(&theme.background));

        if line_nr {
            buf.set_style(
                Rect {
                    x: area.left(),
                    y: area.top(),
                    width: (line_number_max_width as u16 + 2).min(area.width),
                    height: area.height,
                },
                convert_style(&theme.line_nr),
            );
        }

        let current_line_number = buffer.cursor_line_idx() + 1;

        // We have to overwrite all rendered whitespace with the correct color
        let mut dim_cells = Vec::new();

        let mut grapheme_buffer = String::new();

        let view = buffer.get_buffer_view();
        {
            for (i, (line, line_number)) in view
                .lines
                .iter()
                .zip((buffer.line_pos() + 1)..=buffer.line_pos() + buffer.len_lines())
                .enumerate()
            {
                if line_nr {
                    let is_current_line = line_number == current_line_number;
                    let line_number = if (config.line_number == LineNumber::Absolute)
                        || is_current_line
                    {
                        line_number
                    } else {
                        (line_number as i64 - current_line_number as i64).unsigned_abs() as usize
                    };
                    let line_number_str = line_number.to_string();
                    let line_number_str = format!(
                        " {}{} ",
                        " ".repeat(
                            line_number_max_width
                                - line_number_str.len()
                                - is_current_line as usize
                        ),
                        line_number
                    );
                    let line_nr_theme = if is_current_line {
                        convert_style(&theme.current_line_nr)
                    } else {
                        convert_style(&theme.line_nr)
                    };

                    buf.set_stringn(
                        area.x,
                        area.y + i as u16,
                        &line_number_str,
                        area.width.into(),
                        line_nr_theme,
                    );

                    let start_offset = " ".repeat(line.col_start_offset);
                    if text_area.width > 0 {
                        buf.set_stringn(
                            text_area.x,
                            text_area.y + i as u16,
                            &start_offset,
                            text_area.width as usize,
                            convert_style(&theme.text),
                        );
                    }
                }

                let mut current_width: usize = 0;

                let mut render_text = |text: &str, theme, current_width: usize| -> usize {
                    buf.set_stringn(
                        text_area.x + current_width as u16,
                        text_area.y + i as u16,
                        text,
                        text_area.width as usize,
                        theme,
                    );
                    text.width()
                };

                let render_whitespace = |col: usize, text_end_col: usize| -> bool {
                    match self.config.render_whitespace {
                        config::RenderWhitespace::All => true,
                        config::RenderWhitespace::None => false,
                        config::RenderWhitespace::Trailing => col >= text_end_col,
                    }
                };

                let text = line.text.line_without_line_ending(0);
                for grapheme in text.grapehemes() {
                    if current_width >= text_area.width as usize {
                        break;
                    }

                    if grapheme.starts_width_char('\t') {
                        let tab_width = tab_width_at(current_width, TAB_WIDTH);
                        if render_whitespace(current_width, line.text_end_col) {
                            dim_cells.push((current_width, i));
                            grapheme_buffer.push('→');
                        } else {
                            grapheme_buffer.push(' ');
                        }
                        grapheme_buffer
                            .extend(std::iter::repeat(" ").take(tab_width.saturating_sub(1)));
                        render_text(
                            &grapheme_buffer,
                            convert_style(&theme.dim_text),
                            current_width,
                        );
                        grapheme_buffer.clear();
                        current_width += tab_width;
                        continue;
                    }

                    if grapheme.chars().any(|ch| ch.is_ascii_control()) {
                        render_text("�", convert_style(&theme.text), current_width);
                    } else if grapheme.is_whitespace() {
                        let width = grapheme.width(current_width);
                        if render_whitespace(current_width, line.text_end_col) {
                            dim_cells.push((current_width, i));
                            current_width +=
                                render_text("·", convert_style(&theme.dim_text), current_width);
                        } else {
                            current_width +=
                                render_text(" ", convert_style(&theme.text), current_width);
                        }
                        for _ in 0..width.saturating_sub(1) {
                            current_width +=
                                render_text(" ", convert_style(&theme.dim_text), current_width);
                        }
                    } else {
                        for ch in grapheme.chars() {
                            grapheme_buffer.push(ch);
                        }
                        render_text(&grapheme_buffer, convert_style(&theme.text), current_width);
                        grapheme_buffer.clear();
                        current_width += grapheme.width(current_width);
                    }
                }
            }
            let mut ruler_cells = Vec::new();
            if !view.lines.is_empty() && config.show_indent_rulers {
                // TODO fix empty line gaps in blocks using tree-sitter indent queries
                let mut last_text_start_col = 0;
                'outer: for line in text_area.top()..text_area.bottom() {
                    for col in text_area.left()..text_area.right() {
                        let Some(view_line) = view.lines.get((line - text_area.y) as usize) else {
                            break 'outer;
                        };
                        let text_start = if view_line.text.is_whitespace() {
                            last_text_start_col
                        } else {
                            view_line.text_start_col
                        };
                        last_text_start_col = text_start;

                        let visual_text_start = text_start + text_area.x as usize;
                        if col as usize + buffer.col_pos() > visual_text_start || text_start == 0 {
                            break;
                        }

                        let cell = buf.cell_mut((col, line)).unwrap();
                        if !RopeSlice::from(cell.symbol()).is_whitespace()
                            || (col as usize - text_area.left() as usize + buffer.col_pos())
                                % buffer.indent.width()
                                != 0
                        {
                            continue;
                        }

                        ruler_cells.push((col, line));
                    }
                }
            }

            let mut cursor_rect = None;
            if has_focus {
                'exit: {
                    if let Some((_, row)) = buffer.cursor_view_pos(text_area.height.into()) {
                        let column =
                            buffer.cursor_grapheme_column() as i64 - buffer.col_pos() as i64;

                        if view.lines.get(row).is_some()
                            && column < text_area.width as i64
                            && column >= 0
                        {
                            cursor_rect = Some(Rect {
                                x: text_area.x + column as u16,
                                y: text_area.y + row as u16,
                                width: 1,
                                height: 1,
                            });
                            break 'exit;
                        }
                    }
                }
            }

            let range = buffer.view_range();
            let col_pos = buffer.col_pos();
            let line_pos = buffer.line_pos();
            let mut highlights = Vec::new();
            let mut syntax_rope = None;
            {
                if let Some(syntax) = buffer.get_syntax() {
                    if let Some((rope, events)) = &*syntax.get_highlight_events() {
                        syntax_rope = Some(rope.clone());
                        let mut highlight_stack: Vec<Highlight> = Vec::new();
                        for event in events {
                            match event {
                                HighlightEvent::Source { start, end } => {
                                    if range.contains(start) || range.contains(end) {
                                        let mut style = convert_style(&theme.text);
                                        if let Some(highlight) = highlight_stack.last() {
                                            if let Some(name) = highlight
                                                .query
                                                .capture_names()
                                                .get(highlight.capture_index)
                                            {
                                                style = convert_style(&self.theme.get_syntax(name));
                                            }
                                        }
                                        highlights.push((*start, *end, style));
                                    }
                                }
                                HighlightEvent::HighlightStart(h) => highlight_stack.push(*h),
                                HighlightEvent::HighlightEnd => drop(highlight_stack.pop()),
                            }
                        }
                    }
                }
            }

            // Apply highlight
            if let Some(rope) = syntax_rope {
                let highlights: Vec<_> = highlights
                    .par_iter()
                    .map(|(start, end, style)| {
                        let start_point = rope.byte_to_point((*start).min(rope.len_bytes()));
                        let end_point = rope.byte_to_point((*end).min(rope.len_bytes()));

                        let start_x = start_point
                            .column
                            .saturating_sub(col_pos)
                            .clamp(0, text_area.width.into());
                        let start_y = start_point
                            .line
                            .saturating_sub(line_pos)
                            .clamp(0, text_area.height.into());

                        let end_x = end_point
                            .column
                            .saturating_sub(col_pos)
                            .clamp(0, text_area.width.into());
                        let end_y = end_point
                            .line
                            .saturating_sub(line_pos)
                            .clamp(0, text_area.height.into());

                        // FIXME This should not be needed
                        let end_x = end_x.max(start_x);

                        let highlight_area = Rect {
                            x: start_x as u16 + text_area.x,
                            y: start_y as u16 + text_area.y,
                            width: (end_x as u16 - start_x as u16),
                            height: (end_y as u16 - start_y as u16) + 1,
                        };

                        (highlight_area, style)
                    })
                    .collect();

                for (area, style) in highlights {
                    buf.set_style(area, *style);
                }
            }

            // Stupid hack to fix tree sitter writing over rendered whitespace
            for (col, line) in dim_cells {
                let cell_area = Rect {
                    x: col as u16 + text_area.x,
                    y: line as u16 + text_area.y,
                    width: 1,
                    height: 1,
                };
                buf.set_style(cell_area, convert_style(&theme.dim_text));
            }

            for ruler in config.rulers.iter().copied() {
                let real_col =
                    ruler as i64 - buffer.col_pos() as i64 + area.x as i64 + left_offset as i64 + 1;
                if (area.left().into()..area.right().into()).contains(&real_col) {
                    for y in area.top()..(area.bottom() - 1) {
                        let cell = buf.cell_mut((real_col as u16, y)).unwrap();
                        if cell.symbol().chars().all(|ch| ch.is_whitespace()) {
                            cell.set_symbol("│");
                            cell.set_style(convert_style(&theme.ruler));
                        }
                    }
                }
            }

            for (col, line) in ruler_cells {
                let cell = buf.cell_mut((col, line)).unwrap();
                cell.set_char('│');
                cell.set_style(convert_style(&self.theme.ruler));
            }

            if let Some(rect) = cursor_rect {
                buf.set_style(
                    rect,
                    convert_style(&theme.text).add_modifier(tui::style::Modifier::REVERSED),
                );
            }

            let matches = buffer.get_searcher().map(|searcher| searcher.get_matches());
            if let Some(matches) = matches {
                let matches = matches.lock().unwrap();
                let matches = &*matches.0;

                for SearchMatch { start, end, .. } in matches {
                    if start.line >= buffer.line_pos()
                        && end.line + 2 < buffer.line_pos() + buffer.get_view_lines()
                    {
                        let highlight_area = Rect {
                            x: (start.column + text_area.left() as usize - buffer.col_pos()) as u16,
                            y: (start.line + text_area.top() as usize - buffer.line_pos()) as u16,
                            width: (end.column - start.column) as u16,
                            height: (end.line - start.line + 1) as u16,
                        };

                        buf.set_style(
                            highlight_area.clamp_within(text_area),
                            convert_style(&self.theme.search_match),
                        );
                    }
                }
            }

            if let Some(bg) = convert_style(&theme.selection).bg {
                let Selection { start, end } = buffer.get_view_selection();
                let line_pos = buffer.line_pos();

                for y in 0..text_area.height {
                    let line_idx = y as usize + line_pos;
                    let width = if line_idx >= buffer.rope().len_lines() {
                        0
                    } else {
                        buffer.rope().line_without_line_ending(line_idx).width(0)
                    };
                    for x in 0..text_area.width {
                        if x as usize > width {
                            break;
                        }
                        let current = Point {
                            column: x.into(),
                            line: y.into(),
                        };
                        if current >= start && current < end {
                            let cell = buf.cell_mut((x + text_area.left(), y + text_area.top())).unwrap();
                            cell.bg = bg;
                        }
                    }
                }
            }

            if info_line {
                let info_line = InfoLine {
                    theme,
                    config: &self.config.info_line,
                    focus: self.has_focus,
                    encoding: buffer.encoding,
                    name: buffer.name().to_string(),
                    line: buffer.cursor_pos().1 + 1,
                    column: buffer.cursor_grapheme_column() + 1,
                    dirty: buffer.is_dirty(),
                    branch: &branch,
                    language: buffer.language_name(),
                    size: buffer.rope().len_bytes(),
                    read_only: buffer.read_only_file,
                    spinner,
                };
                info_line.render(
                    Rect::new(area.x, text_area.height + text_area.y, area.width, 1),
                    buf,
                );
            }
        }
    }
}
