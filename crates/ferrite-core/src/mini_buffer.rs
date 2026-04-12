use ferrite_utility::{graphemes::RopeGraphemeExt, line_ending::LineEnding};
use ropey::RopeSlice;

use crate::{
    buffer::{Buffer, error::BufferError},
    cmd::Cmd,
};
pub struct MiniBuffer {
    pub buffer: Buffer,
    pub left_prompt: Option<String>,
    pub right_prompt: Option<String>,
    pub one_line: bool,
}

impl Default for MiniBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl MiniBuffer {
    pub fn new() -> Self {
        let mut buffer = Buffer::builder().simple(true).build().unwrap();
        let view_id = buffer.create_view();
        buffer.set_view_lines(view_id, 1);
        buffer.views[view_id].clamp_cursor = true;
        Self {
            buffer,
            left_prompt: None,
            right_prompt: None,
            one_line: true,
        }
    }

    pub fn set_left_prompt(&mut self, left_prompt: String) -> &mut Self {
        self.left_prompt = Some(left_prompt);
        self
    }

    pub fn set_right_prompt(&mut self, right_prompt: String) -> &mut Self {
        self.right_prompt = Some(right_prompt);
        self
    }

    pub fn handle_input(&mut self, cmd: Cmd) -> Result<bool, BufferError> {
        let view_id = self.buffer.get_first_view_or_create();
        let mut enter = false;
        match cmd {
            Cmd::Insert { text } => {
                let rope = RopeSlice::from(text.as_str());
                let line = rope.line_without_line_ending(0);
                self.buffer.handle_input(
                    view_id,
                    Cmd::Insert {
                        text: line.to_string(),
                    },
                )?;
                if line.len_bytes() != rope.len_bytes() {
                    enter = true;
                }
            }
            Cmd::Char { ch } if LineEnding::from_char(ch).is_some() => {
                enter = true;
            }
            Cmd::Enter | Cmd::NewLineAboveWithoutBreaking => {
                enter = true;
            }
            Cmd::NewLineWithoutBreaking => {
                if self.one_line {
                    enter = true;
                } else {
                    self.buffer.handle_input(view_id, cmd)?;
                }
            }
            cmd => {
                self.buffer.handle_input(view_id, cmd)?;
                // Make sure there is only a single cursor
                self.buffer.views[view_id].cursors.clear();
            }
        }
        Ok(enter)
    }
}
