use std::{
    cmp,
    fs::File,
    io::{self, Read},
    path::{Path, PathBuf},
};

use encoding_rs::Encoding;
use ropey::{Rope, RopeBuilder, RopeSlice};
use utility::graphemes::RopeGraphemeExt as _;

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

#[derive(Debug, Default, Clone, Copy)]
pub struct Selection {
    pub start: BufferPos,
    pub end: BufferPos,
}

pub struct Buffer {
    cursor: Cursor,
    line_pos: usize,
    rope: Rope,
    file: Option<PathBuf>,
    pub encoding: &'static Encoding,
}

impl Default for Buffer {
    fn default() -> Self {
        Self {
            cursor: Cursor::default(),
            line_pos: 0,
            rope: Rope::new(),
            file: None,
            encoding: encoding_rs::UTF_8,
        }
    }
}

impl Buffer {
    pub fn new() -> Self {
        Self::default()
    }

    #[allow(dead_code)]
    pub fn with_text(text: &str) -> Self {
        Self {
            rope: Rope::from(text),
            ..Default::default()
        }
    }

    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, io::Error> {
        const BUFFER_SIZE: usize = 8192;
        let path = path.as_ref();

        let mut encoding_detector = chardetng::EncodingDetector::new();
        let mut content = Vec::new();
        let mut buffer = vec![0u8; BUFFER_SIZE];
        let mut file = File::open(path)?;

        let encoding = loop {
            let len = file.read(&mut buffer)?;
            let filled = &buffer[..len];
            encoding_detector.feed(filled, len == 0);
            content.extend_from_slice(filled);

            if let (e, true) = encoding_detector.guess_assess(None, true) {
                break e;
            }
        };

        let mut decoder = encoding.new_decoder();
        let mut rope_builder = RopeBuilder::new();
        let mut output = String::with_capacity(BUFFER_SIZE);

        let mut input = &content[..];
        loop {
            if input.is_empty() {
                let len = file.read(&mut buffer)?;
                input = &buffer[..len];
            }
            let (result, read, _) = decoder.decode_to_string(input, &mut output, input.is_empty());
            input = &input[read..];
            match result {
                encoding_rs::CoderResult::InputEmpty => {
                    rope_builder.append(&output);
                    break;
                }
                encoding_rs::CoderResult::OutputFull => {
                    rope_builder.append(&output);
                    output.clear();
                }
            };
        }

        Ok(Self {
            rope: rope_builder.finish(),
            file: Some(path.to_path_buf()),
            encoding,
            ..Default::default()
        })
    }

    #[allow(dead_code)]
    pub fn set_text(&mut self, text: &str) {
        self.rope = Rope::from(text);
    }

    #[allow(dead_code)]
    pub fn cursor(&self) -> Cursor {
        self.cursor
    }

    pub fn get_buffer_view(&self, max_lines: usize) -> BufferView {
        let end_line = std::cmp::min(self.rope.len_lines(), max_lines + self.line_pos);

        let mut lines = Vec::new();
        for line_idx in self.line_pos..end_line {
            lines.push(self.rope.get_line(line_idx).unwrap());
        }

        BufferView { lines }
    }

    pub fn get_line(&self, line_idx: usize) -> Option<RopeSlice> {
        self.rope.get_line(line_idx)
    }

    pub fn file(&self) -> Option<&Path> {
        self.file.as_deref()
    }

    pub fn line_pos(&self) -> usize {
        self.line_pos
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
        let line = self.get_line(line_idx).unwrap();
        let start = line.byte_slice(..column_idx);
        start.width()
    }

    pub fn anchor_grapheme_column(&self) -> usize {
        let (column_idx, line_idx) = self.anchor_pos();
        let line = self.get_line(line_idx).unwrap();
        let start = line.byte_slice(..column_idx);
        start.width()
    }

    pub fn scroll(&mut self, distance: i64) {
        if distance.is_positive() {
            let new_pos = cmp::min(
                cmp::max(self.len_lines(), 1) as i64 - 1,
                self.line_pos as i64 + distance,
            );
            self.line_pos = new_pos as usize;
        } else {
            let new_pos = cmp::max(0, self.line_pos as i64 + distance);
            self.line_pos = new_pos as usize;
        }
    }

    pub fn move_right_char(&mut self, shift: bool) {
        let new_idx = self.rope.next_grapheme_boundary_byte(self.cursor.position);
        self.cursor.position = new_idx;

        if !shift {
            self.cursor.anchor = self.cursor.position;
        }

        let (column_idx, line_idx) = self.cursor_pos();
        self.cursor.affinity = self
            .rope
            .line_without_line_ending(line_idx)
            .byte_slice(..column_idx)
            .width();
    }

    pub fn move_left_char(&mut self, shift: bool) {
        let new_idx = self.rope.prev_grapheme_boundary_byte(self.cursor.position);
        self.cursor.position = new_idx;

        if !shift {
            self.cursor.anchor = self.cursor.position;
        }

        let (column_idx, line_idx) = self.cursor_pos();
        self.cursor.affinity = self
            .rope
            .line_without_line_ending(line_idx)
            .byte_slice(..column_idx)
            .width();
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
            .width()
            .max(self.cursor.affinity);
        let next_line = self.rope.line_without_line_ending(line_idx + 1);
        let next_width = next_line.width();
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
            .width()
            .max(self.cursor.affinity);
        let next_line = self.rope.line_without_line_ending(line_idx - 1);
        let next_width = next_line.width();
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
    }

    pub fn insert_text(&mut self, text: &str) {
        self.rope
            .insert(self.rope.byte_to_char(self.cursor.position), text);
        self.cursor.position += text.len();
        self.cursor.anchor = self.cursor.position;
    }

    pub fn backspace(&mut self) {
        let (start_byte_idx, end_byte_idx) = if self.cursor.position == self.cursor.anchor {
            let start_byte_idx = self.rope.prev_grapheme_boundary_byte(self.cursor.position);
            (start_byte_idx, self.cursor.position)
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
    }

    pub fn select_all(&mut self) {
        self.cursor.anchor = 0;
        self.cursor.position = self.rope.len_bytes();
    }
}

pub struct BufferView<'a> {
    pub lines: Vec<RopeSlice<'a>>,
}
