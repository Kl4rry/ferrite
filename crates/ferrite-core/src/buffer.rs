use core::fmt;
use std::{
    cmp, fs, io,
    num::NonZeroUsize,
    ops::Range,
    path::{Path, PathBuf},
    sync::OnceLock,
    time::{Duration, Instant},
};

use encoding_rs::Encoding;
use ferrite_utility::{
    graphemes::RopeGraphemeExt,
    line_ending::{rope_end_without_line_ending, LineEnding, DEFAULT_LINE_ENDING},
    point::Point,
    vec1::Vec1,
};
use ropey::{Rope, RopeSlice};
use serde::{Deserialize, Serialize};
use slotmap::SlotMap;

use self::{error::BufferError, history::History, search::BufferSearcher};
use super::{
    indent::Indentation,
    language::{get_language_from_path, syntax::Syntax},
};
use crate::{
    clipboard, cmd::LineMoveDir, event_loop_proxy::EventLoopProxy,
    language::detect::detect_language, workspace::BufferData,
};

pub mod case;
pub mod encoding;
pub mod error;
mod format;
mod history;
pub mod input;
pub mod read;
pub mod search;
pub mod write;

#[cfg(test)]
pub mod buffer_tests;

static PROXY: OnceLock<Box<dyn EventLoopProxy>> = OnceLock::new();

pub fn set_buffer_proxy(proxy: Box<dyn EventLoopProxy>) {
    if PROXY.set(proxy).is_err() {
        tracing::error!("Error attempted to set buffer proxy twice");
    }
}

