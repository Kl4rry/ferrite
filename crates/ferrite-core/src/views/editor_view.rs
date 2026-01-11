use std::{ops::Add, sync::Arc};

use ferrite_ctx::{ArenaString, ArenaVec};
use ferrite_geom::{
    point::Point,
    rect::{Rect, Vec2},
};
use ferrite_runtime::{
    Bounds, MouseButton, MouseInterction, MouseInterctionKind, View,
    input::keycode::KeyModifiers,
    painter::{CursorIcon, Rounding},
};
use ferrite_style::Style;
use ferrite_utility::{
    graphemes::{RopeGraphemeExt, TAB_WIDTH, tab_width_at},
    tui_buf_ext::TuiBufExt,
};
use rayon::{
    iter::IndexedParallelIterator,
    prelude::{IntoParallelRefIterator, ParallelIterator},
};
use ropey::RopeSlice;
use tui::{prelude::Widget as _, widgets::Clear};
use unicode_width::UnicodeWidthStr;

use super::info_line_view::InfoLineView;
use crate::{
    buffer::{Buffer, ViewDrag, ViewId, cursor::Selection, search::SearchMatch},
    cmd::Cmd,
    config::{
        self,
        editor::{CursorType, Editor, LineNumber},
    },
    language::syntax::{Highlight, HighlightEvent},
    theme::EditorTheme,
};

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

pub struct EditorView {
    view_id: ViewId,
    config: Arc<Editor>,
    theme: Arc<EditorTheme>,
    has_focus: bool,
    branch: Option<String>,
    spinner: Option<char>,
    pub line_nr: bool,
    pub info_line: bool,
    pub draw_rulers: bool,
    pub ceil_surface_size: bool,
    pub scrollbar: bool,
}

impl EditorView {
    pub fn new(
        view_id: ViewId,
        config: Arc<Editor>,
        theme: Arc<EditorTheme>,
        has_focus: bool,
        branch: Option<String>,
        spinner: Option<char>,
    ) -> Self {
        Self {
            view_id,
            config,
            theme,
            has_focus,
            branch,
            spinner,
            line_nr: true,
            info_line: true,
            draw_rulers: true,
            ceil_surface_size: false,
            scrollbar: false,
        }
    }

    pub fn set_ceil_surface_size(mut self, ceil_surface_size: bool) -> Self {
        self.ceil_surface_size = ceil_surface_size;
        self
    }

    pub fn set_scrollbar(mut self, scrollbar: bool) -> Self {
        self.scrollbar = scrollbar;
        self
    }
}

