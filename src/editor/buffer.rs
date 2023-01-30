use ropey::{Rope, RopeSlice};

pub struct Buffer {
    cursor_pos: usize,
    line_pos: usize,
    rope: Rope,
}

impl Buffer {
    pub fn new(text: &str) -> Self {
        Self {
            cursor_pos: 0,
            line_pos: 0,
            rope: Rope::from(text),
        }
    }

    pub fn set_text(&mut self, text: &str) {
        self.rope = Rope::from(text);
    }

    pub fn get_buffer_view(&self, max_lines: usize) -> BufferView {
        let last_line = std::cmp::min(self.rope.len_lines(), max_lines + self.line_pos);

        let mut lines = Vec::new();
        for line_idx in self.line_pos..last_line {
            lines.push(self.rope.get_line(line_idx).unwrap());
        }

        BufferView { lines }
    }

    pub fn line_pos(&self) -> usize {
        self.line_pos
    }

    pub fn len_lines(&self) -> usize {
        self.rope.len_lines()
    }

    pub fn scroll_up(&mut self) -> bool {
        if self.line_pos > 0 {
            self.line_pos -= 1;
            true
        } else {
            false
        }
    }

    pub fn scroll_down(&mut self) -> bool {
        if self.line_pos + 1 < self.rope.len_lines() {
            self.line_pos += 1;
            true
        } else {
            false
        }
    }
}

pub struct BufferView<'a> {
    pub lines: Vec<RopeSlice<'a>>,
}
