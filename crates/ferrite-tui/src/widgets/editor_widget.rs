use std::ops::Add;

use ferrite_core::{
    buffer::{Buffer, ViewId, cursor::Selection, search::SearchMatch},
    config::{
        self,
        editor::{CursorType, Editor, LineNumber},
    },
    language::syntax::{Highlight, HighlightEvent},
    theme::EditorTheme,
};
use ferrite_ctx::ArenaString;
use ferrite_utility::{
    graphemes::{RopeGraphemeExt, TAB_WIDTH, tab_width_at},
    point::Point,
};
use rayon::{
    iter::IndexedParallelIterator,
    prelude::{IntoParallelRefIterator, ParallelIterator},
};
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
    const BEFORE_PADDING: usize = 0;
    const AFTER_PADDING: usize = 2;
    let left_offset = BEFORE_PADDING + AFTER_PADDING + line_number_max_width;
    (line_number_max_width, left_offset)
}

fn intersects(start1: usize, end1: usize, start2: usize, end2: usize) -> bool {
    !(start1 > end2 || end1 < start2)
}

pub struct EditorWidget<'a> {
    theme: &'a EditorTheme,
    config: &'a Editor,
    view_id: ViewId,
    has_focus: bool,
    branch: Option<String>,
    spinner: Option<char>,
    pub line_nr: bool,
    pub info_line: bool,
    pub draw_rulers: bool,
}