impl View<Buffer> for EditorView {
    fn handle_mouse(
        &self,
        buffer: &mut Buffer,
        bounds: Bounds,
        mouse_interaction: MouseInterction,
    ) -> bool {
        let cell_position = mouse_interaction.cell_position(bounds.view_bounds().position());
        let (_, left_offset) = lines_to_left_offset(buffer.len_lines());
        match mouse_interaction.kind {
            MouseInterctionKind::Click(clicks) if mouse_interaction.button == MouseButton::Left => {
                if get_scrollbar_track(bounds).contains(mouse_interaction.position) {
                    // TODO: handle clicks on scrollbar
                } else {
                    buffer.handle_click(
                        self.view_id,
                        clicks,
                        mouse_interaction.modifiers == KeyModifiers::ALT,
                        cell_position.x.saturating_sub(left_offset),
                        cell_position.y,
                    );
                }
            }
            MouseInterctionKind::Click(_) if mouse_interaction.button == MouseButton::Middle => {
                let cmd = Cmd::PastePrimary {
                    column: cell_position.x.saturating_sub(left_offset),
                    line: cell_position.y,
                };
                // NOTE: Should never panic
                buffer.handle_input(self.view_id, cmd).unwrap();
            }
            MouseInterctionKind::Drag {
                drag_start: _,
                last_pos,
            } if mouse_interaction.button == MouseButton::Left => {
                match buffer.views[self.view_id].drag {
                    ViewDrag::Text => {
                        let cmd = Cmd::DragCell {
                            column: cell_position.x.saturating_sub(left_offset),
                            line: cell_position.y,
                        };
                        // NOTE: Should never panic
                        buffer.handle_input(self.view_id, cmd).unwrap();
                    }
                    ViewDrag::Scrollbar => {
                        let moved_distance = last_pos.y - mouse_interaction.position.y;
                        let content_height = buffer.get_view_lines(self.view_id);
                        let len_lines = (buffer.len_lines() + content_height) - 1;
                        let scrollbar_ratio = content_height as f32 / len_lines as f32;
                        let line_distance =
                            (moved_distance / mouse_interaction.cell_size.y) / scrollbar_ratio;

                        let cmd = Cmd::VerticalScroll {
                            distance: -line_distance as f64,
                        };
                        // NOTE: Should never panic
                        buffer.handle_input(self.view_id, cmd).unwrap();
                    }
                    ViewDrag::None => {
                        if get_scrollbar_track(bounds).contains(mouse_interaction.position) {
                            buffer.views[self.view_id].drag = ViewDrag::Scrollbar;

                            // NOTE: everything in this branch is copy pasted from the scrollbar branch
                            let moved_distance = last_pos.y - mouse_interaction.position.y;
                            let content_height = buffer.get_view_lines(self.view_id);
                            let len_lines = (buffer.len_lines() + content_height) - 1;
                            let scrollbar_ratio = content_height as f32 / len_lines as f32;
                            let line_distance =
                                (moved_distance / mouse_interaction.cell_size.y) / scrollbar_ratio;

                            let cmd = Cmd::VerticalScroll {
                                distance: -line_distance as f64,
                            };
                            // NOTE: Should never panic
                            buffer.handle_input(self.view_id, cmd).unwrap();
                        } else {
                            buffer.views[self.view_id].drag = ViewDrag::Text;
                            let cmd = Cmd::DragCell {
                                column: cell_position.x.saturating_sub(left_offset),
                                line: cell_position.y,
                            };
                            // NOTE: Should never panic
                            buffer.handle_input(self.view_id, cmd).unwrap();
                        }
                    }
                }
            }
            MouseInterctionKind::DragStop if mouse_interaction.button == MouseButton::Left => {
                buffer.copy_selection_to_primary(self.view_id);
                buffer.views[self.view_id].drag = ViewDrag::None;
            }
            _ => (),
        }
        true
    }

    fn render(
        &self,
        buffer: &mut Buffer,
        mut bounds: Bounds,
        painter: &mut ferrite_runtime::Painter,
    ) {
        let Self {
            view_id,
            config,
            theme,
            has_focus,
            branch,
            spinner,
            line_nr,
            info_line,
            draw_rulers,
            ceil_surface_size,
            scrollbar,
        } = self;
        let view_id = *view_id;
        let has_focus = *has_focus;
        let line_nr = *line_nr;
        let info_line = *info_line;
        let draw_rulers = *draw_rulers;

        let unique_id = buffer.views[view_id].unique_id();
        let rounding = if *ceil_surface_size && bounds.cell_size() != Vec2::new(1.0, 1.0) {
            Rounding::Ceil
        } else {
            Rounding::Round
        };
        bounds.rounding = rounding;
        let layer = painter.create_layer(unique_id, bounds);
        let mut layer = layer.lock().unwrap();
        let has_2d_painter = layer.painter2d.is_some();
        let view_bounds = bounds.view_bounds();
        let area: Rect = layer.buf.area.into();
        let buf = &mut layer.buf;
        if area.area() == 0 {
            return;
        }

        let arena = ferrite_ctx::Ctx::arena();

        let (line_number_max_width, left_offset) =
            if line_nr && config.line_number != LineNumber::None {
                lines_to_left_offset(buffer.len_lines())
            } else {
                (0, 0)
            };

        let text_area = Rect {
            x: area.x + left_offset,
            y: area.y,
            width: area.width.saturating_sub(left_offset),
            height: area.height - info_line as usize,
        };

        {
            let left_offset = (left_offset as f32 * bounds.cell_size().x) as usize;
            let scrollbar_width = bounds.cell_size().x as usize;
            let info_line_height = bounds.cell_size().y as usize;

            painter.push_cursor_zone(CursorIcon::Text, view_bounds);
            painter.push_cursor_zone(
                CursorIcon::Default,
                Rect::new(
                    view_bounds.x,
                    view_bounds.y,
                    left_offset,
                    view_bounds.height,
                ),
            );
            if info_line {
                painter.push_cursor_zone(
                    CursorIcon::Default,
                    Rect::new(
                        view_bounds.x,
                        (view_bounds.y + view_bounds.height).saturating_sub(info_line_height),
                        view_bounds.width,
                        info_line_height,
                    ),
                );
            }
            if *scrollbar {
                painter.push_cursor_zone(
                    CursorIcon::Pointer,
                    Rect::new(
                        (view_bounds.x + view_bounds.width).saturating_sub(scrollbar_width),
                        view_bounds.y,
                        scrollbar_width,
                        view_bounds.height,
                    ),
                );
            }
        }

        buffer.set_view_lines(view_id, text_area.height);

        buffer.set_view_columns(view_id, text_area.width.saturating_sub(left_offset));
        buf.set_style(area.into(), theme.background);

        if line_nr {
            buf.set_style(
                Rect {
                    x: area.left(),
                    y: area.top(),
                    width: (line_number_max_width).min(area.width),
                    height: area.height,
                }
                .into(),
                theme.line_nr,
            );
        }

        let cursor_line_number = buffer.cursor_line_idx(view_id, 0) + 1;

        // We have to overwrite all rendered whitespace with the correct color
        let mut dim_cells = Vec::new(); // TODO: tmp alloc
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
                profiling::scope!("line");
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
                        theme.current_line_nr
                    } else {
                        theme.line_nr
                    };

