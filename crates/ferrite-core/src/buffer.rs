use core::fmt;
use std::{
    cmp, io,
    num::NonZeroUsize,
    ops::Range,
    path::{Path, PathBuf},
    sync::OnceLock,
    time::Instant,
};

use encoding_rs::Encoding;
use ferrite_utility::{
    graphemes::RopeGraphemeExt as _,
    line_ending::{rope_end_without_line_ending, LineEnding, DEFAULT_LINE_ENDING},
    point::Point,
};
use ropey::{Rope, RopeSlice};
use serde::{Deserialize, Serialize};

use self::{error::BufferError, history::History, search::BufferSearcher};
use super::{
    indent::Indentation,
    language::{get_language_from_path, syntax::Syntax},
};
use crate::{clipboard, event_loop_proxy::EventLoopProxy, keymap::LineMoveDir};

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

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
pub struct Cursor {
    pub anchor: usize,
    pub position: usize,
    pub affinity: usize,
}

impl Cursor {
    pub fn has_selection(&self) -> bool {
        self.position != self.anchor
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Selection {
    pub start: Point<i64>,
    pub end: Point<i64>,
}

pub struct Buffer {
    cursor: Cursor,
    line_pos: usize,
    col_pos: usize,
    rope: Rope,
    file: Option<PathBuf>,
    dirty: bool,
    read_only: bool,
    last_edit: Instant,
    pub line_ending: LineEnding,
    pub encoding: &'static Encoding,
    pub indent: Indentation,
    pub clamp_cursor: bool,
    // view stuff
    view_lines: usize,
    view_columns: usize,
    // syntax highlight
    syntax: Option<Syntax>,
    history: History,
    // file searching
    searcher: Option<BufferSearcher>,
}

impl Default for Buffer {
    fn default() -> Self {
        Self {
            cursor: Cursor::default(),
            line_pos: 0,
            col_pos: 0,
            rope: Rope::new(),
            file: None,
            encoding: encoding_rs::UTF_8,
            indent: Indentation::Tabs(NonZeroUsize::new(1).unwrap()),
            dirty: false,
            last_edit: Instant::now(),
            read_only: false,
            line_ending: DEFAULT_LINE_ENDING,
            clamp_cursor: true,
            view_lines: 10000,
            view_columns: 10000,
            syntax: None,
            history: History::default(),
            searcher: None,
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

    pub fn with_path(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let mut syntax = Syntax::new(get_buffer_proxy());
        if let Some(language) = get_language_from_path(&path) {
            if let Err(err) = syntax.set_language(language) {
                tracing::error!("Error setting language: {err}");
            }
            syntax.update_text(Rope::new());
        }

        Self {
            file: Some(path),
            syntax: Some(syntax),
            ..Default::default()
        }
    }

    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, io::Error> {
        let path = path.as_ref();
        let (encoding, rope) = read::read_from_file(path)?;

        let mut syntax = Syntax::new(get_buffer_proxy());
        if let Some(language) = get_language_from_path(path) {
            if let Err(err) = syntax.set_language(language) {
                tracing::error!("Error setting language: {err}");
            }
            syntax.update_text(rope.clone());
        }

        Ok(Self {
            indent: Indentation::detect_indent_rope(rope.slice(..)),
            rope,
            file: Some(path.into()),
            encoding,
            syntax: Some(syntax),
            ..Default::default()
        })
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, io::Error> {
        let (encoding, rope) = read::read(bytes)?;
        let syntax = Syntax::new(get_buffer_proxy());

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
    }

    #[allow(dead_code)]
    pub fn cursor(&self) -> Cursor {
        self.cursor
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn set_view_lines(&mut self, lines: usize) {
        self.view_lines = lines;
    }

    pub fn get_view_lines(&self) -> usize {
        self.view_lines
    }

    pub fn set_view_columns(&mut self, cols: usize) {
        self.view_columns = cols;
    }

    pub fn _get_view_columns(&self) -> usize {
        self.view_columns
    }

    pub fn name(&self) -> Option<String> {
        Some(
            self.file
                .as_ref()?
                .file_name()?
                .to_string_lossy()
                .to_string(),
        )
    }

    pub fn language_name(&self) -> String {
        match &self.syntax {
            Some(syntax) => syntax
                .get_language_name()
                .unwrap_or_else(|| String::from("text")),
            None => String::from("text"),
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

    pub fn get_buffer_view(&self) -> BufferView {
        let end_line = cmp::min(self.rope.len_lines(), self.view_lines + self.line_pos);

        let mut lines = Vec::new();
        for line_idx in self.line_pos..end_line {
            let Some(line) = self.rope.get_line(line_idx) else {
                break;
            };
            let mut idx = 0;
            let mut width = 0;
            for grapheme in line.grapehemes() {
                if width >= self.col_pos {
                    break;
                }
                width += grapheme.width(width);
                idx += grapheme.len_bytes();
            }
            let line = line.byte_slice(idx..);
            lines.push(ViewLine {
                text: line,
                col_start_offset: width.saturating_sub(self.col_pos),
                text_start_col: self.rope.get_text_start_col(line_idx),
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

    pub fn set_file(&mut self, path: impl Into<PathBuf>) {
        self.file = Some(path.into());
    }

    pub fn line_pos(&self) -> usize {
        self.line_pos
    }

    pub fn col_pos(&self) -> usize {
        self.col_pos
    }

    pub fn len_lines(&self) -> usize {
        self.rope.len_lines()
    }

    pub fn cursor_view_pos(&self, max_lines: usize) -> Option<(usize, usize)> {
        let start_line = self.line_pos;
        let end_line = std::cmp::min(self.rope.len_lines(), max_lines + self.line_pos);

        let (column, line) = self.cursor_pos();

        if line >= start_line && line < end_line {
            Some((column, line - start_line))
        } else {
            None
        }
    }

    pub fn get_view_selection(&self) -> Selection {
        let pos = Point {
            line: self.cursor_line_idx() as i64,
            column: self.cursor_grapheme_column() as i64,
        };

        let anchor = Point {
            line: self.anchor_line_idx() as i64,
            column: self.anchor_grapheme_column() as i64,
        };

        let mut start = pos.min(anchor);
        let mut end = pos.max(anchor);
        start.line -= self.line_pos as i64;
        end.line -= self.line_pos as i64;
        start.column -= self.col_pos as i64;
        end.column -= self.col_pos as i64;

        Selection { start, end }
    }

    pub fn cursor_line_idx(&self) -> usize {
        self.rope.byte_to_line(self.cursor.position)
    }

    pub fn anchor_line_idx(&self) -> usize {
        self.rope.byte_to_line(self.cursor.anchor)
    }

    pub fn cursor_pos(&self) -> (usize, usize) {
        let current_line = self.cursor_line_idx();
        let start_of_line = self.rope.line_to_byte(current_line);
        let column = self.cursor.position - start_of_line;

        (column, current_line)
    }

    pub fn anchor_pos(&self) -> (usize, usize) {
        let current_line = self.anchor_line_idx();
        let start_of_line = self.rope.line_to_byte(current_line);
        let column = self.cursor.anchor - start_of_line;

        (column, current_line)
    }

    pub fn cursor_grapheme_column(&self) -> usize {
        let (column_idx, line_idx) = self.cursor_pos();
        let line = self.rope.line(line_idx);
        let start = line.byte_slice(..column_idx);
        start.width(0)
    }

    pub fn anchor_grapheme_column(&self) -> usize {
        let (column_idx, line_idx) = self.anchor_pos();
        let line = self.rope.line(line_idx);
        let start = line.byte_slice(..column_idx);
        start.width(0)
    }

    pub fn update_affinity(&mut self) {
        let (column_idx, line_idx) = self.cursor_pos();
        self.cursor.affinity = self
            .rope
            .line_without_line_ending(line_idx)
            .byte_slice(..column_idx)
            .width(0);
    }

    pub fn vertical_scroll(&mut self, distance: i64) {
        self.line_pos = (self.line_pos as i128 + distance as i128)
            .clamp(0, self.len_lines() as i128 - 1) as usize;
    }

    pub fn horizontal_scroll(&mut self, distance: i64) {
        self.col_pos =
            (self.col_pos as i128 + distance as i128).clamp(0, usize::MAX as i128 - 1) as usize;
    }

    pub fn move_right_char(&mut self, shift: bool) {
        let new_idx = self.rope.next_grapheme_boundary_byte(self.cursor.position);
        self.cursor.position = new_idx;

        if !shift {
            if self.cursor.anchor > self.cursor.position {
                self.cursor.position = self.cursor.anchor;
            } else {
                self.cursor.anchor = self.cursor.position;
            }
        }

        self.update_affinity();

        if self.clamp_cursor {
            self.center_on_cursor();
        }
    }

    pub fn move_left_char(&mut self, shift: bool) {
        let new_idx = self.rope.prev_grapheme_boundary_byte(self.cursor.position);
        self.cursor.position = new_idx;

        if !shift {
            if self.cursor.anchor < self.cursor.position {
                self.cursor.position = self.cursor.anchor;
            } else {
                self.cursor.anchor = self.cursor.position;
            }
        }

        self.update_affinity();

        if self.clamp_cursor {
            self.center_on_cursor();
        }
    }

    pub fn move_down(&mut self, shift: bool, distance: usize) {
        let (column_idx, line_idx) = self.cursor_pos();
        let new_line_idx = (line_idx + distance).min(self.rope.len_lines().saturating_sub(1));
        if line_idx == new_line_idx {
            return;
        }

        let before_cursor = self
            .rope
            .line_without_line_ending(line_idx)
            .byte_slice(..column_idx)
            .width(0)
            .max(self.cursor.affinity);
        let next_line = self.rope.line_without_line_ending(new_line_idx);
        let next_width = next_line.width(0);
        let next_line_start = self.rope.line_to_byte(new_line_idx);

        if next_width < before_cursor {
            self.cursor.position = next_line_start + next_line.len_bytes();
        } else {
            let idx = next_line.nth_next_grapheme_boundary_byte(0, before_cursor);
            self.cursor.position = next_line_start + idx;
        }

        if !shift {
            self.cursor.anchor = self.cursor.position;
        }

        if self.clamp_cursor {
            self.center_on_cursor();
        }
    }

    pub fn move_up(&mut self, shift: bool, distance: usize) {
        let (column_idx, line_idx) = self.cursor_pos();
        if line_idx == 0 {
            return;
        }
        let new_line_idx = line_idx.saturating_sub(distance);

        let before_cursor = self
            .rope
            .line_without_line_ending(line_idx)
            .byte_slice(..column_idx)
            .width(0)
            .max(self.cursor.affinity);
        let next_line = self.rope.line_without_line_ending(new_line_idx);
        let next_width = next_line.width(0);
        let next_line_start = self.rope.line_to_byte(new_line_idx);

        if next_width < before_cursor {
            self.cursor.position = next_line_start + next_line.len_bytes();
        } else {
            let idx = next_line.nth_next_grapheme_boundary_byte(0, before_cursor);
            self.cursor.position = next_line_start + idx;
        }

        if !shift {
            self.cursor.anchor = self.cursor.position;
        }

        if self.clamp_cursor {
            self.center_on_cursor();
        }
    }

    pub fn select_word(&mut self) {
        // TODO add matching multi selection when already having a selection
        if !self.cursor.has_selection() {
            let mut start_byte_idx = self.cursor.position;
            loop {
                let new_idx = self.rope.prev_grapheme_boundary_byte(start_byte_idx);
                let grapheme = self.rope.byte_slice(new_idx..start_byte_idx);
                if new_idx == start_byte_idx || !grapheme.is_word_char() {
                    break;
                }
                start_byte_idx = new_idx;
            }

            let mut end_byte_idx = self.cursor.position;
            loop {
                let new_idx = self.rope.next_grapheme_boundary_byte(end_byte_idx);
                let grapheme = self.rope.byte_slice(end_byte_idx..new_idx);
                if new_idx == end_byte_idx || !grapheme.is_word_char() {
                    break;
                }
                end_byte_idx = new_idx;
            }

            self.cursor.position = end_byte_idx;
            self.cursor.anchor = start_byte_idx;
        }
    }

    fn next_word_end(&self) -> usize {
        let mut current_idx = self.cursor.position;
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
                        Skipping::WordChar
                    } else if grapheme.is_whitespace() {
                        if grapheme.get_line_ending().is_some() {
                            break;
                        }
                        Skipping::Whitespace
                    } else {
                        Skipping::Other
                    };
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

    fn prev_word_start(&self) -> usize {
        let mut current_idx = self.cursor.position;
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
                        Skipping::WordChar
                    } else if grapheme.is_whitespace() {
                        if grapheme.get_line_ending().is_some() {
                            break;
                        }
                        Skipping::Whitespace
                    } else {
                        Skipping::Other
                    };
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

    pub fn move_right_word(&mut self, shift: bool) {
        let next_word = self.next_word_end();
        self.cursor.position = next_word;

        if !shift {
            self.cursor.anchor = self.cursor.position;
        }

        self.update_affinity();

        if self.clamp_cursor {
            self.center_on_cursor();
        }
    }

    pub fn move_left_word(&mut self, shift: bool) {
        let prev_word = self.prev_word_start();
        self.cursor.position = prev_word;

        if !shift {
            self.cursor.anchor = self.cursor.position;
        }

        self.update_affinity();

        if self.clamp_cursor {
            self.center_on_cursor();
        }
    }

    /// Move cursor to line. Line is indexed from 1
    pub fn goto(&mut self, line: i64) {
        let line_idx = (self.rope.len_lines().saturating_sub(1) as i64)
            .min(line)
            .max(0) as usize;

        self.set_cursor_pos(0, line_idx);
    }

    pub fn home(&mut self, shift: bool) {
        let (col, line_idx) = self.cursor_pos();
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
        self.cursor.position = byte;
        if !shift {
            self.cursor.anchor = self.cursor.position;
        }
        self.update_affinity();

        if self.clamp_cursor {
            self.center_on_cursor();
        }
    }

    pub fn end(&mut self, shift: bool) {
        let line_idx = self.cursor_line_idx();
        let byte = self.rope.line_to_byte(line_idx);
        let line_len = self.rope.line_without_line_ending(line_idx).len_bytes();
        self.cursor.position = byte + line_len;
        if !shift {
            self.cursor.anchor = self.cursor.position;
        }
        self.update_affinity();

        if self.clamp_cursor {
            self.center_on_cursor();
        }
    }

    pub fn start(&mut self, shift: bool) {
        self.cursor.position = 0;
        if !shift {
            self.cursor.anchor = self.cursor.position;
        }
        self.update_affinity();

        if self.clamp_cursor {
            self.center_on_cursor();
        }
    }

    pub fn eof(&mut self, shift: bool) {
        self.cursor.position = self.rope.len_bytes();
        if !shift {
            self.cursor.anchor = self.cursor.position;
        }
        self.update_affinity();

        if self.clamp_cursor {
            self.center_on_cursor();
        }
    }

    pub fn insert_text(&mut self, text: &str) {
        // TODO collect multiple words/whitespace chars into single undo step
        self.history.begin(self.cursor, self.dirty);

        let auto_pair = matches!(text, "{" | "[" | "(" | "'" | "\"" | "`" | "<");

        if self.cursor.has_selection() {
            let start_byte_idx = self.cursor.position.min(self.cursor.anchor);
            let end_byte_idx = self.cursor.position.max(self.cursor.anchor);
            if auto_pair {
                self.history.insert(&mut self.rope, start_byte_idx, text);
                self.history.insert(&mut self.rope, end_byte_idx + 1, text);
                self.cursor.position = end_byte_idx;
                self.cursor.anchor = end_byte_idx;
            } else {
                self.history
                    .replace(&mut self.rope, start_byte_idx..end_byte_idx, text);
                self.cursor.position = self.cursor.position.min(self.cursor.anchor);
                self.cursor.anchor = self.cursor.position;
            }
        } else {
            self.history
                .insert(&mut self.rope, self.cursor.position, text);
        }

        self.cursor.position += text.len();
        self.cursor.anchor = self.cursor.position;

        self.update_affinity();
        self.mark_dirty();

        if self.clamp_cursor {
            self.center_on_cursor();
        }
        self.history.finish();
    }

    pub fn backspace(&mut self) {
        // this is a bit hacky but it works
        {
            let line_idx = self.cursor_line_idx();
            let line_byte = self.cursor.position - self.rope.line_to_byte(line_idx);
            if !self.cursor.has_selection()
                && line_byte <= self.rope.get_text_start_byte(line_idx)
                && line_byte != 0
            {
                // FIXME back tab does not move the cursor correctly when standing in the middle of the indentation
                self.tab(true);
                return;
            }
        }

        self.history.begin(self.cursor, self.dirty);
        let (start_byte_idx, end_byte_idx) = if !self.cursor.has_selection() {
            let start_byte_idx = self.rope.prev_grapheme_boundary_byte(self.cursor.position);

            //let start_byte = self.rope.get_byte(start_byte_idx);
            //let end_byte = self.rope.get_byte(start_byte_idx + 1);
            let end_byte_idx = self.cursor.position;

            // Remove pair
            /*
            let end_byte_idx = match (start_byte, end_byte) {
                (Some(b'{'), Some(b'}')) => self.cursor.position + 1,
                (Some(b'['), Some(b']')) => self.cursor.position + 1,
                (Some(b'('), Some(b')')) => self.cursor.position + 1,
                (Some(b'\''), Some(b'\'')) => self.cursor.position + 1,
                (Some(b'"'), Some(b'"')) => self.cursor.position + 1,
                _ => self.cursor.position,
            };*/

            (start_byte_idx, end_byte_idx)
        } else {
            let start_byte_idx = self.cursor.position.min(self.cursor.anchor);
            let end_byte_idx = self.cursor.position.max(self.cursor.anchor);
            (start_byte_idx, end_byte_idx)
        };

        self.history
            .remove(&mut self.rope, start_byte_idx..end_byte_idx);

        self.cursor.position = start_byte_idx;
        self.cursor.anchor = self.cursor.position;
        self.update_affinity();

        if start_byte_idx != end_byte_idx {
            self.mark_dirty();
        }

        if self.clamp_cursor {
            self.center_on_cursor();
        }
        self.history.finish();
    }

    pub fn backspace_word(&mut self) {
        if self.cursor.has_selection() {
            self.backspace();
            return;
        }

        self.history.begin(self.cursor, self.dirty);
        let prev_word = self.prev_word_start();
        self.history
            .remove(&mut self.rope, prev_word..self.cursor.position);

        if prev_word != self.cursor.position {
            self.mark_dirty();
        }

        self.cursor.position = prev_word;
        self.cursor.anchor = self.cursor.position;
        self.update_affinity();

        if self.clamp_cursor {
            self.center_on_cursor();
        }
        self.history.finish();
    }

    pub fn delete(&mut self) {
        self.history.begin(self.cursor, self.dirty);
        let (start_byte_idx, end_byte_idx) = if !self.cursor.has_selection() {
            let end_byte_idx = self.rope.next_grapheme_boundary_byte(self.cursor.position);
            (self.cursor.position, end_byte_idx)
        } else {
            let start_byte_idx = self.cursor.position.min(self.cursor.anchor);
            let end_byte_idx = self.cursor.position.max(self.cursor.anchor);
            (start_byte_idx, end_byte_idx)
        };

        self.history
            .remove(&mut self.rope, start_byte_idx..end_byte_idx);

        self.cursor.position = start_byte_idx;
        self.cursor.anchor = self.cursor.position;
        self.update_affinity();

        if start_byte_idx != end_byte_idx {
            self.mark_dirty();
        }

        if self.clamp_cursor {
            self.center_on_cursor();
        }
        self.history.finish();
    }

    pub fn delete_word(&mut self) {
        if self.cursor.has_selection() {
            self.delete();
            return;
        }

        self.history.begin(self.cursor, self.dirty);
        let next_word = self.next_word_end();

        self.history
            .remove(&mut self.rope, self.cursor.position..next_word);
        self.update_affinity();

        if self.cursor.position != next_word {
            self.mark_dirty();
        }

        if self.clamp_cursor {
            self.center_on_cursor();
        }
        self.history.finish();
    }

    pub fn new_line(&mut self) {
        self.history.begin(self.cursor, self.dirty);
        self.end(false);
        self.history
            .insert(&mut self.rope, self.cursor.position, "\n");
        self.cursor.position += 1;
        self.cursor.anchor = self.cursor.position;
        self.update_affinity();
        self.mark_dirty();

        if self.clamp_cursor {
            self.center_on_cursor();
        }
        self.history.finish();
    }

    pub fn move_line(&mut self, dir: LineMoveDir) {
        self.history.begin(self.cursor, self.dirty);
        let len_lines = self.rope.len_lines();
        let (cursor_col, cursor_line_idx) = self.cursor_pos();
        let (anchor_col, anchor_line_idx) = self.anchor_pos();

        let cursor_byte_idx_in_line =
            self.cursor.position - self.rope.line_to_byte(cursor_line_idx);
        let anchor_byte_idx_in_line = self.cursor.anchor - self.rope.line_to_byte(anchor_line_idx);

        let start_line_idx = cursor_line_idx.min(anchor_line_idx);
        let mut end_line_idx = cursor_line_idx.max(anchor_line_idx);

        let end_col = if self.cursor.position > self.cursor.anchor {
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

        let old_line_idx = self
            .rope
            .byte_to_line(self.cursor.position.min(self.cursor.anchor));
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

        self.cursor.position =
            self.rope.line_to_byte(new_cursor_line_idx) + cursor_byte_idx_in_line;
        self.cursor.anchor = self.rope.line_to_byte(new_anchor_line_idx) + anchor_byte_idx_in_line;

        self.update_affinity();
        self.mark_dirty();

        if self.clamp_cursor {
            self.center_on_cursor();
        }
    }

    pub fn tab(&mut self, back: bool) {
        // TODO optimize for larger files

        if !self.cursor.has_selection() && !back {
            let col = self.cursor_grapheme_column();
            self.insert_text(&self.indent.to_next_ident(col));
            return;
        }

        self.history.begin(self.cursor, self.dirty);
        {
            let cursor_col = self.cursor_grapheme_column();
            let anchor_col = self.anchor_grapheme_column();
            let cursor_line_idx = self.cursor_line_idx();
            let anchor_line_idx = self.anchor_line_idx();

            let start = self
                .rope
                .byte_to_line(self.cursor.position.min(self.cursor.anchor));
            let end = self
                .rope
                .byte_to_line(self.cursor.position.max(self.cursor.anchor));

            let last_line_at_start =
                self.cursor.position.max(self.cursor.anchor) == self.rope.line_to_byte(end);

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
                    self.cursor.position = self.rope.line_to_byte(cursor_line_idx);
                    if cursor_col < text_start_col || cursor_col == 0 {
                        self.set_cursor_col(cursor_col);
                    } else {
                        self.set_cursor_col((cursor_col as i64 + diff) as usize);
                    }
                }

                if line_idx == anchor_line_idx {
                    self.cursor.anchor = self.rope.line_to_byte(anchor_line_idx);
                    if anchor_col < text_start_col || anchor_col == 0 {
                        self.set_anchor_col(anchor_col);
                    } else {
                        self.set_anchor_col((anchor_col as i64 + diff) as usize);
                    }
                }
            }
        }

        self.update_affinity();
        self.mark_dirty();

        if self.clamp_cursor {
            self.center_on_cursor();
        }
        self.history.finish();
    }

    pub fn set_cursor_col(&mut self, col: usize) {
        let cursor_line_idx = self.cursor_line_idx();
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
        self.cursor.position = self.rope.line_to_byte(cursor_line_idx) + byte_idx;
    }

    pub fn set_anchor_col(&mut self, col: usize) {
        let anchor_line_idx = self.anchor_line_idx();
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
        self.cursor.anchor = self.rope.line_to_byte(anchor_line_idx) + byte_idx;
    }

    pub fn select_all(&mut self) {
        self.cursor.anchor = 0;
        self.cursor.position = self.rope.len_bytes();

        self.update_affinity();
        if self.clamp_cursor {
            self.center_on_cursor();
        }
    }

    pub fn select_line(&mut self) {
        {
            let line_idx = self.cursor_line_idx();
            let line_start = self.rope.line_to_byte(line_idx + 1);
            self.cursor.position = line_start;
        }

        {
            let line_idx = self.anchor_line_idx();
            let line_start = self.rope.line_to_byte(line_idx);
            self.cursor.anchor = line_start;
        }
    }

    pub fn undo(&mut self) {
        self.history
            .undo(&mut self.rope, &mut self.cursor, &mut self.dirty);
        self.queue_syntax_update();
        if self.clamp_cursor {
            self.center_on_cursor();
        }
    }

    pub fn redo(&mut self) {
        self.history
            .redo(&mut self.rope, &mut self.cursor, &mut self.dirty);
        self.queue_syntax_update();
        if self.clamp_cursor {
            self.center_on_cursor();
        }
    }

    pub fn copy(&mut self) {
        let start = self.cursor.position.min(self.cursor.anchor);
        let end = self.cursor.position.max(self.cursor.anchor);
        let copied = if start == end {
            self.rope.line(self.cursor_line_idx()).to_string()
        } else {
            self.rope.byte_slice(start..end).to_string()
        };
        #[cfg(target_os = "linux")]
        clipboard::set_primary(copied.clone());
        clipboard::set_contents(copied);
    }

    pub fn cut(&mut self) {
        self.history.begin(self.cursor, self.dirty);
        let mut start = self.cursor.position.min(self.cursor.anchor);
        let mut end = self.cursor.position.max(self.cursor.anchor);

        if start == end {
            start = self.rope.line_to_byte(self.rope.byte_to_line(start));
            end = self.rope.end_of_line_byte(self.rope.byte_to_line(end));
        }
        let cut = self.rope.byte_slice(start..end).to_string();
        clipboard::set_contents(cut);
        self.history.remove(&mut self.rope, start..end);

        self.cursor.position = start;
        self.cursor.anchor = self.cursor.position;
        self.update_affinity();

        if start != end {
            self.mark_dirty();
        }

        if self.clamp_cursor {
            self.center_on_cursor();
        }
        self.history.finish();
    }

    pub fn paste(&mut self) {
        self.insert_text(&clipboard::get_contents());
    }

    pub fn paste_primary(&mut self, col: usize, line: usize) {
        self.set_cursor_pos(col, line);
        self.insert_text(&clipboard::get_primary());
    }

    // TODO make this not use eof
    pub fn trim_start(&mut self) {
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
        self.eof(false);
    }

    // TODO make this not use eof
    pub fn replace(&mut self, byte_range: Range<usize>, text: &str) {
        self.history.begin(self.cursor, self.dirty);
        self.history.replace(&mut self.rope, byte_range, text);
        self.eof(false);
        self.history.finish();
    }

    pub fn reload(&mut self) -> Result<(), BufferError> {
        let Some(path) = &self.file else {
            return Err(BufferError::NoPathSet);
        };

        let (encoding, rope) = read::read_from_file(path)?;
        self.encoding = encoding;
        let len_bytes = self.rope.len_bytes();
        self.history.replace(&mut self.rope, 0..len_bytes, rope);

        self.dirty = false;
        self.history.save();
        self.queue_syntax_update();

        Ok(())
    }

    pub fn escape(&mut self) {
        if self.searcher.is_some() {
            self.searcher = None;
            return;
        }

        if self.cursor.has_selection() {
            self.cursor.anchor = self.cursor.position;
            if self.clamp_cursor {
                self.center_on_cursor();
            }
        }
    }

    pub fn set_cursor_pos(&mut self, col: usize, line: usize) {
        let line_idx: usize = line.min(self.rope.len_lines().saturating_sub(1));

        let next_line = self.rope.line_without_line_ending(line_idx);
        let next_width = next_line.width(0);
        let next_line_start = self.rope.line_to_byte(line_idx);

        if next_width < col {
            self.cursor.position = next_line_start + next_line.len_bytes();
        } else {
            let idx = next_line.nth_next_grapheme_boundary_byte(0, col);
            self.cursor.position = next_line_start + idx;
        }
        self.cursor.anchor = self.cursor.position;

        if self.clamp_cursor {
            self.center_on_cursor();
        }
    }

    pub fn set_anchor_pos(&mut self, col: usize, line: usize) {
        let line_idx: usize = line.min(self.rope.len_lines().saturating_sub(1));

        let next_line = self.rope.line_without_line_ending(line_idx);
        let next_width = next_line.width(0);
        let next_line_start = self.rope.line_to_byte(line_idx);

        if next_width < col {
            self.cursor.anchor = next_line_start + next_line.len_bytes();
        } else {
            let idx = next_line.nth_next_grapheme_boundary_byte(0, col);
            self.cursor.anchor = next_line_start + idx;
        }
    }

    pub fn select_area(&mut self, cursor: Point<usize>, anchor: Point<usize>) {
        self.set_cursor_pos(cursor.column, cursor.line);
        self.set_anchor_pos(anchor.column, anchor.line);

        #[cfg(target_os = "linux")]
        {
            let start = self.cursor.position.min(self.cursor.anchor);
            let end = self.cursor.position.max(self.cursor.anchor);
            clipboard::set_primary(self.rope.byte_slice(start..end).to_string());
        }
    }

    pub fn center_on_cursor(&mut self) {
        {
            let cursor_line = self.rope.byte_to_line(self.cursor.position);
            let start_line = self.line_pos;
            let end_line = self.line_pos + self.view_lines;
            if cursor_line < start_line || cursor_line >= end_line {
                self.line_pos = cursor_line.saturating_sub(self.view_lines / 2);
            }
        }

        {
            let cursor_col = self.cursor_grapheme_column();
            let start_col = self.col_pos;
            let end_col = self.col_pos + self.view_columns;

            if cursor_col <= start_col {
                self.horizontal_scroll(-((start_col - cursor_col) as i64));
            } else if cursor_col >= end_col {
                self.horizontal_scroll((cursor_col - end_col + 1) as i64);
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

    pub fn queue_syntax_update(&mut self) {
        if let Some(syntax) = &mut self.syntax {
            syntax.update_text(self.rope.clone());
        }
    }

    pub fn get_syntax(&mut self) -> Option<&mut Syntax> {
        self.syntax.as_mut()
    }

    pub fn view_range(&self) -> Range<usize> {
        let start = self.rope.line_to_byte(self.line_pos);
        let end = self
            .rope
            .try_line_to_byte(self.line_pos + self.view_lines)
            .unwrap_or_else(|_| self.rope.len_bytes());
        start..end
    }

    pub fn start_search(
        &mut self,
        proxy: Box<dyn EventLoopProxy>,
        query: String,
        case_insensitive: bool,
    ) {
        if let Some(searcher) = &mut self.searcher {
            searcher.update_query(query, case_insensitive);
        } else {
            let searcher = BufferSearcher::new(proxy, query, self.rope.clone(), case_insensitive);
            self.searcher = Some(searcher);
        }
    }

    pub fn get_searcher(&self) -> Option<&BufferSearcher> {
        self.searcher.as_ref()
    }

    pub fn next_match(&mut self) {
        if let Some(searcher) = &mut self.searcher {
            if let Some(search_match) = searcher.get_next_match() {
                self.select_area(search_match.end, search_match.start);
            }
        }
    }

    pub fn prev_match(&mut self) {
        if let Some(searcher) = &mut self.searcher {
            if let Some(search_match) = searcher.get_prev_match() {
                self.select_area(search_match.end, search_match.start);
            }
        }
    }

    pub fn cursor_is_eof(&self) -> bool {
        self.cursor.position == self.rope.len_bytes()
    }

    pub fn revert_buffer(&mut self) {
        while self.dirty {
            self.undo();
        }
    }

    pub fn move_to_trash(&self) -> Result<bool, trash::Error> {
        if let Some(path) = &self.file {
            trash::delete(path)?;
            return Ok(true);
        }

        Ok(false)
    }

    pub fn get_selection(&self) -> String {
        let start = self.cursor.anchor.min(self.cursor.position);
        let end = self.cursor.anchor.max(self.cursor.position);
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

    pub fn ensure_cursor_is_valid(&mut self) {
        let position = self.cursor.position.min(self.rope.len_bytes());
        let anchor = self.cursor.position.min(self.rope.len_bytes());
        self.cursor.position = self.rope().ensure_grapheme_boundary_next_byte(position);
        self.cursor.anchor = self.rope().ensure_grapheme_boundary_next_byte(anchor);
    }
}

pub struct ViewLine<'a> {
    pub text: RopeSlice<'a>,
    pub col_start_offset: usize,
    pub text_start_col: usize,
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
