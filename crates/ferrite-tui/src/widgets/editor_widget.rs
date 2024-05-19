use ferrite_core::{
    buffer::{search::SearchMatch, Buffer, Selection},
    config::Config,
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
    widgets::{StatefulWidget, Widget},
};

use super::info_line::InfoLine;
use crate::{glue::convert_style, rect_ext::RectExt};

pub fn lines_to_left_offset(lines: usize) -> (usize, usize) {
    let line_number_max_width = lines.to_string().len().max(4);
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

        let Self {
            theme,
            config,
            has_focus,
            branch,
            spinner,
        } = self;

        let (line_number_max_width, left_offset) = lines_to_left_offset(buffer.len_lines());

        let text_area = Rect {
            x: area.x + left_offset as u16,
            y: area.y,
            width: area.width.saturating_sub(left_offset as u16),
            height: area.height - 1,
        };

        buffer.set_view_lines(text_area.height.into());

        buffer.set_view_columns((text_area.width as usize).saturating_sub(left_offset));
        buf.set_style(area, convert_style(&theme.background));

        buf.set_style(
            Rect {
                x: area.left(),
                y: area.top(),
                width: (line_number_max_width as u16 + 2).min(area.width),
                height: area.height,
            },
            convert_style(&theme.line_nr),
        );

        let current_line_number = buffer.cursor_line_idx() + 1;

        let view = buffer.get_buffer_view();
        {
            let mut line_buffer = String::with_capacity(text_area.width.into());

            for (i, (line, line_number)) in view
                .lines
                .iter()
                .zip((buffer.line_pos() + 1)..=buffer.line_pos() + buffer.len_lines())
                .enumerate()
            {
                let line_number_str = line_number.to_string();
                let line_number_str = format!(
                    " {}{} ",
                    " ".repeat(line_number_max_width - line_number_str.len()),
                    line_number
                );
                let line_nr_theme = if line_number == current_line_number {
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

                if text_area.width > 0 {
                    buf.set_stringn(
                        text_area.x,
                        text_area.y + i as u16,
                        &line_buffer,
                        text_area.width as usize,
                        convert_style(&theme.text),
                    );
                }

                line_buffer.clear();
            }

            let mut ruler_cells = Vec::new();
            if !view.lines.is_empty() && config.show_indent_rulers {
                // TODO fix empty line gaps in blocks using tree-sitter indent queries
                'outer: for line in text_area.top()..text_area.bottom() {
                    for col in text_area.left()..text_area.right() {
                        let Some(view_line) = view.lines.get((line - text_area.y) as usize) else {
                            break 'outer;
                        };
                        let text_start = if view_line.text.is_whitespace() {
                            0
                        } else {
                            view_line.text_start_col
                        };
                        let visual_text_start = text_start + text_area.x as usize;
                        if col as usize + buffer.col_pos() > visual_text_start || text_start == 0 {
                            break;
                        }

                        let cell = buf.get_mut(col, line);
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
                        let mut highlight: Option<Highlight> = None;
                        for event in events {
                            match event {
                                HighlightEvent::Source { start, end } => {
                                    if range.contains(start) || range.contains(end) {
                                        let mut style = convert_style(&theme.text);
                                        if let Some(highlight) = &highlight {
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
                                HighlightEvent::HighlightStart(h) => highlight = Some(*h),
                                HighlightEvent::HighlightEnd => highlight = None,
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

            for ruler in config.rulers.iter().copied() {
                let real_col =
                    ruler as i64 - buffer.col_pos() as i64 + area.x as i64 + left_offset as i64 + 1;
                if (area.left().into()..area.right().into()).contains(&real_col) {
                    for y in area.top()..(area.bottom() - 1) {
                        let cell = buf.get_mut(real_col as u16, y);
                        if cell.symbol().chars().all(|ch| ch.is_whitespace()) {
                            cell.set_symbol("│");
                            cell.set_style(convert_style(&theme.ruler));
                        }
                    }
                }
            }

            for (col, line) in ruler_cells {
                let cell = buf.get_mut(col, line);
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
                let matches = &*matches;

                for SearchMatch { start, end } in matches {
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
                            let cell = buf.get_mut(x + text_area.left(), y + text_area.top());
                            cell.bg = bg;
                        }
                    }
                }
            }

            let info_line = InfoLine {
                theme,
                config: &self.config.info_line,
                focus: self.has_focus,
                encoding: buffer.encoding,
                file: buffer.file(),
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
