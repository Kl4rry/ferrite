use std::{mem, ops::Range};

use ropey::Rope;
use utility::graphemes::ensure_grapheme_boundary_next_byte;

use super::Cursor;

#[derive(Debug, Clone)]
enum EditKind {
    Insert { byte_idx: usize, text: String },
    Remove { range: Range<usize> },
    Replace { range: Range<usize>, text: String },
}

impl EditKind {
    fn apply(&self, rope: &mut Rope) -> EditKind {
        match self {
            Self::Insert { byte_idx, text } => {
                rope.insert(rope.byte_to_char(*byte_idx), text);
                Self::Remove {
                    range: *byte_idx..(*byte_idx + text.len()),
                }
            }
            Self::Remove { range } => {
                let text = rope.byte_slice(range.clone()).to_string();
                rope.remove(rope.byte_to_char(range.start)..rope.byte_to_char(range.end));
                Self::Insert {
                    byte_idx: range.start,
                    text,
                }
            }
            Self::Replace { range, text } => {
                let old = rope.byte_slice(range.clone()).to_string();
                let char_range = rope.byte_to_char(range.start)..rope.byte_to_char(range.end);
                rope.remove(char_range.clone());
                rope.insert(char_range.start, text);
                Self::Replace {
                    range: range.start..(range.start + text.len()),
                    text: old,
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
struct Frame {
    finished: bool,
    cursor: Cursor,
    edits: Vec<EditKind>,
    dirty: bool,
}

#[derive(Debug, Clone)]
pub struct History {
    stack: Vec<Frame>,
    current_frame: i64,
}

impl Default for History {
    fn default() -> Self {
        Self {
            stack: Vec::new(),
            current_frame: -1,
        }
    }
}

impl History {
    fn edit(&mut self, rope: &mut Rope, edit: EditKind) {
        match self.stack.last_mut() {
            Some(frame) => {
                let inverse = edit.apply(rope);
                if !frame.finished {
                    frame.edits.push(inverse);
                }
            }
            None => log::error!("Edited rope before starting new edit frame"),
        }
    }

    pub fn insert(&mut self, rope: &mut Rope, byte_idx: usize, text: impl Into<String>) {
        let insert = EditKind::Insert {
            byte_idx,
            text: text.into(),
        };
        self.edit(rope, insert);
    }

    pub fn remove(&mut self, rope: &mut Rope, byte_range: Range<usize>) {
        let remove = EditKind::Remove { range: byte_range };
        self.edit(rope, remove);
    }

    pub fn replace(&mut self, rope: &mut Rope, byte_range: Range<usize>, text: impl Into<String>) {
        let replace = EditKind::Replace {
            range: byte_range,
            text: text.into(),
        };
        self.edit(rope, replace);
    }

    pub fn begin(&mut self, cursor: Cursor, dirty: bool) {
        self.stack.truncate((self.current_frame + 1) as usize);

        self.stack.push(Frame {
            finished: false,
            cursor,
            edits: Vec::new(),
            dirty,
        });
        self.current_frame += 1;

        self.stack[self.current_frame as usize].cursor = cursor;
    }

    pub fn finish(&mut self) {
        // maybe should be current_frame
        if let Some(frame) = self.stack.last_mut() {
            if !frame.finished {
                frame.finished = true;
            }
        }
    }

    pub fn undo(&mut self, rope: &mut Rope, cursor: &mut Cursor, dirty: &mut bool) {
        if self.current_frame == -1 {
            return;
        }

        let frame = &mut self.stack[self.current_frame as usize];
        for edit in frame.edits.iter_mut().rev() {
            *edit = edit.apply(rope);
        }
        mem::swap(&mut frame.cursor, cursor);
        mem::swap(&mut frame.dirty, dirty);
        cursor.position = ensure_grapheme_boundary_next_byte(rope.slice(..), cursor.position);
        cursor.anchor = ensure_grapheme_boundary_next_byte(rope.slice(..), cursor.anchor);

        self.current_frame -= 1;
    }

    pub fn redo(&mut self, rope: &mut Rope, cursor: &mut Cursor, dirty: &mut bool) {
        if self.current_frame + 1 >= self.stack.len() as i64 {
            return;
        }

        self.current_frame += 1;

        let frame = &mut self.stack[self.current_frame as usize];
        for edit in &mut frame.edits {
            *edit = edit.apply(rope);
        }
        mem::swap(&mut frame.cursor, cursor);
        mem::swap(&mut frame.dirty, dirty);
        cursor.position = ensure_grapheme_boundary_next_byte(rope.slice(..), cursor.position);
        cursor.anchor = ensure_grapheme_boundary_next_byte(rope.slice(..), cursor.anchor);
    }
}
