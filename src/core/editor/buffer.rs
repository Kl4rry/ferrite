use std::{
    cmp,
    fs::File,
    io::{self, Read},
    path::{Path, PathBuf},
};

use encoding_rs::Encoding;
use ropey::{Rope, RopeBuilder, RopeSlice};
use utility::graphemes::{RopeGraphemeExt as _, RopeGraphemes};

pub struct Buffer {
    cursor: usize,
    line_pos: usize,
    rope: Rope,
    file: Option<PathBuf>,
    pub encoding: &'static Encoding,
    cursor_affinity: usize,
}

impl Default for Buffer {
    fn default() -> Self {
        Self {
            cursor: 0,
            line_pos: 0,
            rope: Rope::new(),
            file: None,
            encoding: encoding_rs::UTF_8,
            cursor_affinity: 0,
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

        let (column, line) = self.cursor_pos();

        if line >= start_line && line < end_line {
            Some((column, line - start_line))
        } else {
            None
        }
    }

    pub fn current_line_idx(&self) -> usize {
        self.rope.byte_to_line(self.cursor)
    }

    pub fn cursor_pos(&self) -> (usize, usize) {
        let current_line = self.current_line_idx();
        let start_of_line = self.rope.line_to_byte(current_line);
        let column = self.cursor - start_of_line;

        (column, current_line)
    }

    pub fn cursor_grapheme_column(&self) -> usize {
        let (column_idx, line_idx) = self.cursor_pos();

        let line = self.get_line(line_idx).unwrap();
        let graphemes = RopeGraphemes::new(line);
        let mut byte_width = 0;
        let mut col = 0;
        for grapheme in graphemes {
            let buf = grapheme.to_string();

            if byte_width > column_idx {
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
        let new_idx = self.rope.next_grapheme_boundary_byte(self.cursor);
        self.cursor = new_idx;

        let (column_idx, line_idx) = self.cursor_pos();
        self.cursor_affinity = self
            .rope
            .line_without_line_ending(line_idx)
            .byte_slice(..column_idx)
            .width();
    }

    pub fn move_left(&mut self) {
        let new_idx = self.rope.prev_grapheme_boundary_byte(self.cursor);
        self.cursor = new_idx;

        let (column_idx, line_idx) = self.cursor_pos();
        self.cursor_affinity = self
            .rope
            .line_without_line_ending(line_idx)
            .byte_slice(..column_idx)
            .width();
    }

    pub fn move_down(&mut self) {
        let (column_idx, line_idx) = self.cursor_pos();
        if line_idx + 1 >= self.rope.len_lines() {
            return;
        }

        let before_cursor = self
            .rope
            .line_without_line_ending(line_idx)
            .byte_slice(..column_idx)
            .width()
            .max(self.cursor_affinity);
        let next_line = self.rope.line_without_line_ending(line_idx + 1);
        let next_width = next_line.width();
        let next_line_start = self.rope.line_to_byte(line_idx + 1);

        if next_width < before_cursor {
            self.cursor = next_line_start + next_line.len_bytes();
        } else {
            let idx = next_line.nth_next_grapheme_boundary_byte(0, before_cursor);
            self.cursor = next_line_start + idx;
        }
    }

    pub fn move_up(&mut self) {
        let (column_idx, line_idx) = self.cursor_pos();
        if line_idx == 0 {
            return;
        }

        let before_cursor = self
            .rope
            .line_without_line_ending(line_idx)
            .byte_slice(..column_idx)
            .width()
            .max(self.cursor_affinity);
        let next_line = self.rope.line_without_line_ending(line_idx - 1);
        let next_width = next_line.width();
        let next_line_start = self.rope.line_to_byte(line_idx - 1);

        if next_width < before_cursor {
            self.cursor = next_line_start + next_line.len_bytes();
        } else {
            let idx = next_line.nth_next_grapheme_boundary_byte(0, before_cursor);
            self.cursor = next_line_start + idx;
        }
    }

    pub fn insert_text(&mut self, text: &str) {
        self.rope.insert(self.rope.byte_to_char(self.cursor), text);
        self.cursor += text.len();
    }

    pub fn backspace(&mut self) {
        let start_byte_idx = self.rope.prev_grapheme_boundary_byte(self.cursor);
        let start_char_idx = self.rope.byte_to_char(start_byte_idx);
        let end_char_idx = self.rope.byte_to_char(self.cursor);
        self.rope.remove(start_char_idx..end_char_idx);
        self.cursor = start_byte_idx;
    }
}

pub struct BufferView<'a> {
    pub lines: Vec<RopeSlice<'a>>,
}