                    buf.set_stringn(
                        area.x as u16,
                        (area.y + i) as u16,
                        &line_number_str,
                        area.width,
                        line_nr_theme,
                    );

                    // TODO: rm temp alloc
                    let start_offset = " ".repeat(line.col_start_offset);
                    if text_area.width > 0 {
                        buf.set_stringn(
                            text_area.x as u16,
                            (text_area.y + i) as u16,
                            &start_offset,
                            text_area.width,
                            theme.text,
                        );
                    }
                }

                let mut current_width: usize = 0;

                let mut render_text = |text: &str, style: Style, current_width: usize| -> usize {
                    buf.set_stringn(
                        (text_area.x + current_width) as u16,
                        (text_area.y + i) as u16,
                        text,
                        text_area.width - current_width,
                        style,
                    );
                    text.width()
                };

                let render_whitespace = |col: usize, text_end_col: usize| -> bool {
                    match config.render_whitespace {
                        config::editor::RenderWhitespace::All => true,
                        config::editor::RenderWhitespace::None => false,
                        config::editor::RenderWhitespace::Trailing => col >= text_end_col,
                    }
                };

                let text = line.text.line_without_line_ending(0);
                for grapheme in text.graphemes() {
                    if current_width >= text_area.width {
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
                        current_width +=
                            render_text(&grapheme_buffer, theme.dim_text, current_width);
                        grapheme_buffer.clear();
                        continue;
                    }

                    if grapheme.chars().any(|ch| ch.is_ascii_control()) {
                        current_width += render_text("�", theme.text, current_width);
                    } else if grapheme.is_whitespace() {
                        let width = grapheme.width(current_width);
                        if render_whitespace(current_width, line.text_end_col) {
                            dim_cells.push((current_width, i));
                            current_width += render_text("·", theme.dim_text, current_width);
                        } else {
                            current_width += render_text(" ", theme.text, current_width);
                        }
                        for _ in 0..width.saturating_sub(1) {
                            current_width += render_text(" ", theme.dim_text, current_width);
                        }
                    } else {
                        for ch in grapheme.chars() {
                            grapheme_buffer.push(ch);
                        }
                        current_width += render_text(&grapheme_buffer, theme.text, current_width);
                        grapheme_buffer.clear();
                    }
                }
            }
            let mut ruler_cells = Vec::new();
            if !view.lines.is_empty() && config.show_indent_rulers {
                profiling::scope!("indent rulers");
                // TODO fix empty line gaps in blocks using tree-sitter indent queries
                let mut last_text_start_col = 0;
                'outer: for line in text_area.top()..text_area.bottom() {
                    for col in text_area.left()..text_area.right() {
                        let Some(view_line) = view.lines.get(line - text_area.y) else {
                            break 'outer;
                        };
                        let text_start = if view_line.text.is_whitespace() {
                            last_text_start_col
                        } else {
                            view_line.text_start_col
                        };
                        last_text_start_col = text_start;

                        let visual_text_start = text_start + text_area.x;
                        if col + buffer.col_pos(view_id) > visual_text_start || text_start == 0 {
                            break;
                        }

                        let cell = buf.cell_mut((col as u16, line as u16)).unwrap();
                        if !RopeSlice::from(cell.symbol()).is_whitespace()
                            || (col - text_area.left() + buffer.col_pos(view_id))
                                .is_multiple_of(buffer.indent.width())
                        {
                            continue;
                        }

                        ruler_cells.push((col, line));
                    }
                }
            }

            let mut draw_cursor_line = true;

            let cursor_view_pos =
                buffer.cursors_view_pos(view_id, text_area.width, text_area.height);

            if cursor_view_pos.len() > 1 {
                draw_cursor_line = false;
            }

            let mut cursor_rects = Vec::new();
            if has_focus {
                for (column, row) in cursor_view_pos {
                    cursor_rects.push(Rect {
                        x: text_area.x + column,
                        y: text_area.y + row,
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
                if let Some(syntax) = buffer.get_syntax()
                    && let Some((rope, events)) = &*syntax.get_highlight_events()
                {
                    syntax_rope = Some(rope.clone());
                    let mut highlight_stack: Vec<Highlight> = Vec::new();
                    for event in events {
                        match event {
                            HighlightEvent::Source { start, end } => {
                                if intersects(*start, *end, range.start, range.end) {
                                    let mut style = theme.text;
                                    if let Some(highlight) = highlight_stack.last()
                                        && let Some(name) = highlight
                                            .query
                                            .capture_names()
                                            .get(highlight.capture_index)
                                    {
                                        style = theme.get_syntax(name);
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
                            .clamp(0, text_area.height);
                        let end_y = end_point
                            .line
                            .saturating_sub(line_pos)
                            .add(i)
                            .clamp(0, text_area.height);

                        let first = i == 0;
                        let last = i == diff;

                        let start_view_col = start_point.column.saturating_sub(col_pos);
                        let start_x = if first {
                            start_view_col.clamp(0, text_area.width)
                        } else {
                            0
                        };

                        let end_view_col = end_point.column.saturating_sub(col_pos);
                        let end_x = if last {
                            end_view_col.clamp(0, text_area.width)
                        } else {
                            text_area.width
                        };

                        // FIXME This should not be needed
                        let end_x = end_x.max(start_x);

                        let highlight_area = Rect {
                            x: start_x + text_area.x,
                            y: start_y + text_area.y,
                            width: (end_x - start_x),
                            height: (end_y - start_y) + 1,
                        };
                        buf.set_style(highlight_area.into(), *style);
                    }
                }
            }

            // Stupid hack to fix tree sitter writing over rendered whitespace
            for (col, line) in dim_cells {
                let cell_area = Rect {
                    x: col + text_area.x,
                    y: line + text_area.y,
                    width: 1,
                    height: 1,
                };
                buf.set_style(cell_area.into(), theme.dim_text);
            }

            if draw_rulers {
                for ruler in config.rulers.iter().copied() {
                    let real_col = ruler as i64 - buffer.col_pos(view_id) as i64
                        + area.x as i64
                        + left_offset as i64
                        + 1;
                    if ((area.left() as i64)..(area.right() as i64)).contains(&real_col) {
                        for y in area.top()..(area.bottom() - 1) {
                            let cell = buf.cell_mut((real_col as u16, y as u16)).unwrap();
                            if cell.symbol().chars().all(|ch| ch.is_whitespace()) {
                                cell.set_symbol("│");
                                cell.set_style(theme.ruler);
                            }
                        }
                    }
                }
            }

            for (col, line) in ruler_cells {
                let cell = buf.cell_mut((col as u16, line as u16)).unwrap();
                cell.set_char('│');
                cell.set_style(theme.ruler);
            }

            for rect in cursor_rects {
                match config.gui.cursor_type {
                    CursorType::Line if has_2d_painter => {
                        buf.set_style(
                            rect.into(),
                            tui::style::Style::default()
                                .add_modifier(tui::style::Modifier::SLOW_BLINK),
                        );
                    }
                    _ => {
                        buf.set_style(
                            rect.into(),
                            tui::style::Style::from(theme.text)
                                .add_modifier(tui::style::Modifier::REVERSED),
                        );
                    }
                }
            }

            draw_cursor_line &= !buffer.views[view_id]
                .cursors
                .iter()
                .any(|c| c.has_selection());

            if config.highlight_cursor_line && draw_cursor_line && has_focus {
                let line_idx = buffer.cursor_line_idx(view_id, 0);
                let start_line = buffer.views[view_id].line_pos_floored();
                let end_line =
                    buffer.views[view_id].line_pos_floored() + buffer.get_view_lines(view_id);

                if line_idx > start_line && line_idx < end_line {
                    let cursor_line_area = Rect::new(
                        text_area.x,
                        text_area.y + (line_idx - start_line),
                        text_area.width,
                        1,
                    );
                    buf.set_style(cursor_line_area.into(), theme.cursorline);
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
                            x: (start.column + text_area.left() - buffer.col_pos(view_id)),
                            y: (start.line + text_area.top() - buffer.line_pos(view_id)),
                            width: (end.column - start.column),
                            height: (end.line - start.line + 1),
                        };

                        buf.set_style(
                            highlight_area.clamp_within(text_area).into(),
                            theme.search_match,
                        );
                    }
                }
            }

            'block: {
                profiling::scope!("draw blame");
                let rope = buffer.rope().clone();
                let blame = buffer.blame.get_blame();
                let line_pos = buffer.views[view_id].line_pos_floored();
                let mut hunk_iter = blame.iter();
                let Some(mut current_hunk) = hunk_iter.next() else {
                    break 'block;
                };

                for i in 0..text_area.height {
                    let line_idx = i + line_pos;
                    if line_idx >= rope.len_lines() {
                        break;
                    }
                    while line_idx > current_hunk.start_line + current_hunk.len_lines {
                        match hunk_iter.next() {
                            Some(hunk) => current_hunk = hunk,
                            None => break 'block,
                        }
                    }
                    let width = if line_idx >= rope.len_lines() {
                        0
                    } else {
                        rope.line_without_line_ending(line_idx).width(0)
                    };
                    let Some(x) = (text_area.x + width + 4)
                        .checked_sub(buffer.views[view_id].col_pos_floored())
                    else {
                        continue;
                    };

                    buf.draw_string(
                        x as u16,
                        (i + text_area.y) as u16,
                        &ferrite_ctx::format!(in &arena, "{} {} {} {}", current_hunk.author,
                            "",//humantime::format_rfc3339(current_hunk.author_time),
                            &current_hunk.commit[..8],
                            &current_hunk.summary,
                        ),
                        text_area.into(),
                        theme.dim_text,
                    );
                }
            }

            let mut normalized_conflicts = ArenaVec::new_in(&arena);
            {
                profiling::scope!("draw git conflicts");
                let conflicts = buffer.conflicts.lock().unwrap();
                normalized_conflicts.reserve_exact(conflicts.len());
                let start_line = buffer.views[view_id].line_pos_floored();
                let end_line = start_line + buffer.get_view_lines(view_id);
                let len_lines = buffer.len_lines() as f32 + bounds.grid_bounds().height as f32;
                for (start, middle, end) in &*conflicts {
                    normalized_conflicts.push((
                        *start as f32 / len_lines,
                        *middle as f32 / len_lines,
                        *end as f32 / len_lines,
                    ));
                    if !intersects(*start, *end, start_line, end_line) {
                        continue;
                    }
                    let area_start = (text_area.y as i64 + *start as i64 - start_line as i64)
                        .clamp(text_area.top() as i64, text_area.bottom() as i64);
                    let area_middle = (text_area.y as i64 + *middle as i64 - start_line as i64)
                        .clamp(text_area.top() as i64, text_area.bottom() as i64);
                    let area_end = (text_area.y as i64 + *end as i64 - start_line as i64)
                        .clamp(text_area.top() as i64, text_area.bottom() as i64);
                    let first_area = Rect::new(
                        text_area.x,
                        area_start as usize,
                        text_area.width,
                        (area_middle - area_start) as usize,
                    );
                    let second_area = Rect::new(
                        text_area.x,
                        area_middle as usize,
                        text_area.width,
                        (area_end - area_middle) as usize,
                    );
                    buf.set_style(first_area.into(), theme.conflict_current);
                    buf.set_style(second_area.into(), theme.conflict_incoming);
                }
            }

            if let Some(bg) = tui::style::Style::from(theme.selection).bg {
                profiling::scope!("draw selections");
                for Selection { start, end } in buffer.get_view_selection(view_id) {
                    let line_pos = buffer.line_pos(view_id);

                    for y in 0..text_area.height {
                        let line_idx = y + line_pos;
                        let width = if line_idx >= buffer.rope().len_lines() {
                            0
                        } else {
                            buffer.rope().line_without_line_ending(line_idx).width(0)
                        };
                        for x in 0..text_area.width {
                            if x > width {
                                break;
                            }
                            let current = Point {
                                column: x as i64,
                                line: y as i64,
                            };
                            if current >= start && current < end {
                                let cell = buf
                                    .cell_mut((
                                        (x + text_area.left()) as u16,
                                        (y + text_area.top()) as u16,
                                    ))
                                    .unwrap();
                                cell.bg = bg;
                            }
                        }
                    }
                }
            }

            if buffer.views[view_id].completer.visible
                && !buffer.views[view_id].completer.matching_words.is_empty()
            {
                profiling::scope!("draw completer");
                let cursor_view_pos =
                    buffer.cursor_view_pos(view_id, 0, text_area.width, text_area.height);
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
                        let rect =
                            Rect::new(column + text_area.x, line + text_area.y + i + 1, longest, 1);
                        let rect = rect.intersection(text_area);
                        if rect.area() > 0 {
                            Clear.render(rect.into(), buf);
                            buf.set_stringn(
                                (rect.x + 1) as u16,
                                rect.y as u16,
                                word,
                                rect.width,
                                tui::style::Style::default(),
                            );
                            let style = if i == buffer.views[view_id].completer.index {
                                theme.completer_selected
                            } else {
                                theme.completer
                            };
                            buf.set_style(rect.into(), style);
                        }
                    }
                }
            }

            if *scrollbar {
                profiling::scope!("draw scrollbar");
                let scrollbar_bounds = get_scrollbar_bounds(buffer, view_id, bounds);
                let cell_size = bounds.cell_size();
                let rect = Rect::new(
                    view_bounds.x as f32 + view_bounds.width as f32 - cell_size.x,
                    view_bounds.y as f32,
                    cell_size.x,
                    view_bounds.height as f32,
                );

                let mut conflict_areas = ArenaVec::new_in(&arena);
                conflict_areas.reserve_exact(normalized_conflicts.len() * 2);
                // Draw git conflicts in the scrollbar
                let conflict_width = cell_size.x;
                for (start, middle, end) in normalized_conflicts {
                    let start = start * view_bounds.height as f32;
                    let middle = middle * view_bounds.height as f32;
                    let end = end * view_bounds.height as f32;
                    conflict_areas.push((
                        Rect::new(
                            view_bounds.x as f32 + view_bounds.width as f32 - conflict_width,
                            start,
                            conflict_width,
                            middle - start,
                        ),
                        theme.conflict_current.bg.unwrap_or_default(),
                    ));
                    conflict_areas.push((
                        Rect::new(
                            view_bounds.x as f32 + view_bounds.width as f32 - conflict_width,
                            middle,
                            conflict_width,
                            end - middle,
                        ),
                        theme.conflict_incoming.bg.unwrap_or_default(),
                    ));
                }

                if painter.has_painter2d() {
                    let painter2d = layer.painter2d.as_mut().unwrap();
                    painter2d.draw_quad(rect, theme.scrollbar.bg.unwrap_or_default());

                    for (rect, color) in conflict_areas {
                        painter2d.draw_quad(rect, color);
                    }

                    painter2d.draw_quad(scrollbar_bounds, theme.scrollbar.fg.unwrap_or_default());
                } else {
                    // TODO: use 1/8 blocks to make bar higher resolution
                    let rect = Rect::new(
                        rect.x as usize,
                        rect.y as usize,
                        rect.width as usize,
                        rect.height as usize,
                    );
                    Clear.render(rect.into(), buf);
                    buf.set_style(rect.into(), theme.scrollbar);
                    let rect = Rect::new(
                        scrollbar_bounds.x as usize,
                        scrollbar_bounds.y as usize,
                        scrollbar_bounds.width as usize,
                        scrollbar_bounds.height as usize,
                    );
                    for (rect, color) in conflict_areas {
                        let rect = Rect::new(
                            rect.x as usize,
                            rect.y as usize,
                            rect.width as usize,
                            rect.height.ceil() as usize,
                        );
                        buf.set_style(rect.into(), Style::default().bg(color));
                    }
                    buf.set_style(
                        rect.into(),
                        Style::default().bg(theme.scrollbar.fg.unwrap_or_default()),
                    );
                }
            }

            if info_line {
                let path = if let Some(path) = buffer.file() {
                    path.to_string_lossy().into()
                } else {
                    buffer.name().to_string()
                };

                let info_line = InfoLineView {
                    theme,
                    config: &config.info_line,
                    focus: self.has_focus,
                    encoding: buffer.encoding,
                    path,
                    line: buffer.cursor_line_idx(view_id, 0) + 1,
                    column: buffer.cursor_grapheme_column(view_id, 0) + 1,
                    dirty: buffer.is_dirty(),
                    branch,
                    language: buffer.language_name().into(),
                    size: buffer.rope().len_bytes(),
                    read_only: buffer.read_only_file,
                    spinner: *spinner,
                    parent_unique_id: unique_id,
                };
                let cell_size = bounds.cell_size();
                info_line.render(
                    &mut (),
                    Bounds::new(
                        Rect::new(
                            view_bounds.x,
                            (view_bounds.y as f32 + view_bounds.height as f32 - cell_size.y).round()
                                as usize,
                            view_bounds.width,
                            (1.0 * cell_size.y).round() as usize,
                        ),
                        cell_size,
                        rounding,
                    ),
                    painter,
                );
            }
        }
    }
}