fn get_buffer_proxy() -> Box<dyn EventLoopProxy> {
    PROXY.get().unwrap().dup()
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cursor {
    pub anchor: usize,
    pub position: usize,
    pub affinity: usize,
}

impl Cursor {
    pub fn has_selection(&self) -> bool {
        self.position != self.anchor
    }

    pub fn intersects(&self, other: Cursor) -> bool {
        let range = self.position.min(self.anchor)..self.position.max(self.anchor);
        range.contains(&other.position) || range.contains(&other.position)
    }

    pub fn coalesce(self, other: Cursor) -> Self {
        if self.position >= self.anchor {
            Self {
                position: self.position.max(other.position),
                anchor: self.anchor.min(other.anchor),
                affinity: self.affinity,
            }
        } else {
            Self {
                position: self.position.min(other.position),
                anchor: self.anchor.max(other.anchor),
                affinity: self.affinity,
            }
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Selection {
    pub start: Point<i64>,
    pub end: Point<i64>,
}

pub struct View {
    pub cursors: Vec1<Cursor>,
    pub line_pos: usize,
    pub col_pos: usize,
    last_click: Instant,
    last_click_pos: Point<usize>,
    clicks_in_a_row: u8,
    pub clamp_cursor: bool,
    searcher: Option<BufferSearcher>,
    pub replacement: Option<String>,
    view_lines: usize,
    view_columns: usize,
}

impl Default for View {
    fn default() -> Self {
        Self {
            cursors: Vec1::default(),
            line_pos: 0,
            col_pos: 0,
            last_click: Instant::now(),
            last_click_pos: Point::new(0, 0),
            clicks_in_a_row: 0,
            clamp_cursor: true,
            searcher: None,
            replacement: None,
            view_lines: 100000,
            view_columns: 100000,
        }
    }
}

impl Clone for View {
    fn clone(&self) -> Self {
        Self {
            cursors: self.cursors.clone(),
            line_pos: self.line_pos,
            col_pos: self.col_pos,
            last_click: self.last_click,
            last_click_pos: self.last_click_pos,
            clicks_in_a_row: self.clicks_in_a_row,
            clamp_cursor: self.clamp_cursor,
            searcher: None,    // TODO: fix
            replacement: None, // TODO: fix
            view_lines: self.view_lines,
            view_columns: self.view_columns,
        }
    }
}

impl View {
    pub fn coalesce_cursors(&mut self) {
        let mut removed = 0;
        for i in 0..self.cursors.len() {
            for j in 0..self.cursors.len() {
                if i == j || j == 0 {
                    continue;
                }

                if self.cursors[i - removed].intersects(self.cursors[j - removed]) {
                    self.cursors[i - removed] =
                        self.cursors[i - removed].coalesce(self.cursors[j - removed]);
                    self.cursors.remove(j - removed);
                    removed += 1;
                }
            }
        }
    }

    pub fn clear(&mut self) {
        self.cursors.clear();
    }
}

slotmap::new_key_type! {
    pub struct ViewId;
}

pub struct Buffer {
    rope: Rope,
    pub views: SlotMap<ViewId, View>,
    file: Option<PathBuf>,
    name: String,
    dirty: bool,
    pub read_only: bool,
    pub read_only_file: bool,
    last_edit: Instant,
    pub line_ending: LineEnding,
    pub encoding: &'static Encoding,
    pub indent: Indentation,
    last_interact: Instant,
    // syntax highlight
    syntax: Option<Syntax>,
    history: History,
}

impl Clone for Buffer {
    fn clone(&self) -> Self {
        let rope = self.rope.clone();
        let mut syntax = Syntax::new(get_buffer_proxy());
        if let Err(err) = syntax.set_language(self.language_name()) {
            tracing::error!("Error setting language: {err}");
        }
        syntax.update_text(rope.clone());

        Self {
            rope,
            file: self.file.clone(),
            name: self.name.clone(),
            dirty: self.dirty,
            read_only: self.read_only,
            read_only_file: self.read_only_file,
            last_edit: self.last_edit,
            line_ending: self.line_ending,
            encoding: self.encoding,
            indent: self.indent,
            syntax: Some(syntax),
            history: self.history.clone(),
            last_interact: self.last_interact,
            views: self.views.clone(),
        }
    }
}

impl Default for Buffer {
    fn default() -> Self {
        Self {
            rope: Rope::new(),
            file: None,
            name: String::from("[scratch]"),
            encoding: encoding_rs::UTF_8,
            indent: Indentation::Tabs(NonZeroUsize::new(1).unwrap()),
            dirty: false,
            last_edit: Instant::now(),
            read_only: false,
            read_only_file: false,
            line_ending: DEFAULT_LINE_ENDING,
            syntax: None,
            history: History::default(),
            last_interact: Instant::now(),
            views: SlotMap::with_key(),
        }
    }
}

impl fmt::Display for Buffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.rope)
    }
}

impl Buffer {
    pub fn new() -> Self {
        Self::default()
    }

    #[allow(dead_code)]
    pub fn with_text(text: &str) -> Self {
        Self {
            indent: Indentation::detect_indent(text),
            rope: Rope::from(text),
            ..Default::default()
        }
    }

    pub fn with_path(path: impl Into<PathBuf>) -> Result<Self, anyhow::Error> {
        let path = path.into();
        let path = if path.has_root() {
            path
        } else {
            let cwd = std::env::current_dir()?;
            cwd.join(path)
        };

        let mut syntax = Syntax::new(get_buffer_proxy());
        if let Some(language) = get_language_from_path(&path) {
            if let Err(err) = syntax.set_language(language) {
                tracing::error!("Error setting language: {err}");
            }
            syntax.update_text(Rope::new());
        }

        let Some(name) = path.file_name() else {
            anyhow::bail!("path has no filename name");
        };
        let name = name.to_string_lossy().into();

        Ok(Self {
            name,
            file: Some(path),
            syntax: Some(syntax),
            ..Default::default()
        })
    }

    pub fn with_name(name: impl Into<String>) -> Self {
        let name = name.into();
        let path = Path::new(&name);
        let mut syntax = Syntax::new(get_buffer_proxy());
        if let Some(language) = get_language_from_path(path) {
            if let Err(err) = syntax.set_language(language) {
                tracing::error!("Error setting language: {err}");
            }
            syntax.update_text(Rope::new());
        }

        Self {
            name,
            syntax: Some(syntax),
            ..Default::default()
        }
    }

    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, io::Error> {
        let path = path.as_ref();
        #[cfg(not(unix))]
        let read_only_file = {
            let metadata = std::fs::metadata(path)?;
            metadata.permissions().readonly()
        };
        #[cfg(unix)]
        let read_only_file = rustix::fs::access(path, rustix::fs::Access::WRITE_OK).is_err();
        let (encoding, rope) = read::read_from_file(path)?;

        let mut syntax = Syntax::new(get_buffer_proxy());
        if let Some(language) = get_language_from_path(path) {
            if let Err(err) = syntax.set_language(language) {
                tracing::error!("Error setting language: {err}");
            }
            syntax.update_text(rope.clone());
        }

        if let Some(language) = detect_language(syntax.get_language_name(), rope.clone()) {
            if let Err(err) = syntax.set_language(language) {
                tracing::error!("Error setting language: {err}");
            }
            syntax.update_text(rope.clone());
        }

        let name = path.file_name().unwrap().to_string_lossy().into();

        Ok(Self {
            indent: Indentation::detect_indent_rope(rope.slice(..)),
            rope,
            read_only_file,
            name,
            file: Some(dunce::canonicalize(path)?),
            encoding,
            syntax: Some(syntax),
            ..Default::default()
        })
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, io::Error> {
        let (encoding, rope) = read::read(bytes)?;
        let mut syntax = Syntax::new(get_buffer_proxy());

        if let Some(language) = detect_language(None, rope.clone()) {
            if let Err(err) = syntax.set_language(language) {
                tracing::error!("Error setting language: {err}");
            }
            syntax.update_text(rope.clone());
        }

        Ok(Self {
            indent: Indentation::detect_indent_rope(rope.slice(..)),
            rope,
            file: None,
            encoding,
            syntax: Some(syntax),
            ..Default::default()
        })
    }

    pub fn set_text(&mut self, text: &str) {
        self.rope = Rope::from(text);
        self.history = History::default();
        if let Some(ref mut syntax) = self.syntax {
            syntax.update_text(self.rope.clone());
        }
    }

    pub fn cursor(&self, view_id: ViewId, cursor_index: usize) -> Cursor {
        self.views[view_id].cursors[cursor_index]
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn set_view_lines(&mut self, view_id: ViewId, lines: usize) {
        self.views[view_id].view_lines = lines;
    }

    pub fn get_view_lines(&self, view_id: ViewId) -> usize {
        self.views[view_id].view_lines
    }

    pub fn set_view_columns(&mut self, view_id: ViewId, cols: usize) {
        self.views[view_id].view_columns = cols;
    }

    pub fn _get_view_columns(&self, view_id: ViewId) -> usize {
        self.views[view_id].view_columns
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    pub fn language_name(&self) -> &str {
        match &self.syntax {
            Some(syntax) => syntax.get_language_name().unwrap_or("text"),
            None => "text",
        }
    }

    pub fn set_langauge(
        &mut self,
        language: &str,
        proxy: Box<dyn EventLoopProxy>,
    ) -> anyhow::Result<()> {
        let syntax = match self.syntax.as_mut() {
            Some(syntax) => syntax,
            None => {
                self.syntax = Some(Syntax::new(proxy));
                self.syntax.as_mut().unwrap()
            }
        };
        syntax.set_language(language)?;
        syntax.update_text(self.rope.clone());
        Ok(())
    }

    pub fn get_buffer_view(&self, view_id: ViewId) -> BufferView {
        let view = &self.views[view_id];
        let end_line = cmp::min(self.rope.len_lines(), view.view_lines + view.line_pos);

        let mut lines = Vec::new();
        for line_idx in view.line_pos..end_line {
            let Some(line) = self.rope.get_line(line_idx) else {
                break;
            };
            let mut idx = 0;
            let mut width = 0;
            for grapheme in line.grapehemes() {
                if width >= view.col_pos {
                    break;
                }
                width += grapheme.width(width);
                idx += grapheme.len_bytes();
            }
            let line = line.byte_slice(idx..);
            lines.push(ViewLine {
                text: line,
                col_start_offset: width.saturating_sub(view.col_pos),
                text_start_col: self.rope.get_text_start_col(line_idx),
                text_end_col: self.rope.get_text_end_col(line_idx),
            });
        }

        BufferView { lines }
    }

    pub fn rope(&self) -> &Rope {
        &self.rope
    }

    pub fn file(&self) -> Option<&Path> {
        self.file.as_deref()
    }

    pub fn set_file(&mut self, path: impl Into<PathBuf>) -> Result<(), std::io::Error> {
        let path = path.into();
        if path.to_string_lossy().ends_with(std::path::MAIN_SEPARATOR) {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "File must have filename",
            ));
        }
        let Some(name) = path.file_name() else {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "File must have filename",
            ));
        };
        self.name = name.to_string_lossy().into();
        let path = if path.is_absolute() {
            path
        } else {
            let cwd = std::env::current_dir()?;
            cwd.join(path)
        };
        self.file = Some(path);
        Ok(())
    }

    pub fn line_pos(&self, view_id: ViewId) -> usize {
        self.views[view_id].line_pos
    }

    pub fn col_pos(&self, view_id: ViewId) -> usize {
        self.views[view_id].col_pos
    }

    pub fn len_lines(&self) -> usize {
        self.rope.len_lines()
    }

    pub fn cursor_view_pos(
        &self,
        view_id: ViewId,
        max_cols: usize,
        max_lines: usize,
    ) -> Vec<(usize, usize)> {
        let view = &self.views[view_id];
        let start_line = view.line_pos;
        let end_line = std::cmp::min(self.rope.len_lines(), max_lines + view.line_pos);
        let start_col = view.col_pos;
        let end_col = view.col_pos + max_cols;

        let mut output = Vec::new();
        for i in 0..view.cursors.len() {
            let line = self.cursor_line_idx(view_id, i);
            let col = self.cursor_grapheme_column(view_id, i);
            if col >= start_col && col < end_col && line >= start_line && line < end_line {
                output.push((col, line - start_line))
            }
        }
        output
    }

    pub fn get_view_selection(&self, view_id: ViewId) -> Vec<Selection> {
        let view = &self.views[view_id];
        let mut output = Vec::new();
        for i in 0..view.cursors.len() {
            let pos = Point {
                line: self.cursor_line_idx(view_id, i) as i64,
                column: self.cursor_grapheme_column(view_id, i) as i64,
            };

            let anchor = Point {
                line: self.anchor_line_idx(view_id, i) as i64,
                column: self.anchor_grapheme_column(view_id, i) as i64,
            };

            let mut start = pos.min(anchor);
            let mut end = pos.max(anchor);
            start.line -= view.line_pos as i64;
            end.line -= view.line_pos as i64;
            start.column -= view.col_pos as i64;
            end.column -= view.col_pos as i64;
            output.push(Selection { start, end });
        }
        output
    }

    pub fn cursor_line_idx(&self, view_id: ViewId, cursor_index: usize) -> usize {
        self.rope
            .byte_to_line(self.views[view_id].cursors[cursor_index].position)
    }

    pub fn anchor_line_idx(&self, view_id: ViewId, cursor_index: usize) -> usize {
        self.rope
            .byte_to_line(self.views[view_id].cursors[cursor_index].anchor)
    }

    pub fn cursor_pos(&self, view_id: ViewId, cursor_index: usize) -> (usize, usize) {
        let current_line = self.cursor_line_idx(view_id, cursor_index);
        let start_of_line = self.rope.line_to_byte(current_line);
        let column = self.views[view_id].cursors[cursor_index].position - start_of_line;

        (column, current_line)
    }

    pub fn anchor_pos(&self, view_id: ViewId, cursor_index: usize) -> (usize, usize) {
        let current_line = self.anchor_line_idx(view_id, cursor_index);
        let start_of_line = self.rope.line_to_byte(current_line);
        let column = self.views[view_id].cursors[cursor_index].anchor - start_of_line;

        (column, current_line)
    }

    pub fn cursor_grapheme_column(&self, view_id: ViewId, cursor_index: usize) -> usize {
        let (column_idx, line_idx) = self.cursor_pos(view_id, cursor_index);
        let line = self.rope.line(line_idx);
        let start = line.byte_slice(..column_idx);
        start.width(0)
    }

    pub fn anchor_grapheme_column(&self, view_id: ViewId, cursor_index: usize) -> usize {
        let (column_idx, line_idx) = self.anchor_pos(view_id, cursor_index);
        let line = self.rope.line(line_idx);
        let start = line.byte_slice(..column_idx);
        start.width(0)
    }

    pub fn update_affinity(&mut self, view_id: ViewId) {
        for i in 0..self.views[view_id].cursors.len() {
            self.views[view_id].cursors[i].affinity = self.cursor_grapheme_column(view_id, i);
        }
    }

    pub fn vertical_scroll(&mut self, view_id: ViewId, distance: i64) {
        let len_lines = self.len_lines();
        self.views[view_id].line_pos = (self.views[view_id].line_pos as i128 + distance as i128)
            .clamp(0, len_lines as i128 - 1) as usize;
    }

    pub fn horizontal_scroll(&mut self, view_id: ViewId, distance: i64) {
        self.views[view_id].col_pos = (self.views[view_id].col_pos as i128 + distance as i128)
            .clamp(0, usize::MAX as i128 - 1) as usize;
    }

    // TODO make this multicursor aware
    // TODO make this remove selection but not move cursor
    pub fn move_right_char(&mut self, view_id: ViewId, shift: bool) {
        self.views[view_id].cursors.clear();

        let new_idx = self
            .rope
            .next_grapheme_boundary_byte(self.views[view_id].cursors.first().position);
        self.views[view_id].cursors.first_mut().position = new_idx;

        if !shift {
            if self.views[view_id].cursors.first().anchor
                > self.views[view_id].cursors.first().position
            {
                self.views[view_id].cursors.first_mut().position =
                    self.views[view_id].cursors.first().anchor;
            } else {
                self.views[view_id].cursors.first_mut().anchor =
                    self.views[view_id].cursors.first().position;
            }
        }

        self.update_affinity(view_id);
        self.history.finish();

        if self.views[view_id].clamp_cursor {
            self.center_on_cursor(view_id);
        }
    }

    // TODO make this multicursor aware
    // TODO make this remove selection but not move cursor
    pub fn move_left_char(&mut self, view_id: ViewId, shift: bool) {
        self.views[view_id].cursors.clear();

        let new_idx = self
            .rope
            .prev_grapheme_boundary_byte(self.views[view_id].cursors.first().position);
        self.views[view_id].cursors.first_mut().position = new_idx;

        if !shift {
            if self.views[view_id].cursors.first().anchor
                < self.views[view_id].cursors.first().position
            {
                self.views[view_id].cursors.first_mut().position =
                    self.views[view_id].cursors.first().anchor;
            } else {
                self.views[view_id].cursors.first_mut().anchor =
                    self.views[view_id].cursors.first().position;
            }
        }

        self.update_affinity(view_id);
        self.history.finish();

        if self.views[view_id].clamp_cursor {
            self.center_on_cursor(view_id);
        }
    }

    // TODO make this mulicursor aware
    pub fn move_down(&mut self, view_id: ViewId, shift: bool, distance: usize) {
        self.views[view_id].cursors.clear();

        let (column_idx, line_idx) = self.cursor_pos(view_id, 0);
        let new_line_idx = (line_idx + distance).min(self.rope.len_lines().saturating_sub(1));
        if line_idx == new_line_idx {
            return;
        }

        let view: &mut View = &mut self.views[view_id];
        let before_cursor = self
            .rope
            .line_without_line_ending(line_idx)
            .byte_slice(..column_idx)
            .width(0)
            .max(view.cursors.first().affinity);
        let next_line = self.rope.line_without_line_ending(new_line_idx);
        let next_width = next_line.width(0);
        let next_line_start = self.rope.line_to_byte(new_line_idx);

        if next_width < before_cursor {
            view.cursors.first_mut().position = next_line_start + next_line.len_bytes();
        } else {
            let idx = next_line.nth_next_grapheme_boundary_byte(0, before_cursor);
            view.cursors.first_mut().position = next_line_start + idx;
        }

        if !shift {
            view.cursors.first_mut().anchor = view.cursors.first().position;
        }

        self.history.finish();

        if view.clamp_cursor {
            self.center_on_cursor(view_id);
        }
    }

    // TODO make this mulicursor aware
    pub fn move_up(&mut self, view_id: ViewId, shift: bool, distance: usize) {
        self.views[view_id].cursors.clear();

        let (column_idx, line_idx) = self.cursor_pos(view_id, 0);
        if line_idx == 0 {
            return;
        }

        let new_line_idx = line_idx.saturating_sub(distance);

        let before_cursor = self
            .rope
            .line_without_line_ending(line_idx)
            .byte_slice(..column_idx)
            .width(0)
            .max(self.views[view_id].cursors.first().affinity);
        let next_line = self.rope.line_without_line_ending(new_line_idx);
        let next_width = next_line.width(0);
        let next_line_start = self.rope.line_to_byte(new_line_idx);

        if next_width < before_cursor {
            self.views[view_id].cursors.first_mut().position =
                next_line_start + next_line.len_bytes();
        } else {
            let idx = next_line.nth_next_grapheme_boundary_byte(0, before_cursor);
            self.views[view_id].cursors.first_mut().position = next_line_start + idx;
        }

        if !shift {
            self.views[view_id].cursors.first_mut().anchor =
                self.views[view_id].cursors.first().position;
        }

        self.history.finish();

        if self.views[view_id].clamp_cursor {
            self.center_on_cursor(view_id);
        }
    }

    // TODO make multicursor aware
    pub fn select_word(&mut self, view_id: ViewId) {
        self.views[view_id].cursors.clear();
        // TODO add matching multi selection when already having a selection
        if !self.views[view_id].cursors.first().has_selection() {
            let mut start_byte_idx = self.views[view_id].cursors.first().position;
            loop {
                let new_idx = self.rope.prev_grapheme_boundary_byte(start_byte_idx);
                let grapheme = self.rope.byte_slice(new_idx..start_byte_idx);
                if new_idx == start_byte_idx || !grapheme.is_word_char() {
                    break;
                }
                start_byte_idx = new_idx;
            }

            let mut end_byte_idx = self.views[view_id].cursors.first().position;
            loop {
                let new_idx = self.rope.next_grapheme_boundary_byte(end_byte_idx);
                let grapheme = self.rope.byte_slice(end_byte_idx..new_idx);
                if new_idx == end_byte_idx || !grapheme.is_word_char() {
                    break;
                }
                end_byte_idx = new_idx;
            }

            self.views[view_id].cursors.first_mut().position = end_byte_idx;
            self.views[view_id].cursors.first_mut().anchor = start_byte_idx;

            self.history.finish();
        }
    }

    fn next_word_end(&self, view_id: ViewId, cursor_index: usize, greedy: bool) -> usize {
        let view = &self.views[view_id];
        let mut current_idx = view.cursors[cursor_index].position;
        let mut skipping = Skipping::None;
        loop {
            let new_idx = self.rope.next_grapheme_boundary_byte(current_idx);
            if new_idx == current_idx {
                break;
            }

            let grapheme = self.rope.byte_slice(current_idx..new_idx);
            match skipping {
                Skipping::Whitespace => {
                    skipping = if grapheme.is_word_char() {
                        if greedy {
                            Skipping::WordChar
                        } else {
                            break;
                        }
                    } else if grapheme.is_whitespace() {
                        if grapheme.get_line_ending().is_some() {
                            break;
                        }
                        Skipping::Whitespace
                    } else if greedy {
                        Skipping::Other
                    } else {
                        break;
                    }
                }
                Skipping::WordChar => {
                    if !grapheme.is_word_char() {
                        break;
                    }
                }
                Skipping::Other => {
                    if grapheme.is_whitespace() || grapheme.is_word_char() {
                        break;
                    }
                }
                Skipping::None => {
                    skipping = if grapheme.is_whitespace() {
                        Skipping::Whitespace
                    } else if grapheme.is_word_char() {
                        Skipping::WordChar
                    } else {
                        Skipping::Other
                    };
                }
            }
            current_idx = new_idx;
        }
        current_idx
    }

    fn prev_word_start(&self, view_id: ViewId, cursor_index: usize, greedy: bool) -> usize {
        let view = &self.views[view_id];
        let mut current_idx = view.cursors[cursor_index].position;
        let mut skipping = Skipping::None;
        loop {
            let new_idx = self.rope.prev_grapheme_boundary_byte(current_idx);
            if new_idx == current_idx {
                break;
            }

            let grapheme = self.rope.byte_slice(new_idx..current_idx);
            match skipping {
                Skipping::Whitespace => {
                    skipping = if grapheme.is_word_char() {
                        if greedy {
                            Skipping::WordChar
                        } else {
                            break;
                        }
                    } else if grapheme.is_whitespace() {
                        if grapheme.get_line_ending().is_some() {
                            break;
                        }
                        Skipping::Whitespace
                    } else if greedy {
                        Skipping::Other
                    } else {
                        break;
                    }
                }
                Skipping::WordChar => {
                    if !grapheme.is_word_char() {
                        break;
                    }
                }
                Skipping::Other => {
                    if grapheme.is_whitespace() || grapheme.is_word_char() {
                        break;
                    }
                }
                Skipping::None => {
                    skipping = if grapheme.is_whitespace() {
                        Skipping::Whitespace
                    } else if grapheme.is_word_char() {
                        Skipping::WordChar
                    } else {
                        Skipping::Other
                    };
                }
            }
            current_idx = new_idx;
        }
        current_idx
    }

    // TODO make multicursor aware
    pub fn move_right_word(&mut self, view_id: ViewId, shift: bool) {
        self.views[view_id].cursors.clear();
        let next_word = self.next_word_end(view_id, 0, true);
        self.views[view_id].cursors.first_mut().position = next_word;

        if !shift {
            self.views[view_id].cursors.first_mut().anchor =
                self.views[view_id].cursors.first().position;
        }

        self.update_affinity(view_id);
        self.history.finish();

        if self.views[view_id].clamp_cursor {
            self.center_on_cursor(view_id);
        }
    }

    // TODO make multicursor aware
    pub fn move_left_word(&mut self, view_id: ViewId, shift: bool) {
        self.views[view_id].cursors.clear();
        let prev_word = self.prev_word_start(view_id, 0, true);
        self.views[view_id].cursors.first_mut().position = prev_word;

        if !shift {
            self.views[view_id].cursors.first_mut().anchor =
                self.views[view_id].cursors.first().position;
        }

        self.update_affinity(view_id);
        self.history.finish();

        if self.views[view_id].clamp_cursor {
            self.center_on_cursor(view_id);
        }
    }

    /// Move cursor to line. Line is indexed from 1
    pub fn goto(&mut self, view_id: ViewId, line: i64) {
        self.views[view_id].cursors.clear();
        let line_idx = (self.rope.len_lines().saturating_sub(1) as i64)
            .min(line.saturating_sub(1))
            .max(0) as usize;

        self.set_cursor_pos(view_id, 0, 0, line_idx);
        self.history.finish();
    }

    // TODO make multicursor aware
    pub fn home(&mut self, view_id: ViewId, shift: bool) {
        self.views[view_id].cursors.clear();
        let (col, line_idx) = self.cursor_pos(view_id, 0);
        let line = self.rope.line_without_line_ending(line_idx);

        let mut byte_col = 0;
        for grapheme in line.grapehemes() {
            if byte_col >= col {
                byte_col = 0;
                break;
            }

            if grapheme.chars().any(char::is_whitespace) {
                byte_col += grapheme.len_bytes();
            } else {
                break;
            }
        }

        let byte = self.rope.line_to_byte(line_idx) + byte_col;
        self.views[view_id].cursors.first_mut().position = byte;
        if !shift {
            self.views[view_id].cursors.first_mut().anchor =
                self.views[view_id].cursors.first().position;
        }

        self.update_affinity(view_id);
        self.history.finish();

        if self.views[view_id].clamp_cursor {
            self.center_on_cursor(view_id);
        }
    }

    // TODO make multicursor aware
    pub fn end(&mut self, view_id: ViewId, shift: bool) {
        self.views[view_id].cursors.clear();
        let line_idx = self.cursor_line_idx(view_id, 0);
        let byte = self.rope.line_to_byte(line_idx);
        let line_len = self.rope.line_without_line_ending(line_idx).len_bytes();
        self.views[view_id].cursors.first_mut().position = byte + line_len;
        if !shift {
            self.views[view_id].cursors.first_mut().anchor =
                self.views[view_id].cursors.first().position;
        }

        self.update_affinity(view_id);
        self.history.finish();

        if self.views[view_id].clamp_cursor {
            self.center_on_cursor(view_id);
        }
    }

    pub fn start(&mut self, view_id: ViewId, shift: bool) {
        self.views[view_id].cursors.clear();
        self.views[view_id].cursors.first_mut().position = 0;
        if !shift {
            self.views[view_id].cursors.first_mut().anchor =
                self.views[view_id].cursors.first().position;
        }

        self.update_affinity(view_id);
        self.history.finish();

        if self.views[view_id].clamp_cursor {
            self.center_on_cursor(view_id);
        }
    }

    pub fn eof(&mut self, view_id: ViewId, shift: bool) {
        self.views[view_id].cursors.clear();
        self.views[view_id].cursors.first_mut().position = self.rope.len_bytes();
        if !shift {
            self.views[view_id].cursors.first_mut().anchor =
                self.views[view_id].cursors.first().position;
        }

        self.update_affinity(view_id);
        self.history.finish();

        if self.views[view_id].clamp_cursor {
            self.center_on_cursor(view_id);
        }
    }

    // TODO make multicursor aware
    pub fn insert_text(&mut self, view_id: ViewId, text: &str, auto_indent: bool) {
        self.views[view_id].cursors.clear();
        if text.is_empty() {
            return;
        }
        /*let mut text = Cow::Borrowed(text);
        if memchr::memchr(b'\r', text.as_bytes()).is_some() {
            text = text.replace("\r", "").into();
        }
        let text: &str = &text;*/

        self.history
            .begin(*self.views[view_id].cursors.first(), self.dirty);

        fn get_pair_char(s: &str) -> Option<&str> {
            Some(match s {
                "{" => "}",
                "[" => "]",
                "(" => ")",
                "'" => "'",
                "\"" => "\"",
                "`" => "`",
                "<" => ">",
                _ => return None,
            })
        }

        let lines = Rope::from_str(text).len_lines();

        let (inserted_bytes, finish) = if self.views[view_id].cursors.first().has_selection() {
            let start_byte_idx = self.views[view_id]
                .cursors
                .first()
                .position
                .min(self.views[view_id].cursors.first().anchor);
            let end_byte_idx = self.views[view_id]
                .cursors
                .first()
                .position
                .max(self.views[view_id].cursors.first().anchor);
            if let Some(pair) = get_pair_char(text) {
                self.history.insert(&mut self.rope, start_byte_idx, text);
                self.history.insert(&mut self.rope, end_byte_idx + 1, pair);
                self.views[view_id].cursors.first_mut().position = end_byte_idx;
                self.views[view_id].cursors.first_mut().anchor = end_byte_idx;
            } else {
                self.history
                    .replace(&mut self.rope, start_byte_idx..end_byte_idx, text);
                self.views[view_id].cursors.first_mut().position = self.views[view_id]
                    .cursors
                    .first()
                    .position
                    .min(self.views[view_id].cursors.first().anchor);
                self.views[view_id].cursors.first_mut().anchor =
                    self.views[view_id].cursors.first_mut().position;
            }
            (text.len(), false)
        } else if auto_indent && lines > 1 {
            let indent = self.guess_indent(self.views[view_id].cursors.first().position);
            let min_indent_width = Rope::from_str(&indent).width(0);

            let mut smallest_indent_width = usize::MAX;
            for line in Rope::from_str(text).lines() {
                if line.is_whitespace() {
                    continue;
                }
                let text_start_col = line.get_text_start_col(0);
                smallest_indent_width = smallest_indent_width.min(text_start_col);
            }

            let current_line = self.rope.line(
                self.rope
                    .byte_to_line(self.views[view_id].cursors.first().position),
            );
            let current_line_is_whitespace = current_line.is_whitespace();
            let current_line_text_start = current_line.get_text_start_col(0);

            let mut input = String::new();
            let mut first = true;
            for line in Rope::from_str(text).lines() {
                let line_text_start_col = line.get_text_start_col(0);
                let extra_indent_width = line_text_start_col.saturating_sub(smallest_indent_width);
                let string = line.to_string();
                let trimmed = if line.is_whitespace() {
                    string.as_str()
                } else {
                    string.trim_start()
                };

                let total_indent_width = min_indent_width + extra_indent_width;
                if first {
                    if !line.is_whitespace() && current_line_is_whitespace {
                        input.push_str(&self.indent.from_width(
                            total_indent_width.saturating_sub(current_line_text_start),
                        ));
                    }
                    first = false;
                } else {
                    input.push_str(&self.indent.from_width(total_indent_width));
                }

                input.push_str(trimmed);
            }

            self.history.insert(
                &mut self.rope,
                self.views[view_id].cursors.first().position,
                &input,
            );
            /*if let Some(pair) = get_pair_char(text) {
                self.history
                    .insert(&mut self.rope, self.cursors.first().position + text.len(), pair);
            }*/
            (input.len(), true)
        } else {
            self.history.insert(
                &mut self.rope,
                self.views[view_id].cursors.first().position,
                text,
            );
            /*if let Some(pair) = get_pair_char(text) {
                self.history
                    .insert(&mut self.rope, self.cursors.first().position + text.len(), pair);
            }*/
            (text.len(), false)
        };

        self.views[view_id].cursors.first_mut().position += inserted_bytes;
        self.views[view_id].cursors.first_mut().anchor =
            self.views[view_id].cursors.first().position;

        if self.views[view_id].clamp_cursor {
            self.center_on_cursor(view_id);
        }

        self.update_affinity(view_id);
        self.mark_dirty();
        self.ensure_every_cursor_is_valid();

        if finish {
            self.history.finish();
        }
    }

    // TODO make multicursor aware
    pub fn backspace(&mut self, view_id: ViewId) {
        self.views[view_id].cursors.clear();
        // this is a bit hacky but it works
        {
            let line_idx = self.cursor_line_idx(view_id, 0);
            let line_byte =
                self.views[view_id].cursors.first().position - self.rope.line_to_byte(line_idx);
            if !self.views[view_id].cursors.first().has_selection()
                && line_byte <= self.rope.get_text_start_byte(line_idx)
                && line_byte != 0
            {
                // FIXME back tab does not move the cursors.first() correctly when standing in the middle of the indentation
                self.tab(view_id, true);
                return;
            }
        }

        self.history
            .begin(*self.views[view_id].cursors.first(), self.dirty);
        let (start_byte_idx, end_byte_idx) = if !self.views[view_id].cursors.first().has_selection()
        {
            let start_byte_idx = self
                .rope
                .prev_grapheme_boundary_byte(self.views[view_id].cursors.first().position);

            //let start_byte = self.rope.get_byte(start_byte_idx);
            //let end_byte = self.rope.get_byte(start_byte_idx + 1);
            let end_byte_idx = self.views[view_id].cursors.first().position;

            // Remove pair
            /*
            let end_byte_idx = match (start_byte, end_byte) {
                (Some(b'{'), Some(b'}')) => self.cursors.first().position + 1,
                (Some(b'['), Some(b']')) => self.cursors.first().position + 1,
                (Some(b'('), Some(b')')) => self.cursors.first().position + 1,
                (Some(b'\''), Some(b'\'')) => self.cursors.first().position + 1,
                (Some(b'"'), Some(b'"')) => self.cursors.first().position + 1,
                _ => self.cursors.first().position,
            };*/

            (start_byte_idx, end_byte_idx)
        } else {
            let start_byte_idx = self.views[view_id]
                .cursors
                .first()
                .position
                .min(self.views[view_id].cursors.first().anchor);
            let end_byte_idx = self.views[view_id]
                .cursors
                .first()
                .position
                .max(self.views[view_id].cursors.first().anchor);
            (start_byte_idx, end_byte_idx)
        };

        self.history
            .remove(&mut self.rope, start_byte_idx..end_byte_idx);

        self.views[view_id].cursors.first_mut().position = start_byte_idx;
        self.views[view_id].cursors.first_mut().anchor =
            self.views[view_id].cursors.first().position;

        self.update_affinity(view_id);

        if start_byte_idx != end_byte_idx {
            self.mark_dirty();
            self.ensure_every_cursor_is_valid();
        }

        if self.views[view_id].clamp_cursor {
            self.center_on_cursor(view_id);
        }
    }

    pub fn backspace_word(&mut self, view_id: ViewId) {
        if self.views[view_id].cursors.first().has_selection() {
            self.backspace(view_id);
            return;
        }

        self.history
            .begin(*self.views[view_id].cursors.first(), self.dirty);
        let prev_word = self.prev_word_start(view_id, 0, false);
        self.history.remove(
            &mut self.rope,
            prev_word..self.views[view_id].cursors.first().position,
        );

        if prev_word != self.views[view_id].cursors.first().position {
            self.mark_dirty();
            self.ensure_every_cursor_is_valid();
        }

        self.views[view_id].cursors.first_mut().position = prev_word;
        self.views[view_id].cursors.first_mut().anchor =
            self.views[view_id].cursors.first().position;
        self.update_affinity(view_id);

        if self.views[view_id].clamp_cursor {
            self.center_on_cursor(view_id);
        }
        self.history.finish();
    }

    // TODO make multicursor aware
    pub fn delete(&mut self, view_id: ViewId) {
        self.views[view_id].cursors.clear();
        self.history
            .begin(*self.views[view_id].cursors.first(), self.dirty);
        let (start_byte_idx, end_byte_idx) = if !self.views[view_id].cursors.first().has_selection()
        {
            let end_byte_idx = self
                .rope
                .next_grapheme_boundary_byte(self.views[view_id].cursors.first().position);
            (self.views[view_id].cursors.first().position, end_byte_idx)
        } else {
            let start_byte_idx = self.views[view_id]
                .cursors
                .first()
                .position
                .min(self.views[view_id].cursors.first().anchor);
            let end_byte_idx = self.views[view_id]
                .cursors
                .first()
                .position
                .max(self.views[view_id].cursors.first().anchor);
            (start_byte_idx, end_byte_idx)
        };

        self.history
            .remove(&mut self.rope, start_byte_idx..end_byte_idx);

        self.views[view_id].cursors.first_mut().position = start_byte_idx;
        self.views[view_id].cursors.first_mut().anchor =
            self.views[view_id].cursors.first().position;

        self.update_affinity(view_id);

        if start_byte_idx != end_byte_idx {
            self.mark_dirty();
            self.ensure_every_cursor_is_valid();
        }

        if self.views[view_id].clamp_cursor {
            self.center_on_cursor(view_id);
        }
    }

    // TODO make multicursor aware
    pub fn delete_word(&mut self, view_id: ViewId) {
        self.views[view_id].cursors.clear();
        if self.views[view_id].cursors.first().has_selection() {
            self.delete(view_id);
            return;
        }

        self.history
            .begin(*self.views[view_id].cursors.first(), self.dirty);
        let next_word = self.next_word_end(view_id, 0, false);

        self.history.remove(
            &mut self.rope,
            self.views[view_id].cursors.first().position..next_word,
        );
        self.update_affinity(view_id);

        if self.views[view_id].cursors.first().position != next_word {
            self.mark_dirty();
            self.ensure_every_cursor_is_valid();
        }

        if self.views[view_id].clamp_cursor {
            self.center_on_cursor(view_id);
        }
        self.history.finish();
    }

    // TODO make multicursor aware
    pub fn new_line(&mut self, view_id: ViewId) {
        self.views[view_id].cursors.clear();
        self.history
            .begin(*self.views[view_id].cursors.first(), self.dirty);
        self.end(view_id, false);
        self.history.insert(
            &mut self.rope,
            self.views[view_id].cursors.first().position,
            "\n",
        );
        self.views[view_id].cursors.first_mut().position += 1;
        self.views[view_id].cursors.first_mut().anchor =
            self.views[view_id].cursors.first().position;
        self.update_affinity(view_id);
        self.mark_dirty();
        self.ensure_every_cursor_is_valid();

        if self.views[view_id].clamp_cursor {
            self.center_on_cursor(view_id);
        }
        self.history.finish();
    }

    // TODO make multicursor aware
    pub fn move_line(&mut self, view_id: ViewId, dir: LineMoveDir) {
        self.views[view_id].cursors.clear();
        self.history
            .begin(*self.views[view_id].cursors.first(), self.dirty);
        let len_lines = self.rope.len_lines();
        let (cursor_col, cursor_line_idx) = self.cursor_pos(view_id, 0);
        let (anchor_col, anchor_line_idx) = self.anchor_pos(view_id, 0);

        let cursor_byte_idx_in_line =
            self.views[view_id].cursors.first().position - self.rope.line_to_byte(cursor_line_idx);
        let anchor_byte_idx_in_line =
            self.views[view_id].cursors.first().anchor - self.rope.line_to_byte(anchor_line_idx);

        let start_line_idx = cursor_line_idx.min(anchor_line_idx);
        let mut end_line_idx = cursor_line_idx.max(anchor_line_idx);

        let end_col = if self.views[view_id].cursors.first().position
            > self.views[view_id].cursors.first().anchor
        {
            cursor_col
        } else {
            anchor_col
        };
        if end_col == 0 && start_line_idx < end_line_idx {
            end_line_idx -= 1;
        }

        if (end_line_idx + 1 >= self.len_lines() && dir == LineMoveDir::Down)
            || (start_line_idx == 0 && dir == LineMoveDir::Up)
        {
            return;
        }

        let old_line_idx = self.rope.byte_to_line(
            self.views[view_id]
                .cursors
                .first()
                .position
                .min(self.views[view_id].cursors.first().anchor),
        );
        let offset = match dir {
            LineMoveDir::Up => -1,
            LineMoveDir::Down => 1,
        };
        let new_line_idx = (old_line_idx as i64 + offset) as usize;

        let start_byte_idx = self.rope.line_to_byte(start_line_idx);
        let end_byte_idx = self.rope.end_of_line_byte(end_line_idx);

        let mut removed = self
            .rope
            .byte_slice(start_byte_idx..end_byte_idx)
            .to_string();

        if RopeSlice::from(removed.as_str())
            .get_line_ending()
            .is_none()
        {
            removed.push('\n');
        }

        self.history
            .remove(&mut self.rope, start_byte_idx..end_byte_idx);
        let end_idx = self.rope.len_bytes();
        self.history.insert(&mut self.rope, end_idx, "\n");

        let new_line_start_byte_idx = self.rope.line_to_byte(new_line_idx);
        self.history
            .insert(&mut self.rope, new_line_start_byte_idx, &removed);

        while len_lines < self.rope.len_lines() && self.rope.get_line_ending().is_some() {
            let start = self
                .rope
                .char_to_byte(rope_end_without_line_ending(&self.rope.slice(..)));
            let end = self.rope.len_bytes();
            self.history.remove(&mut self.rope, start..end);
        }

        let new_cursor_line_idx = (cursor_line_idx as i64 + offset) as usize;
        let new_anchor_line_idx = (anchor_line_idx as i64 + offset) as usize;

        self.views[view_id].cursors.first_mut().position =
            self.rope.line_to_byte(new_cursor_line_idx) + cursor_byte_idx_in_line;
        self.views[view_id].cursors.first_mut().anchor =
            self.rope.line_to_byte(new_anchor_line_idx) + anchor_byte_idx_in_line;

        self.update_affinity(view_id);
        self.mark_dirty();
        self.ensure_every_cursor_is_valid();

        if self.views[view_id].clamp_cursor {
            self.center_on_cursor(view_id);
        }
        self.history.finish();
    }

    // TODO make multicursor aware
    pub fn tab(&mut self, view_id: ViewId, back: bool) {
        self.views[view_id].cursors.clear();
        // TODO optimize for larger files

        if !self.views[view_id].cursors.first().has_selection() && !back {
            let col = self.cursor_grapheme_column(view_id, 0);
            self.insert_text(view_id, &self.indent.to_next_ident(col), false);
            self.history.finish();
            return;
        }

        self.history
            .begin(*self.views[view_id].cursors.first(), self.dirty);
        {
            let cursor_col = self.cursor_grapheme_column(view_id, 0);
            let anchor_col = self.anchor_grapheme_column(view_id, 0);
            let cursor_line_idx = self.cursor_line_idx(view_id, 0);
            let anchor_line_idx = self.anchor_line_idx(view_id, 0);

            let start = self.rope.byte_to_line(
                self.views[view_id]
                    .cursors
                    .first()
                    .position
                    .min(self.views[view_id].cursors.first().anchor),
            );
            let end = self.rope.byte_to_line(
                self.views[view_id]
                    .cursors
                    .first()
                    .position
                    .max(self.views[view_id].cursors.first().anchor),
            );

            let last_line_at_start = self.views[view_id]
                .cursors
                .first()
                .position
                .max(self.views[view_id].cursors.first().anchor)
                == self.rope.line_to_byte(end);

            let tab_direction = match back {
                true => -1,
                false => 1,
            };

            for line_idx in start..=end {
                let line = self.rope.line_without_line_ending(line_idx);
                let line_start_byte_idx = self.rope.line_to_byte(line_idx);
                let text_start_byte = self.rope.get_text_start_byte(line_idx);
                let text_start_col = self.rope.get_text_start_col(line_idx);

                let diff: i64 = 'm: {
                    if line_idx == end && last_line_at_start {
                        break 'm 0;
                    }

                    let current_indent = line.byte_slice(..text_start_byte);
                    let current_indent_width = current_indent.width(0);
                    let indent_width = self.indent.width();

                    let new_number_of_indent = ((current_indent_width as i64 / indent_width as i64)
                        + tab_direction)
                        .max(0) as usize;
                    let new_start_of_line =
                        self.indent.to_next_ident(0).repeat(new_number_of_indent);

                    let start_byte_idx = line_start_byte_idx;
                    let end_byte_idx = line_start_byte_idx + text_start_byte;

                    self.history.replace(
                        &mut self.rope,
                        start_byte_idx..end_byte_idx,
                        &new_start_of_line,
                    );

                    new_number_of_indent as i64 * indent_width as i64 - current_indent_width as i64
                };

                if line_idx == cursor_line_idx {
                    self.views[view_id].cursors.first_mut().position =
                        self.rope.line_to_byte(cursor_line_idx);
                    if cursor_col < text_start_col || cursor_col == 0 {
                        self.set_cursor_col(view_id, cursor_col);
                    } else {
                        self.set_cursor_col(view_id, (cursor_col as i64 + diff) as usize);
                    }
                }

                if line_idx == anchor_line_idx {
                    self.views[view_id].cursors.first_mut().anchor =
                        self.rope.line_to_byte(anchor_line_idx);
                    if anchor_col < text_start_col || anchor_col == 0 {
                        self.set_anchor_col(view_id, anchor_col);
                    } else {
                        self.set_anchor_col(view_id, (anchor_col as i64 + diff) as usize);
                    }
                }
            }
        }

        self.update_affinity(view_id);
        self.mark_dirty();
        self.ensure_every_cursor_is_valid();

        if self.views[view_id].clamp_cursor {
            self.center_on_cursor(view_id);
        }
        self.history.finish();
    }

    // TODO make multicursor aware
    pub fn set_cursor_col(&mut self, view_id: ViewId, col: usize) {
        self.views[view_id].cursors.clear();
        let cursor_line_idx = self.cursor_line_idx(view_id, 0);
        let line = self.rope.line_without_line_ending(cursor_line_idx);
        let mut byte_idx = 0;
        let mut width = 0;
        for grapheme in line.grapehemes() {
            if width >= col {
                break;
            }
            byte_idx += grapheme.len_bytes();
            width += grapheme.width(width);
        }
        self.views[view_id].cursors.first_mut().position =
            self.rope.line_to_byte(cursor_line_idx) + byte_idx;
    }

    // TODO make multicursor aware
    pub fn set_anchor_col(&mut self, view_id: ViewId, col: usize) {
        self.views[view_id].cursors.clear();
        let anchor_line_idx = self.anchor_line_idx(view_id, 0);
        let line = self.rope.line_without_line_ending(anchor_line_idx);
        let mut byte_idx = 0;
        let mut width = 0;
        for grapheme in line.grapehemes() {
            if width >= col {
                break;
            }
            byte_idx += grapheme.len_bytes();
            width += grapheme.width(width);
        }
        self.views[view_id].cursors.first_mut().anchor =
            self.rope.line_to_byte(anchor_line_idx) + byte_idx;
    }

    pub fn select_all(&mut self, view_id: ViewId) {
        self.views[view_id].cursors.clear();
        self.views[view_id].cursors.first_mut().anchor = 0;
        self.views[view_id].cursors.first_mut().position = self.rope.len_bytes();

        self.update_affinity(view_id);
        self.history.finish();

        if self.views[view_id].clamp_cursor {
            self.center_on_cursor(view_id);
        }
    }

    // TODO make multicursor aware
    pub fn select_line(&mut self, view_id: ViewId) {
        self.views[view_id].cursors.clear();
        {
            let line_idx = self.cursor_line_idx(view_id, 0);
            let line_start = self.rope.line_to_byte(line_idx + 1);
            self.views[view_id].cursors.first_mut().position = line_start;
        }

        {
            let line_idx = self.anchor_line_idx(view_id, 0);
            let line_start = self.rope.line_to_byte(line_idx);
            self.views[view_id].cursors.first_mut().anchor = line_start;
        }

        self.update_affinity(view_id);
        self.history.finish();
    }

    // TODO make multicursor aware
    pub fn undo(&mut self, view_id: ViewId) {
        self.views[view_id].cursors.clear();
        self.history.undo(
            &mut self.rope,
            self.views[view_id].cursors.first_mut(),
            &mut self.dirty,
        );
        self.queue_syntax_update();
        if self.views[view_id].clamp_cursor {
            self.center_on_cursor(view_id);
        }
    }

    // TODO make multicursor aware
    pub fn redo(&mut self, view_id: ViewId) {
        self.views[view_id].cursors.clear();
        self.history.redo(
            &mut self.rope,
            self.views[view_id].cursors.first_mut(),
            &mut self.dirty,
        );
        self.queue_syntax_update();
        if self.views[view_id].clamp_cursor {
            self.center_on_cursor(view_id);
        }
    }

    pub fn copy(&mut self, view_id: ViewId) {
        self.views[view_id].coalesce_cursors();
        let multiple_cursors = self.views[view_id].cursors.len() > 1;
        let mut text = String::new();
        for i in 0..self.views[view_id].cursors.len() {
            let start = self.views[view_id].cursors[i]
                .position
                .min(self.views[view_id].cursors[i].anchor);
            let end = self.views[view_id].cursors[i]
                .position
                .max(self.views[view_id].cursors[i].anchor);
            let copied = if start == end {
                self.rope.line(self.cursor_line_idx(view_id, i))
            } else {
                self.rope.byte_slice(start..end)
            };
            for chunk in copied.chunks() {
                text.push_str(chunk);
            }
            if copied.get_line_ending().is_none() && multiple_cursors {
                // TODO figure out if this should be done even on the last cursor
                text.push('\n');
            }
        }
        #[cfg(target_os = "linux")]
        clipboard::set_primary(text.clone());
        clipboard::set_contents(text);
    }

    // TODO make multicursor aware
    pub fn cut(&mut self, view_id: ViewId) {
        self.views[view_id].cursors.clear();
        self.history
            .begin(*self.views[view_id].cursors.first(), self.dirty);
        let mut start = self.views[view_id]
            .cursors
            .first()
            .position
            .min(self.views[view_id].cursors.first().anchor);
        let mut end = self.views[view_id]
            .cursors
            .first()
            .position
            .max(self.views[view_id].cursors.first().anchor);

        if start == end {
            start = self.rope.line_to_byte(self.rope.byte_to_line(start));
            end = self.rope.end_of_line_byte(self.rope.byte_to_line(end));
        }
        let cut = self.rope.byte_slice(start..end).to_string();
        clipboard::set_contents(cut);
        self.history.remove(&mut self.rope, start..end);

        self.views[view_id].cursors.first_mut().position = start;
        self.views[view_id].cursors.first_mut().anchor =
            self.views[view_id].cursors.first().position;
        self.update_affinity(view_id);

        if start != end {
            self.mark_dirty();
            self.ensure_every_cursor_is_valid();
        }

        if self.views[view_id].clamp_cursor {
            self.center_on_cursor(view_id);
        }
        self.history.finish();
    }

    // TODO make this multicursor aware
    pub fn paste(&mut self, view_id: ViewId) {
        self.insert_text(view_id, &clipboard::get_contents(), true);
        self.history.finish();
    }

    pub fn paste_primary(&mut self, view_id: ViewId, col: usize, line: usize) {
        self.views[view_id].cursors.clear();
        self.set_cursor_pos(view_id, 0, col, line);
        self.insert_text(view_id, &clipboard::get_primary(), true);
        self.history.finish();
    }

    // TODO make this not use eof
    pub fn trim_start(&mut self, view_id: ViewId) {
        let mut start_white_spaces = 0;
        for ch in self.rope.chars() {
            if ch.is_whitespace() {
                start_white_spaces += 1;
            } else {
                break;
            }
        }
        let byte_end = self.rope.char_to_byte(start_white_spaces);
        self.history.remove(&mut self.rope, 0..byte_end);
        self.eof(view_id, false);
    }

    // TODO make this multicursor aware
    pub fn replace(&mut self, view_id: ViewId, byte_range: Range<usize>, text: &str) {
        self.views[view_id].cursors.clear();
        self.history
            .begin(*self.views[view_id].cursors.first(), self.dirty);
        let (cursor_col, cursor_line) = self.cursor_pos(view_id, 0);
        let (anchor_col, anchor_line) = self.anchor_pos(view_id, 0);
        self.history.replace(&mut self.rope, byte_range, text);
        self.set_cursor_pos(view_id, 0, cursor_col, cursor_line);
        self.set_anchor_pos(view_id, 0, anchor_col, anchor_line);
        self.ensure_cursor_is_valid(view_id);
        self.history.finish();
    }

    pub fn reload(&mut self) -> Result<(), BufferError> {
        let Some(path) = &self.file else {
            return Err(BufferError::NoPathSet);
        };
        self.history.finish();

        let (encoding, rope) = read::read_from_file(path)?;
        self.encoding = encoding;
        let len_bytes = self.rope.len_bytes();
        self.history.replace(&mut self.rope, 0..len_bytes, rope);

        for view in self.views.values_mut() {
            view.coalesce_cursors();
        }

        self.dirty = false;
        self.history.save();
        self.queue_syntax_update();

        self.history.finish();

        self.ensure_every_cursor_is_valid();
        Ok(())
    }

    pub fn escape(&mut self, view_id: ViewId) {
        if self.views[view_id].searcher.is_some() || self.views[view_id].replacement.is_some() {
            self.views[view_id].searcher = None;
            self.views[view_id].replacement = None;
            return;
        }

        if self.views[view_id].cursors.len() > 1 {
            self.views[view_id].cursors.clear();
            return;
        }

        if self.views[view_id].cursors.first().has_selection() {
            self.views[view_id].cursors.first_mut().anchor =
                self.views[view_id].cursors.first().position;
            if self.views[view_id].clamp_cursor {
                self.center_on_cursor(view_id);
            }
        }
    }

    pub fn handle_click(&mut self, view_id: ViewId, col: usize, line: usize) {
        self.views[view_id].cursors.clear();
        self.set_cursor_pos(view_id, 0, col, line);
        let click_point = Point::new(col, line);
        let now = Instant::now();
        if now.duration_since(self.views[view_id].last_click) < Duration::from_millis(500)
            && click_point == self.views[view_id].last_click_pos
        {
            self.views[view_id].clicks_in_a_row += 1;
            if self.views[view_id].clicks_in_a_row == 1 {
                self.select_word(view_id);
                self.copy_selection_to_primary(view_id);
            } else if self.views[view_id].clicks_in_a_row == 2 {
                self.select_line(view_id);
                self.copy_selection_to_primary(view_id);
            } else {
                self.views[view_id].clicks_in_a_row = 0;
            }
        } else {
            self.views[view_id].clicks_in_a_row = 0;
        }
        self.views[view_id].last_click = now;
        self.views[view_id].last_click_pos = click_point;
    }

    pub fn set_cursor_pos(
        &mut self,
        view_id: ViewId,
        cursor_index: usize,
        col: usize,
        line: usize,
    ) {
        self.views[view_id].cursors.clear();
        let line_idx: usize = line.min(self.rope.len_lines().saturating_sub(1));

        let next_line = self.rope.line_without_line_ending(line_idx);
        let next_width = next_line.width(0);
        let next_line_start = self.rope.line_to_byte(line_idx);

        if next_width < col {
            self.views[view_id].cursors[cursor_index].position =
                next_line_start + next_line.len_bytes();
        } else {
            let idx = next_line.nth_next_grapheme_boundary_byte(0, col);
            self.views[view_id].cursors[cursor_index].position = next_line_start + idx;
        }
        self.views[view_id].cursors[cursor_index].anchor =
            self.views[view_id].cursors[cursor_index].position;

        if self.views[view_id].clamp_cursor {
            self.center_on_cursor(view_id);
        }
    }

    pub fn set_anchor_pos(
        &mut self,
        view_id: ViewId,
        cursor_index: usize,
        col: usize,
        line: usize,
    ) {
        let line_idx: usize = line.min(self.rope.len_lines().saturating_sub(1));

        let next_line = self.rope.line_without_line_ending(line_idx);
        let next_width = next_line.width(0);
        let next_line_start = self.rope.line_to_byte(line_idx);

        if next_width < col {
            self.views[view_id].cursors[cursor_index].anchor =
                next_line_start + next_line.len_bytes();
        } else {
            let idx = next_line.nth_next_grapheme_boundary_byte(0, col);
            self.views[view_id].cursors[cursor_index].anchor = next_line_start + idx;
        }
    }

    pub fn select_area(
        &mut self,
        view_id: ViewId,
        cursor: Point<usize>,
        anchor: Point<usize>,
        copy_to_clipboard: bool,
    ) {
        self.views[view_id].cursors.clear();
        self.set_cursor_pos(view_id, 0, cursor.column, cursor.line);
        self.set_anchor_pos(view_id, 0, anchor.column, anchor.line);

        if copy_to_clipboard {
            self.copy_selection_to_primary(view_id);
        }
        self.update_affinity(view_id);
        self.history.finish();
    }

    pub fn copy_selection_to_primary(&mut self, view_id: ViewId) {
        #[cfg(target_os = "linux")]
        {
            self.views[view_id].cursors.clear();
            let start = self.views[view_id]
                .cursors
                .first()
                .position
                .min(self.views[view_id].cursors.first().anchor);
            let end = self.views[view_id]
                .cursors
                .first()
                .position
                .max(self.views[view_id].cursors.first().anchor);
            clipboard::set_primary(self.rope.byte_slice(start..end).to_string());
        }
    }

    pub fn center_on_cursor(&mut self, view_id: ViewId) {
        let cursor_index = self.views[view_id].cursors.len().saturating_sub(1);
        {
            let cursor_line = self
                .rope
                .byte_to_line(self.views[view_id].cursors[cursor_index].position);
            let start_line = self.views[view_id].line_pos;
            let end_line = self.views[view_id].line_pos + self.views[view_id].view_lines;
            if cursor_line < start_line || cursor_line >= end_line {
                self.views[view_id].line_pos =
                    cursor_line.saturating_sub(self.views[view_id].view_lines / 2);
            }
        }

        {
            let cursor_col = self.cursor_grapheme_column(view_id, cursor_index);
            let start_col = self.views[view_id].col_pos;
            let end_col = self.views[view_id].col_pos + self.views[view_id].view_columns;

            if cursor_col <= start_col {
                self.horizontal_scroll(view_id, -((start_col - cursor_col) as i64));
            } else if cursor_col >= end_col {
                self.horizontal_scroll(view_id, (cursor_col - end_col + 1) as i64);
            }
        }
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
        self.last_edit = Instant::now();
        self.queue_syntax_update();
    }

    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }

    pub fn get_last_edit(&self) -> Instant {
        self.last_edit
    }

    pub fn get_last_interact(&self) -> Instant {
        self.last_interact
    }

    pub fn update_interact(&mut self) {
        self.last_interact = Instant::now();
    }

    pub fn queue_syntax_update(&mut self) {
        if let Some(syntax) = &mut self.syntax {
            syntax.update_text(self.rope.clone());
        }
    }

    pub fn get_syntax(&mut self) -> Option<&mut Syntax> {
        self.syntax.as_mut()
    }

    pub fn view_range(&self, view_id: ViewId) -> Range<usize> {
        let start = self.rope.line_to_byte(self.views[view_id].line_pos);
        let end = self
            .rope
            .try_line_to_byte(self.views[view_id].line_pos + self.views[view_id].view_lines)
            .unwrap_or_else(|_| self.rope.len_bytes());
        start..end
    }

    pub fn start_search(
        &mut self,
        view_id: ViewId,
        proxy: Box<dyn EventLoopProxy>,
        query: String,
        case_insensitive: bool,
    ) {
        let cursor_pos = self.views[view_id].cursors.first().position;
        if let Some(searcher) = &mut self.views[view_id].searcher {
            searcher.update_query(query, case_insensitive, cursor_pos);
        } else {
            let searcher = BufferSearcher::new(
                proxy,
                query,
                self.rope.clone(),
                case_insensitive,
                self.views[view_id].cursors.first().position,
            );
            self.views[view_id].searcher = Some(searcher);
        }
    }

    pub fn get_searcher(&self, view_id: ViewId) -> Option<&BufferSearcher> {
        self.views[view_id].searcher.as_ref()
    }

    pub fn next_match(&mut self, view_id: ViewId) {
        if let Some(searcher) = &mut self.views[view_id].searcher {
            if let Some(search_match) = searcher.get_next_match() {
                self.select_area(view_id, search_match.end, search_match.start, false);
            }
        }
    }

    pub fn prev_match(&mut self, view_id: ViewId) {
        if let Some(searcher) = &mut self.views[view_id].searcher {
            if let Some(search_match) = searcher.get_prev_match() {
                self.select_area(view_id, search_match.end, search_match.start, false);
            }
        }
    }

    pub fn cursor_is_eof(&self, view_id: ViewId, cursor_index: usize) -> bool {
        self.views[view_id].cursors[cursor_index].position == self.rope.len_bytes()
    }

    pub fn revert_buffer(&mut self, view_id: ViewId) {
        while self.dirty {
            self.undo(view_id);
        }
    }

    pub fn move_to_trash(&self) -> Result<bool, trash::Error> {
        if let Some(path) = &self.file {
            trash::delete(path)?;
            return Ok(true);
        }

        Ok(false)
    }

    pub fn get_selection(&self, view_id: ViewId, cursor_index: usize) -> String {
        let start = self.views[view_id].cursors[cursor_index]
            .anchor
            .min(self.views[view_id].cursors[cursor_index].position);
        let end = self.views[view_id].cursors[cursor_index]
            .anchor
            .max(self.views[view_id].cursors[cursor_index].position);
        let slice = self.rope.byte_slice(start..end);
        slice.to_string()
    }

    pub fn mark_history_dirty(&mut self) {
        self.history.mark_all_dirty();
    }

    pub fn mark_saved(&mut self) {
        self.dirty = false;
        self.history.save();
    }

    pub fn len_bytes(&self) -> usize {
        self.rope.len_bytes()
    }

    pub fn ensure_cursor_is_valid(&mut self, view_id: ViewId) {
        let num_cursors = self.views[view_id].cursors.len();
        for i in 0..num_cursors {
            self.views[view_id].cursors[i].position = self.views[view_id].cursors[i]
                .position
                .min(self.rope.len_bytes());
            self.views[view_id].cursors[i].anchor = self.views[view_id].cursors[i]
                .anchor
                .min(self.rope.len_bytes());

            {
                let view = &mut self.views[view_id];
                while view.cursors[i].position != 0
                    && view.cursors[i].position != self.rope.len_bytes()
                    && !is_utf8_char_boundary(self.rope.byte(view.cursors[i].position))
                {
                    view.cursors[i].position = view.cursors[i].position.saturating_sub(1);
                }
                while view.cursors[i].anchor != 0
                    && view.cursors[i].anchor != self.rope.len_bytes()
                    && !is_utf8_char_boundary(self.rope.byte(view.cursors[i].anchor))
                {
                    view.cursors[i].anchor = view.cursors[i].anchor.saturating_sub(1);
                }
            }

            self.views[view_id].cursors[i].position = self
                .rope()
                .ensure_grapheme_boundary_next_byte(self.views[view_id].cursors[i].position);
            self.views[view_id].cursors[i].anchor = self
                .rope()
                .ensure_grapheme_boundary_next_byte(self.views[view_id].cursors[i].anchor);
        }
    }

    pub fn guess_indent(&self, byte_index: usize) -> String {
        let line_idx = self.rope.byte_to_line(byte_index);
        let line = self.rope.line(line_idx);

        let mut indent = String::new();
        for grapheme in line.grapehemes() {
            if !grapheme.is_whitespace() {
                break;
            }
            indent.extend(grapheme.chunks());
        }

        self.indent.from_width(Rope::from_str(&indent).width(0))
    }

    pub fn sort_lines(&mut self, view_id: ViewId, asc: bool) {
        if self.views[view_id].cursors.len() > 1 {
            return;
        }

        self.history
            .begin(*self.views[view_id].cursors.first(), self.dirty);
        let start = self.rope.byte_to_line(
            self.views[view_id]
                .cursors
                .first()
                .position
                .min(self.views[view_id].cursors.first().anchor),
        );
        let end = self.rope.byte_to_line(
            self.views[view_id]
                .cursors
                .first()
                .position
                .max(self.views[view_id].cursors.first().anchor),
        );

        let last_line_at_start = self.views[view_id]
            .cursors
            .first()
            .position
            .max(self.views[view_id].cursors.first().anchor)
            == self.rope.line_to_byte(end);

        let end = if last_line_at_start {
            end.saturating_sub(1).max(start)
        } else {
            end
        };

        if end == start {
            return;
        }

        let start_byte = self.rope.line_to_byte(start);
        let end_byte = self.rope.end_of_line_byte(end);

        let cloned_rope = self.rope.clone();
        let mut lines: Vec<RopeSlice> = cloned_rope
            .byte_slice(start_byte..end_byte)
            .lines()
            .collect();
        lines.sort_by(|lhs, rhs| {
            let lhs = lhs.trim_start_whitespace();
            let rhs = rhs.trim_start_whitespace();
            for (lhs, rhs) in lhs.chunks().zip(rhs.chunks()) {
                match lexical_sort::natural_lexical_cmp(lhs, rhs) {
                    cmp::Ordering::Equal => continue,
                    ordering if asc => return ordering.reverse(),
                    ordering => return ordering,
                }
            }
            cmp::Ordering::Equal
        });

        let cursor_line = self.cursor_line_idx(view_id, 0);
        let anchor_line = self.anchor_line_idx(view_id, 0);
        let cursor_col = self.cursor_grapheme_column(view_id, 0);
        let anchor_col = self.anchor_grapheme_column(view_id, 0);

        self.history.remove(&mut self.rope, start_byte..end_byte);
        let inserted_bytes = 0;
        for line in lines {
            self.history
                .insert(&mut self.rope, start_byte + inserted_bytes, line);
        }

        self.set_cursor_pos(view_id, cursor_col, cursor_line, 0);
        self.set_anchor_pos(view_id, anchor_col, anchor_line, 0);

        self.ensure_cursor_is_valid(view_id);
        self.mark_dirty();
        self.ensure_every_cursor_is_valid();

        if self.views[view_id].clamp_cursor {
            self.center_on_cursor(view_id);
        }

        self.history.finish();
    }

    pub fn replace_all(&mut self, view_id: ViewId, replacement: String) {
        let view = &mut self.views[view_id];
        if let Some(searcher) = &mut view.searcher {
            self.history.begin(*view.cursors.first(), self.dirty);
            let matches = searcher.get_matches();
            let guard = matches.lock().unwrap();
            let (matches, _) = &*guard;

            let mut diff: i64 = 0;
            for m in matches {
                let start_byte_idx = (m.start_byte as i64 + diff) as usize;
                let end_byte_idx = (m.end_byte as i64 + diff) as usize;
                self.history
                    .replace(&mut self.rope, start_byte_idx..end_byte_idx, &replacement);
                let match_len = (end_byte_idx - start_byte_idx) as i64;
                let replacement_diff = replacement.len() as i64 - match_len;
                diff += replacement_diff;

                for cursor in &mut *view.cursors {
                    if cursor.position > start_byte_idx {
                        cursor.position = (cursor.position as i64 + replacement_diff) as usize;
                    }

                    if cursor.anchor > start_byte_idx {
                        cursor.anchor = (cursor.anchor as i64 + replacement_diff) as usize;
                    }
                }
            }

            searcher.update_buffer(self.rope.clone(), None);

            self.ensure_cursor_is_valid(view_id);
            self.mark_dirty();
            self.ensure_every_cursor_is_valid();

            if self.views[view_id].clamp_cursor {
                self.center_on_cursor(view_id);
            }

            self.history.finish();
        }
    }

    pub fn is_disposable(&self) -> bool {
        !self.is_dirty()
            && self.rope().len_bytes() == 0
            && self.views.is_empty()
            && self.file.is_none()
    }

    pub fn get_next_file(&self) -> Result<PathBuf, anyhow::Error> {
        let Some(file) = &self.file else {
            anyhow::bail!("Cannot rotate buffer has no path");
        };

        if file.extension().is_none() {
            anyhow::bail!("Cannot rotate buffer has no file extension");
        };

        let Some(name) = file.file_name() else {
            anyhow::bail!("Cannot rotate buffer has no file name");
        };
        let current_file_name = name.to_string_lossy();

        let Some(stem) = file.file_stem() else {
            anyhow::bail!("Cannot rotate buffer has no file name");
        };
        let current_file_stem = stem.to_string_lossy();

        let Some(parent) = file.parent() else {
            anyhow::bail!("Cannot rotate path has no parent directory");
        };

        let mut entries = Vec::new();

        for entry in fs::read_dir(parent)? {
            let Ok(entry) = entry else {
                continue;
            };

            let path = entry.path();

            let Some(stem) = path.file_stem() else {
                continue;
            };

            let Some(name) = path.file_name() else {
                continue;
            };

            let stem = stem.to_string_lossy();
            let name: String = name.to_string_lossy().into();

            if stem == current_file_stem {
                entries.push((name, path));
            }
        }

        entries.sort_by(|(a, _), (b, _)| a.cmp(b));

        let index = entries
            .iter()
            .position(|(name, _)| *name == current_file_name)
            .unwrap_or_default();

        Ok(entries[(index + 1) % entries.len()].1.clone())
    }

    pub fn replace_current_match(&mut self, view_id: ViewId) {
        let view = &mut self.views[view_id];
        if let (Some(searcher), Some(replacement)) = (&mut view.searcher, view.replacement.clone())
        {
            if let Some(search_match) = searcher.get_current_match() {
                self.select_area(view_id, search_match.end, search_match.start, false);
                self.insert_text(view_id, &replacement, false);
            } else {
                searcher.get_next_match();
            }
        }
    }

    pub fn load_view_data(&mut self, view_id: ViewId, buffer_data: &BufferData) {
        let cursor = buffer_data.cursor;
        let line_pos = buffer_data.line_pos;
        self.vertical_scroll(view_id, line_pos as i64);
        let postion = self
            .rope()
            .byte_to_point(cursor.position.min(self.len_bytes()));
        let anchor = self
            .rope()
            .byte_to_point(cursor.anchor.min(self.len_bytes()));
        self.set_cursor_pos(view_id, 0, postion.column, postion.line);
        self.set_anchor_pos(view_id, 0, anchor.column, anchor.line);
        self.ensure_cursor_is_valid(view_id);
    }

    pub fn load_buffer_data(&mut self, buffer_data: &BufferData) {
        if let Err(err) = self.set_langauge(&buffer_data.language, get_buffer_proxy()) {
            tracing::error!("Error loading buffer data: {err}");
        }
        self.indent = buffer_data.indent;
    }

    pub fn create_view(&mut self) -> ViewId {
        self.views.insert(View::default())
    }

    pub fn get_first_view(&self) -> Option<ViewId> {
        self.views.keys().next()
    }

    pub fn get_first_view_or_create(&mut self) -> ViewId {
        self.views
            .keys()
            .next()
            .unwrap_or_else(|| self.create_view())
    }

    pub fn remove_view(&mut self, view_id: ViewId) {
        self.views.remove(view_id);
    }

    pub fn ensure_every_cursor_is_valid(&mut self) {
        let view_ids = self.views.keys().collect::<Vec<_>>();
        for view_id in view_ids {
            self.ensure_cursor_is_valid(view_id);
            let view = &mut self.views[view_id];
            view.line_pos = self.rope.len_lines().min(view.line_pos);
        }
    }
}

