use std::{
    cmp,
    collections::HashMap,
    fs::File,
    io::{self, Read},
    path::{Path, PathBuf},
};

use encoding_rs::Encoding;
use ropey::{Rope, RopeBuilder, RopeSlice};
use utility::{
    graphemes::{
        next_grapheme_boundary_byte, nth_next_grapheme_boundary_byte, prev_grapheme_boundary_byte,
        rope_width, RopeGraphemes,
    },
    line_ending::line_without_line_ending,
};

pub struct Buffer {
    cursor_x: usize,
    cursor_y: usize,
    line_pos: usize,
    rope: Rope,
    file: Option<PathBuf>,
    pub encoding: &'static Encoding,
    column_cache: HashMap<usize, usize>,
}

impl Default for Buffer {
    fn default() -> Self {
        Self {
            cursor_x: 0,
            cursor_y: 0,
            line_pos: 0,
            rope: Rope::new(),
            file: None,
            encoding: encoding_rs::UTF_8,
            column_cache: HashMap::new(),
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

        if self.cursor_y >= start_line && self.cursor_y < end_line {
            Some((self.cursor_x, self.cursor_y - start_line))
        } else {
            None
        }
    }

    pub fn cursor_pos(&self) -> (usize, usize) {
        (self.cursor_x, self.cursor_y)
    }

    pub fn cursor_grapheme_column(&self) -> usize {
        let line = self.get_line(self.cursor_y).unwrap();
        let graphemes = RopeGraphemes::new(line);
        let mut byte_width = 0;
        let mut col = 0;
        for grapheme in graphemes {
            let buf = grapheme.to_string();

            if byte_width > self.cursor_x {
                break;
            }

            byte_width += buf.len();
            col += unicode_width::UnicodeWidthStr::width_cjk(buf.as_str());
        }
        col
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

    pub fn move_right(&mut self) {
        let slice = self.rope.slice(..);
        let line = line_without_line_ending(&slice, self.cursor_y);
        let new_index = next_grapheme_boundary_byte(line, self.cursor_x);
        if new_index == self.cursor_x {
            if self.get_line(self.cursor_y + 1).is_some() {
                self.cursor_x = 0;
                self.cursor_y += 1;
            }
        } else {
            self.cursor_x = new_index;
        }
        self.column_cache.clear();
        self.column_cache.insert(self.cursor_y, self.cursor_x);
    }

    pub fn move_left(&mut self) {
        let slice = self.rope.slice(..);
        let line = line_without_line_ending(&slice, self.cursor_y);
        let new_index = prev_grapheme_boundary_byte(line, self.cursor_x);
        if new_index == self.cursor_x {
            if self.cursor_y > 0 {
                let line = line_without_line_ending(&slice, self.cursor_y - 1);
                self.cursor_x = line.len_bytes();
                self.cursor_y -= 1;
            }
        } else {
            self.cursor_x = new_index;
        }
        self.column_cache.clear();
        self.column_cache.insert(self.cursor_y, self.cursor_x);
    }

    pub fn move_down(&mut self) {
        if self.cursor_y + 1 >= self.rope.len_lines() {
            return;
        }

        if let Some(pos) = self.column_cache.get(&(self.cursor_y + 1)) {
            self.cursor_x = *pos;
            self.cursor_y += 1;
            return;
        }

        let slice = self.rope.slice(..);
        let before_cursor =
            rope_width(line_without_line_ending(&slice, self.cursor_y).byte_slice(..self.cursor_x));
        let next_line = line_without_line_ending(&slice, self.cursor_y + 1);
        let next_width = rope_width(next_line);

        if next_width < before_cursor {
            self.cursor_x = next_line.len_bytes();
        } else {
            let idx = nth_next_grapheme_boundary_byte(next_line, 0, before_cursor);
            self.cursor_x = idx;
        }
        self.cursor_y += 1;
        self.column_cache.insert(self.cursor_y, self.cursor_x);
    }

    pub fn move_up(&mut self) {
        if self.cursor_y == 0 {
            return;
        }

        if let Some(pos) = self.column_cache.get(&(self.cursor_y - 1)) {
            self.cursor_x = *pos;
            self.cursor_y -= 1;
            return;
        }

        let slice = self.rope.slice(..);
        let before_cursor =
            rope_width(line_without_line_ending(&slice, self.cursor_y).byte_slice(..self.cursor_x));
        let next_line = line_without_line_ending(&slice, self.cursor_y - 1);
        let next_width = rope_width(next_line);

        if next_width < before_cursor {
            self.cursor_x = next_line.len_bytes();
        } else {
            let idx = nth_next_grapheme_boundary_byte(next_line, 0, before_cursor);
            self.cursor_x = idx;
        }
        self.cursor_y -= 1;
        self.column_cache.insert(self.cursor_y, self.cursor_x);
    }

    pub fn insert_text(&mut self, text: &str) {
        let line_start = self.rope.line_to_byte(self.cursor_y);
        let idx = self.rope.byte_to_char(line_start + self.cursor_x);
        self.rope.insert(idx, text);
        self.move_right();
    }

    pub fn backspace(&mut self) {
        let line_start = self.rope.line_to_byte(self.cursor_y);
        let end_byte_idx = line_start + self.cursor_x;
        let start_byte_idx = prev_grapheme_boundary_byte(self.rope.slice(..), end_byte_idx);

        let start = self.rope.byte_to_char(start_byte_idx);
        let end = self.rope.byte_to_char(end_byte_idx);

        self.rope.remove(start..end);

        let new_line_idx = self.rope.byte_to_line(start_byte_idx);
        let new_line_start = self.rope.line_to_byte(new_line_idx);

        self.cursor_x = start_byte_idx - new_line_start;
        self.cursor_y = new_line_idx;

        self.column_cache.clear();
        self.column_cache.insert(self.cursor_y, self.cursor_x);
    }
}

pub struct BufferView<'a> {
    pub lines: Vec<RopeSlice<'a>>,
}
