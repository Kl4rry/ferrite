use std::{
    cmp, io,
    num::NonZeroUsize,
    path::{Path, PathBuf},
};

use encoding_rs::Encoding;
use ropey::{Rope, RopeSlice};
use utility::{
    graphemes::{RopeGraphemeExt as _, RopeReplaceExt},
    line_ending::rope_end_without_line_ending,
};

use self::error::BufferError;
use super::indent::Indentation;
use crate::tui_app::input::LineMoveDir;

pub mod error;
mod input;
mod read;
mod write;

#[cfg(test)]
pub mod buffer_tests;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BufferPos {
    // line must be first for the ord derive to work correctly
    pub line: i64,
    pub column: i64,
}

#[derive(Debug, Default, Clone, Copy)]
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
    pub start: BufferPos,
    pub end: BufferPos,
}

#[derive(Debug)]
pub struct Buffer {
    cursor: Cursor,
    line_pos: usize,
    col_pos: usize,
    rope: Rope,
    file: Option<PathBuf>,
    dirty: bool,
    read_only: bool,
    pub encoding: &'static Encoding,
    pub indent: Indentation,
    pub clamp_cursor: bool,
    pub clamp_distance: usize,
    // view stuff
    view_lines: usize,
    view_columns: usize,
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
            read_only: false,
            clamp_cursor: true,
            clamp_distance: 4,
            view_lines: usize::MAX,
            view_columns: usize::MAX,
        }
    }
}