fn get_scrollbar_bounds(buffer: &Buffer, view_id: ViewId, bounds: Bounds) -> Rect<f32> {
    let view_bounds = bounds.view_bounds();
    let cell_size = bounds.cell_size();

    let viewport_height = (view_bounds.height as f32 - cell_size.y).max(0.0);
    let content_height = (buffer.len_lines() as f32 * cell_size.y + viewport_height - cell_size.y)
        .max(cell_size.y)
        - cell_size.y;
    let content_offset = buffer.views[view_id].line_pos as f32 * cell_size.y;

    let scrollbar_ratio = viewport_height / content_height;
    let scrollbar_pos_ratio = content_offset / content_height;

    // TODO: prevent scroll bar from clipping outside of track
    // probably by doing some thumb height / 2 + nonsense
    let thumb_heigh = (scrollbar_ratio * viewport_height).max(cell_size.y);
    let scrollbar_pos = scrollbar_pos_ratio * viewport_height;

    let x = view_bounds.x as f32 + (view_bounds.width as f32 - cell_size.x).max(0.0);
    let y = view_bounds.y as f32 + scrollbar_pos;
    Rect::new(x, y, cell_size.x, thumb_heigh)
}

fn get_scrollbar_track(bounds: Bounds) -> Rect<f32> {
    let view_bounds = bounds.view_bounds();
    let cell_size = bounds.cell_size();
    Rect::new(
        view_bounds.x as f32 + view_bounds.width as f32 - cell_size.x,
        view_bounds.y as f32,
        cell_size.x,
        view_bounds.height as f32 - cell_size.y,
    )
}