impl<'a> EditorWidget<'a> {
    pub fn new(
        theme: &'a EditorTheme,
        config: &'a Editor,
        view_id: ViewId,
        has_focus: bool,
        branch: Option<String>,
        spinner: Option<char>,
    ) -> Self {
        Self {
            theme,
            config,
            view_id,
            has_focus,
            branch,
            spinner,
            line_nr: true,
            info_line: true,
            draw_rulers: true,
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

        let arena = ferrite_ctx::Ctx::arena();

        let Self {
            theme,
            config,
            view_id,
            has_focus,
            branch,
            spinner,
            line_nr,
            info_line,
            draw_rulers,
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

        buffer.set_view_lines(view_id, text_area.height.into());

        buffer.set_view_columns(
            view_id,
            (text_area.width as usize).saturating_sub(left_offset),
        );
        buf.set_style(area, convert_style(&theme.background));

        if line_nr {
            buf.set_style(
                Rect {
                    x: area.left(),
                    y: area.top(),
                    width: (line_number_max_width as u16).min(area.width),
                    height: area.height,
                },
                convert_style(&theme.line_nr),
            );
        }

        let cursor_line_number = buffer.cursor_line_idx(view_id, 0) + 1;

        // We have to overwrite all rendered whitespace with the correct color
        let mut dim_cells = Vec::new();
        let mut grapheme_buffer = ArenaString::with_capacity_in(100, &arena);
        let view = buffer.get_buffer_view(view_id);
        {
            profiling::scope!("render text");
            for (i, (line, line_number)) in view
                .lines
                .iter()
                .zip((buffer.line_pos(view_id) + 1)..=buffer.line_pos(view_id) + buffer.len_lines())
                .enumerate()
            {
                if line_nr {
                    let is_current_line = line_number == cursor_line_number;
                    let line_number =
                        if (config.line_number == LineNumber::Absolute) || is_current_line {
                            line_number
                        } else {
                            (line_number as i64 - cursor_line_number as i64).unsigned_abs() as usize
                        };
                    let line_number_str = line_number.to_string();
                    let line_number_str = ferrite_ctx::format!(in &arena,
                        "{}{}",
                        // TODO: rm temp alloc
                        " ".repeat(line_number_max_width - line_number_str.len()),
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

                    // TODO: rm temp alloc
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
                        text_area.width as usize - current_width,
                        theme,
                    );
                    text.width()
                };

                let render_whitespace = |col: usize, text_end_col: usize| -> bool {
                    match self.config.render_whitespace {
                        config::editor::RenderWhitespace::All => true,
                        config::editor::RenderWhitespace::None => false,
                        config::editor::RenderWhitespace::Trailing => col >= text_end_col,
                    }
                };

                let text = line.text.line_without_line_ending(0);
                for grapheme in text.grapehemes() {
                    if current_width >= text_area.width as usize {
                        break;
                    }

                    if grapheme.starts_with_char('\t') {
                        let tab_width = tab_width_at(current_width, TAB_WIDTH);
                        if render_whitespace(current_width, line.text_end_col) {
                            dim_cells.push((current_width, i));
                            grapheme_buffer.push('→');
                        } else {
                            grapheme_buffer.push(' ');
                        }
                        grapheme_buffer
                            .extend(std::iter::repeat_n(" ", tab_width.saturating_sub(1)));
                        current_width += render_text(
                            &grapheme_buffer,
                            convert_style(&theme.dim_text),
                            current_width,
                        );
                        grapheme_buffer.clear();
                        continue;
                    }

                    if grapheme.chars().any(|ch| ch.is_ascii_control()) {
                        current_width +=
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
                        current_width += render_text(
                            &grapheme_buffer,
                            convert_style(&theme.text),
                            current_width,
                        );
                        grapheme_buffer.clear();
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
                        if col as usize + buffer.col_pos(view_id) > visual_text_start
                            || text_start == 0
                        {
                            break;
                        }

                        let cell = buf.cell_mut((col, line)).unwrap();
                        if !RopeSlice::from(cell.symbol()).is_whitespace()
                            || (col as usize - text_area.left() as usize + buffer.col_pos(view_id))
                                % buffer.indent.width()
                                != 0
                        {
                            continue;
                        }

                        ruler_cells.push((col, line));
                    }
                }
            }

            let mut draw_cursor_line = true;

            let cursor_view_pos =
                buffer.cursors_view_pos(view_id, text_area.width.into(), text_area.height.into());

            if cursor_view_pos.len() > 1 {
                draw_cursor_line = false;
            }

            let mut cursor_rects = Vec::new();
            if has_focus {
                for (column, row) in cursor_view_pos {
                    cursor_rects.push(Rect {
                        x: text_area.x + column as u16,
                        y: text_area.y + row as u16,
                        width: 1,
                        height: 1,
                    });
                }
            }

            let range = buffer.view_range(view_id);
            let col_pos = buffer.col_pos(view_id);
            let line_pos = buffer.line_pos(view_id);
            let mut highlights = Vec::new();
            let mut syntax_rope = None;
            {
                // TODO do this async on syntax thread
                profiling::scope!("collect syntax events");
                if let Some(syntax) = buffer.get_syntax() {
                    if let Some((rope, events)) = &*syntax.get_highlight_events() {
                        syntax_rope = Some(rope.clone());
                        let mut highlight_stack: Vec<Highlight> = Vec::new();
                        for event in events {
                            match event {
                                HighlightEvent::Source { start, end } => {
                                    if intersects(*start, *end, range.start, range.end) {
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
                profiling::scope!("apply highlights");
                let highlights: Vec<_> = {
                    profiling::scope!("take highlight events");
                    highlights
                        .par_iter()
                        .take(10000)
                        .map(|(start, end, style)| {
                            let start_point = rope.byte_to_point((*start).min(rope.len_bytes()));
                            let end_point = rope.byte_to_point((*end).min(rope.len_bytes()));

                            (start_point, end_point, style)
                        })
                        .collect()
                };

                for (start_point, end_point, style) in highlights {
                    let diff = end_point.line - start_point.line;
                    for i in 0..(diff + 1) {
                        let start_y = start_point
                            .line
                            .saturating_sub(line_pos)
                            .add(i)
                            .clamp(0, text_area.height.into());
                        let end_y = end_point
                            .line
                            .saturating_sub(line_pos)
                            .add(i)
                            .clamp(0, text_area.height.into());

                        let first = i == 0;
                        let last = i == diff;

                        let start_view_col = start_point.column.saturating_sub(col_pos);
                        let start_x = if first {
                            start_view_col.clamp(0, text_area.width.into())
                        } else {
                            0
                        };

                        let end_view_col = end_point.column.saturating_sub(col_pos);
                        let end_x = if last {
                            end_view_col.clamp(0, text_area.width.into())
                        } else {
                            text_area.width.into()
                        };

                        // FIXME This should not be needed
                        let end_x = end_x.max(start_x);

                        let highlight_area = Rect {
                            x: start_x as u16 + text_area.x,
                            y: start_y as u16 + text_area.y,
                            width: (end_x as u16 - start_x as u16),
                            height: (end_y as u16 - start_y as u16) + 1,
                        };
                        buf.set_style(highlight_area, *style);
                    }
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

            if draw_rulers {
                for ruler in config.rulers.iter().copied() {
                    let real_col = ruler as i64 - buffer.col_pos(view_id) as i64
                        + area.x as i64
                        + left_offset as i64
                        + 1;
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
            }

            for (col, line) in ruler_cells {
                let cell = buf.cell_mut((col, line)).unwrap();
                cell.set_char('│');
                cell.set_style(convert_style(&self.theme.ruler));
            }

            for rect in cursor_rects {
                match self.config.gui.cursor_type {
                    CursorType::Block => {
                        buf.set_style(
                            rect,
                            convert_style(&theme.text).add_modifier(tui::style::Modifier::REVERSED),
                        );
                    }
                    CursorType::Line => {
                        buf.set_style(
                            rect,
                            tui::style::Style::default()
                                .add_modifier(tui::style::Modifier::SLOW_BLINK),
                        );
                    }
                }
            }

            draw_cursor_line &= !buffer.views[view_id]
                .cursors
                .iter()
                .any(|c| c.has_selection());

            if self.config.highlight_cursor_line && draw_cursor_line && has_focus {
                let line_idx = buffer.cursor_line_idx(view_id, 0);
                let start_line = buffer.views[view_id].line_pos_floored();
                let end_line =
                    buffer.views[view_id].line_pos_floored() + buffer.get_view_lines(view_id);

                if line_idx > start_line && line_idx < end_line {
                    let cursor_line_area = Rect::new(
                        text_area.x,
                        text_area.y + (line_idx - start_line) as u16,
                        text_area.width,
                        1,
                    );
                    buf.set_style(cursor_line_area, convert_style(&theme.cursorline));
                }
            }

            let matches = buffer.views[view_id]
                .searcher
                .as_ref()
                .map(|searcher| searcher.get_matches());
            if let Some(matches) = matches {
                let matches = matches.lock().unwrap();
                let matches = &*matches.0;

                for SearchMatch { start, end, .. } in matches {
                    if start.line >= buffer.line_pos(view_id)
                        && end.line < buffer.line_pos(view_id) + buffer.get_view_lines(view_id)
                    {
                        let highlight_area = Rect {
                            x: (start.column + text_area.left() as usize - buffer.col_pos(view_id))
                                as u16,
                            y: (start.line + text_area.top() as usize - buffer.line_pos(view_id))
                                as u16,
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
                profiling::scope!("draw selections");
                for Selection { start, end } in buffer.get_view_selection(view_id) {
                    let line_pos = buffer.line_pos(view_id);

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
                                let cell = buf
                                    .cell_mut((x + text_area.left(), y + text_area.top()))
                                    .unwrap();
                                cell.bg = bg;
                            }
                        }
                    }
                }
            }

            {
                let start_line = buffer.views[view_id].line_pos_floored();
                let end_line = start_line + buffer.get_view_lines(view_id);
                let conflicts = buffer.conflicts.lock().unwrap();
                for (start, middle, end) in &*conflicts {
                    if intersects(*start, *end, start_line, end_line) {
                        let area_start = (text_area.y as i64 + *start as i64 - start_line as i64)
                            .clamp(text_area.top() as i64, text_area.bottom() as i64);
                        let area_middle = (text_area.y as i64 + *middle as i64 - start_line as i64)
                            .clamp(text_area.top() as i64, text_area.bottom() as i64);
                        let area_end = (text_area.y as i64 + *end as i64 - start_line as i64)
                            .clamp(text_area.top() as i64, text_area.bottom() as i64);
                        let first_area = Rect::new(
                            text_area.x,
                            area_start as u16,
                            text_area.width,
                            (area_middle - area_start) as u16,
                        );
                        let second_area = Rect::new(
                            text_area.x,
                            area_middle as u16,
                            text_area.width,
                            (area_end - area_middle) as u16,
                        );
                        buf.set_style(first_area, convert_style(&theme.cursorline));
                        buf.set_style(second_area, convert_style(&theme.cursorline));
                    }
                }
            }

            if buffer.views[view_id].completer.visible
                && !buffer.views[view_id].completer.matching_words.is_empty()
            {
                let cursor_view_pos = buffer.cursor_view_pos(
                    view_id,
                    0,
                    text_area.width.into(),
                    text_area.height.into(),
                );
                if let Some((column, line)) = cursor_view_pos {
                    let longest: usize = buffer.views[view_id]
                        .completer
                        .matching_words
                        .iter()
                        .map(|w| w.width() + 2)
                        .max()
                        .unwrap_or_default()
                        .min(40);

                    for (i, word) in buffer.views[view_id]
                        .completer
                        .matching_words
                        .iter()
                        .enumerate()
                    {
                        let rect = Rect::new(
                            (column + text_area.x as usize) as u16,
                            (line + text_area.y as usize + i + 1) as u16,
                            longest as u16,
                            1,
                        );
                        let rect = rect.intersection(text_area);
                        if rect.area() > 0 {
                            Clear.render(rect, buf);
                            buf.set_stringn(
                                rect.x + 1,
                                rect.y,
                                word,
                                rect.width.into(),
                                tui::style::Style::default(),
                            );
                            let style = if i == buffer.views[view_id].completer.index {
                                convert_style(&theme.completer_selected)
                            } else {
                                convert_style(&theme.completer)
                            };
                            buf.set_style(rect, style);
                        }
                    }
                }
            }

            if info_line {
                let path = if let Some(path) = buffer.file() {
                    path.to_string_lossy().into()
                } else {
                    buffer.name().to_string()
                };

                let info_line = InfoLine {
                    theme,
                    config: &self.config.info_line,
                    focus: self.has_focus,
                    encoding: buffer.encoding,
                    path,
                    line: buffer.cursor_line_idx(view_id, 0) + 1,
                    column: buffer.cursor_grapheme_column(view_id, 0) + 1,
                    dirty: buffer.is_dirty(),
                    branch: &branch,
                    language: buffer.language_name().into(),
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