impl ToString for Buffer {
    fn to_string(&self) -> String {
        self.rope.to_string()
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
        Self {
            file: Some(path.into()),
            ..Default::default()
        }
    }

    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, io::Error> {
        let path = path.as_ref();
        let (encoding, rope) = read::read(path)?;

        Ok(Self {
            indent: Indentation::detect_indent_rope(rope.slice(..)),
            rope,
            file: Some(path.into()),
            encoding,
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

    pub fn set_view_columns(&mut self, cols: usize) {
        self.view_columns = cols;
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

    pub fn get_buffer_view(&self) -> BufferView {
        let end_line = cmp::min(self.rope.len_lines(), self.view_lines + self.line_pos);

        let mut lines = Vec::new();
        for line_idx in self.line_pos..end_line {
            let line = self.rope.line(line_idx);
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
            });
        }

        BufferView { lines }
    }

    pub fn rope(&self) -> RopeSlice {
        self.rope.slice(..)
    }

    pub fn file(&self) -> Option<&Path> {
        self.file.as_deref()
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
        let pos = BufferPos {
            line: self.cursor_line_idx() as i64,
            column: self.cursor_grapheme_column() as i64,
        };

        let anchor = BufferPos {
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
            self.cursor.anchor = self.cursor.position;
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
            self.cursor.anchor = self.cursor.position;
        }

        self.update_affinity();

        if self.clamp_cursor {
            self.center_on_cursor();
        }
    }

    pub fn move_down(&mut self, shift: bool) {
        let (column_idx, line_idx) = self.cursor_pos();
        if line_idx + 1 >= self.rope.len_lines() {
            return;
        }

        let before_cursor = self
            .rope
            .line_without_line_ending(line_idx)
            .byte_slice(..column_idx)
            .width(0)
            .max(self.cursor.affinity);
        let next_line = self.rope.line_without_line_ending(line_idx + 1);
        let next_width = next_line.width(0);
        let next_line_start = self.rope.line_to_byte(line_idx + 1);

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

    pub fn move_up(&mut self, shift: bool) {
        let (column_idx, line_idx) = self.cursor_pos();
        if line_idx == 0 {
            return;
        }

        let before_cursor = self
            .rope
            .line_without_line_ending(line_idx)
            .byte_slice(..column_idx)
            .width(0)
            .max(self.cursor.affinity);
        let next_line = self.rope.line_without_line_ending(line_idx - 1);
        let next_width = next_line.width(0);
        let next_line_start = self.rope.line_to_byte(line_idx - 1);

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

    pub fn goto(&mut self, line: i64) {
        let line_idx = (self.rope.len_lines().saturating_sub(1) as i64)
            .min(line)
            .max(0) as usize;
        let (column_idx, _) = self.cursor_pos();

        let before_cursor = self
            .rope
            .line_without_line_ending(line_idx)
            .byte_slice(..column_idx)
            .width(0)
            .max(self.cursor.affinity);
        let next_line = self
            .rope
            .line_without_line_ending(line_idx.saturating_sub(1));
        let next_width = next_line.width(0);
        let next_line_start = self.rope.line_to_byte(line_idx.saturating_sub(1));

        if next_width < before_cursor {
            self.cursor.position = next_line_start + next_line.len_bytes();
        } else {
            let idx = next_line.nth_next_grapheme_boundary_byte(0, before_cursor);
            self.cursor.position = next_line_start + idx;
            self.cursor.anchor = self.cursor.position;
        }

        if self.clamp_cursor {
            self.center_on_cursor();
        }
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
        if self.cursor.has_selection() {
            let start_char_idx = self
                .rope
                .byte_to_char(self.cursor.position.min(self.cursor.anchor));
            let end_char_idx = self
                .rope
                .byte_to_char(self.cursor.position.max(self.cursor.anchor));
            self.rope.remove(start_char_idx..end_char_idx);
            self.cursor.position = self.cursor.position.min(self.cursor.anchor);
            self.cursor.anchor = self.cursor.position;
        }

        self.rope
            .insert(self.rope.byte_to_char(self.cursor.position), text);
        self.cursor.position += text.len();
        self.cursor.anchor = self.cursor.position;

        self.update_affinity();
        self.dirty = true;

        // Close pairs
        match text {
            "{" => self
                .rope
                .insert(self.rope.byte_to_char(self.cursor.position), "}"),
            "[" => self
                .rope
                .insert(self.rope.byte_to_char(self.cursor.position), "]"),
            "(" => self
                .rope
                .insert(self.rope.byte_to_char(self.cursor.position), ")"),
            "'" => self
                .rope
                .insert(self.rope.byte_to_char(self.cursor.position), "'"),
            "\"" => self
                .rope
                .insert(self.rope.byte_to_char(self.cursor.position), "\""),
            _ => (),
        }

        if self.clamp_cursor {
            self.center_on_cursor();
        }
    }

    pub fn backspace(&mut self) {
        // TODO add indentation removal when only white space is to the left of cursor
        let (start_byte_idx, end_byte_idx) = if !self.cursor.has_selection() {
            let start_byte_idx = self.rope.prev_grapheme_boundary_byte(self.cursor.position);

            let start_byte = self.rope.get_byte(start_byte_idx);
            let end_byte = self.rope.get_byte(start_byte_idx + 1);

            // Remove pair
            let end_byte_idx = match (start_byte, end_byte) {
                (Some(b'{'), Some(b'}')) => self.cursor.position + 1,
                (Some(b'['), Some(b']')) => self.cursor.position + 1,
                (Some(b'('), Some(b')')) => self.cursor.position + 1,
                (Some(b'\''), Some(b'\'')) => self.cursor.position + 1,
                (Some(b'"'), Some(b'"')) => self.cursor.position + 1,
                _ => self.cursor.position,
            };

            (start_byte_idx, end_byte_idx)
        } else {
            let start_byte_idx = self.cursor.position.min(self.cursor.anchor);
            let end_byte_idx = self.cursor.position.max(self.cursor.anchor);
            (start_byte_idx, end_byte_idx)
        };

        let start_char_idx = self.rope.byte_to_char(start_byte_idx);
        let end_char_idx = self.rope.byte_to_char(end_byte_idx);

        self.rope.remove(start_char_idx..end_char_idx);
        self.cursor.position = start_byte_idx;
        self.cursor.anchor = self.cursor.position;
        self.update_affinity();

        if start_char_idx != end_char_idx {
            self.dirty = true;
        }

        if self.clamp_cursor {
            self.center_on_cursor();
        }
    }

    pub fn backspace_word(&mut self) {
        if self.cursor.has_selection() {
            self.backspace_word();
        } else {
            let prev_word = self.prev_word_start();
            let start_char_idx = self.rope.byte_to_char(prev_word);
            let end_char_idx = self.rope.byte_to_char(self.cursor.position);
            self.rope.remove(start_char_idx..end_char_idx);
            self.cursor.position = prev_word;
            self.cursor.anchor = self.cursor.position;
            self.update_affinity();

            if start_char_idx != end_char_idx {
                self.dirty = true;
            }

            if self.clamp_cursor {
                self.center_on_cursor();
            }
        }
    }

    pub fn delete(&mut self) {
        let (start_byte_idx, end_byte_idx) = if !self.cursor.has_selection() {
            let end_byte_idx = self.rope.next_grapheme_boundary_byte(self.cursor.position);
            (self.cursor.position, end_byte_idx)
        } else {
            let start_byte_idx = self.cursor.position.min(self.cursor.anchor);
            let end_byte_idx = self.cursor.position.max(self.cursor.anchor);
            (start_byte_idx, end_byte_idx)
        };

        let start_char_idx = self.rope.byte_to_char(start_byte_idx);
        let end_char_idx = self.rope.byte_to_char(end_byte_idx);

        self.rope.remove(start_char_idx..end_char_idx);
        self.cursor.position = start_byte_idx;
        self.cursor.anchor = self.cursor.position;
        self.update_affinity();

        if start_char_idx != end_char_idx {
            self.dirty = true;
        }

        if self.clamp_cursor {
            self.center_on_cursor();
        }
    }

    pub fn delete_word(&mut self) {
        if self.cursor.has_selection() {
            self.delete();
        } else {
            let next_word = self.next_word_end();
            let start_char_idx = self.rope.byte_to_char(self.cursor.position);
            let end_char_idx = self.rope.byte_to_char(next_word);
            self.rope.remove(start_char_idx..end_char_idx);
            self.update_affinity();

            if start_char_idx != end_char_idx {
                self.dirty = true;
            }

            if self.clamp_cursor {
                self.center_on_cursor();
            }
        }
    }

    pub fn move_line(&mut self, dir: LineMoveDir) {
        let (cursor_col, cursor_line_idx) = self.cursor_pos();
        let (anchor_col, anchor_line_idx) = self.anchor_pos();

        let start_line_idx = cursor_line_idx.min(anchor_line_idx);
        let mut end_line_idx = cursor_line_idx.max(anchor_line_idx);

        if start_line_idx == 0 && dir == LineMoveDir::Up {
            return;
        }

        if end_line_idx + 1 >= self.len_lines() && dir == LineMoveDir::Down {
            return;
        }

        let end_col = cursor_col.max(anchor_col);
        if end_col == 0 && start_line_idx < end_line_idx {
            end_line_idx -= 1;
        }

        let old_line_idx = self
            .rope
            .byte_to_line(self.cursor.position.min(self.cursor.anchor));
        let offset = match dir {
            LineMoveDir::Up => -1,
            LineMoveDir::Down => 1,
        };
        let new_line_idx = (old_line_idx as i64 + offset) as usize;

        let new_line_has_line_ending = self.rope.line(new_line_idx).get_line_ending().is_some();

        let start_char_idx = self.rope.line_to_char(start_line_idx);
        let end_char_idx = self.rope.end_of_line_char(end_line_idx);

        let removed = self.rope.slice(start_char_idx..end_char_idx);
        let removed = if new_line_has_line_ending {
            removed.to_string()
        } else {
            removed
                .slice(..rope_end_without_line_ending(&removed))
                .to_string()
        };

        self.rope.remove(start_char_idx..end_char_idx);

        let new_line_start_char_idx = self.rope.line_to_char(new_line_idx);
        self.rope.insert(new_line_start_char_idx, &removed);

        if !new_line_has_line_ending {
            self.rope.insert(new_line_start_char_idx, "\n");
        }

        let new_cursor_line_idx = (cursor_line_idx as i64 + offset) as usize;
        let new_anchor_line_idx = (anchor_line_idx as i64 + offset) as usize;

        self.cursor.position = self.rope.line_to_byte(new_cursor_line_idx) + cursor_col;
        self.cursor.anchor = self.rope.line_to_byte(new_anchor_line_idx) + anchor_col;
        self.update_affinity();
        self.dirty = true;

        if self.clamp_cursor {
            self.center_on_cursor();
        }
    }

    pub fn tab(&mut self, back: bool) {
        // TODO optimize for larger files

        if !self.cursor.has_selection() && !back {
            let col = self.cursor_grapheme_column();
            self.insert_text(&self.indent.to_next_ident(col));

            self.update_affinity();
            self.dirty = true;

            if self.clamp_cursor {
                self.center_on_cursor();
            }
            return;
        }

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

                    let start_char_idx = self.rope.byte_to_char(line_start_byte_idx);
                    let end_char_idx = self
                        .rope
                        .byte_to_char(line_start_byte_idx + text_start_byte);
                    self.rope
                        .replace(start_char_idx..end_char_idx, &new_start_of_line);

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
        self.dirty = true;

        if self.clamp_cursor {
            self.center_on_cursor();
        }
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

    pub fn copy(&mut self) {
        let start = self.cursor.position.min(self.cursor.anchor);
        let end = self.cursor.position.max(self.cursor.anchor);
        let copied = if start == end {
            self.rope.line(self.cursor_line_idx()).to_string()
        } else {
            self.rope.byte_slice(start..end).to_string()
        };
        cli_clipboard::set_contents(copied).unwrap();
    }

    pub fn cut(&mut self) {
        let mut start = self.cursor.position.min(self.cursor.anchor);
        let mut end = self.cursor.position.max(self.cursor.anchor);

        if start == end {
            start = self.rope.line_to_byte(self.rope.byte_to_line(start));
            end = self.rope.end_of_line_byte(self.rope.byte_to_line(end));
        }
        let cut = self.rope.byte_slice(start..end).to_string();
        cli_clipboard::set_contents(cut).unwrap();
        self.rope
            .remove(self.rope.byte_to_char(start)..self.rope.byte_to_char(end));
        self.cursor.position = start;
        self.cursor.anchor = self.cursor.position;
        self.update_affinity();

        if start != end {
            self.dirty = true;
        }

        if self.clamp_cursor {
            self.center_on_cursor();
        }
    }

    pub fn paste(&mut self) {
        let Ok(text) = cli_clipboard::get_contents() else {
            return;
        };
        self.insert_text(&text);
    }

    pub fn save(&mut self, path: Option<PathBuf>) -> Result<(), BufferError> {
        if let Some(path) = path {
            self.file = Some(path);
        }

        let Some(path) = self.file.clone() else {
            return Err(BufferError::NoPathSet)
        };

        write::write(self.encoding, self.rope.clone(), path)?;

        self.dirty = false;

        Ok(())
    }

    pub fn reload(&mut self) -> Result<(), BufferError> {
        let Some(path) = &self.file else {
            return Err(BufferError::NoPathSet);
        };

        let (encoding, rope) = read::read(path)?;
        self.encoding = encoding;
        self.rope = rope;
        self.dirty = false;

        Ok(())
    }

    pub fn escape(&mut self) {
        if self.cursor.has_selection() {
            self.cursor.anchor = self.cursor.position;
            if self.clamp_cursor {
                self.center_on_cursor();
            }
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
}

pub struct ViewLine<'a> {
    pub text: RopeSlice<'a>,
    pub col_start_offset: usize,
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