pub struct ViewLine<'a> {
    pub text: RopeSlice<'a>,
    pub col_start_offset: usize,
    pub text_start_col: usize,
    pub text_end_col: usize,
}

pub struct BufferView<'a> {
    pub lines: Vec<ViewLine<'a>>,
}

#[derive(Debug, PartialEq, Eq)]
enum Skipping {
    Whitespace,
    WordChar,
    Other,
    None,
}

/// Copied from core internals
#[inline]
const fn is_utf8_char_boundary(byte: u8) -> bool {
    // This is bit magic equivalent to: b < 128 || b >= 192
    (byte as i8) >= -0x40
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn insert_random_ascii() {
        for _ in 0..100 {
            use rand::Rng;
            fn get_random_text() -> String {
                let mut rng = rand::thread_rng();
                let mut output = Vec::new();
                for _ in 0..rng.gen_range(0..100) {
                    output.push(rng.gen_range(0..128));
                }
                unsafe { String::from_utf8_unchecked(output) }
            }

            let mut rng = rand::thread_rng();
            let mut buffer = Buffer::new();
            let view_id = buffer.get_first_view_or_create();

            for _ in 0..1000 {
                match rng.gen_range(0..5) {
                    0 => {
                        buffer.move_left_char(view_id, false);
                    }
                    1 => {
                        buffer.move_left_char(view_id, false);
                    }
                    2 => {
                        buffer.move_up(view_id, false, 0);
                    }
                    3 => {
                        buffer.move_down(view_id, false, 0);
                    }
                    4 => {
                        let text = get_random_text();
                        buffer.insert_text(view_id, &text, false);
                    }
                    _ => unreachable!(),
                }
            }
        }
    }
}
