use std::{mem, ops::Range};

use ferrite_utility::graphemes::RopeGraphemeExt;
use ropey::Rope;

use super::Cursor;

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
enum EditClass {
    Word,
    WhiteSpace,
    Other,
    Remove,
}

impl EditClass {
    fn mergeable(first: &EditClass, second: &EditClass) -> bool {
        match (first, second) {
            (EditClass::WhiteSpace, EditClass::WhiteSpace) => true,
            (EditClass::Word, EditClass::Word) => true,
            (EditClass::Remove, EditClass::Remove) => true,
            (EditClass::WhiteSpace, EditClass::Word) => true,
            _ => false,
        }
    }
}

impl From<&str> for EditClass {
    fn from(value: &str) -> Self {
        if Rope::from_str(value).is_word_char() {
            return EditClass::Word;
        }
        if Rope::from_str(value).is_whitespace() {
            return EditClass::WhiteSpace;
        }
        EditClass::Other
    }
}

#[derive(Debug, Clone)]
enum EditKind {
    Insert { byte_idx: usize, text: String },
    Replace { range: Range<usize>, text: String },
    Remove { range: Range<usize> },
}

impl EditKind {
    fn get_class(&self) -> EditClass {
        match self {
            EditKind::Insert { text, .. } => EditClass::from(text.as_str()),
            EditKind::Replace { text, .. } => EditClass::from(text.as_str()),
            EditKind::Remove { .. } => EditClass::Remove,
        }
    }

    fn apply(&self, rope: &mut Rope) -> EditKind {
        match self {
            Self::Insert { byte_idx, text } => {
                rope.insert(rope.byte_to_char(*byte_idx), text);
                Self::Remove {
                    range: *byte_idx..(*byte_idx + text.len()),
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
            Self::Remove { range } => {
                let text = rope.byte_slice(range.clone()).to_string();
                rope.remove(rope.byte_to_char(range.start)..rope.byte_to_char(range.end));
                Self::Insert {
                    byte_idx: range.start,
                    text,
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
struct Frame {
    finished: bool,
    edit_class: EditClass,
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
                frame.edit_class = edit.get_class();
                let inverse = edit.apply(rope);
                frame.edits.push(inverse);
            }
            None => tracing::error!("Edited rope before starting new edit frame"),
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
            edit_class: EditClass::Other,
            cursor,
            edits: Vec::new(),
            dirty,
        });
        self.current_frame += 1;

        self.stack[self.current_frame as usize].cursor = cursor;
    }

    pub fn finish(&mut self) {
        // maybe should be current_frame
        if let Some(frame) = self.stack.get_mut(self.current_frame as usize) {
            if !frame.finished {
                frame.finished = true;
            }
        }
    }

    pub fn undo(&mut self, rope: &mut Rope, cursor: &mut Cursor, dirty: &mut bool) {
        if self.current_frame.is_negative() {
            return;
        }

        let mut last_class = None;

        while let Some(frame) = &mut self.stack.get_mut(self.current_frame as usize) {
            for edit in frame.edits.iter_mut().rev() {
                *edit = edit.apply(rope);
            }
            mem::swap(&mut frame.cursor, cursor);
            mem::swap(&mut frame.dirty, dirty);
            cursor.position = rope.ensure_grapheme_boundary_next_byte(cursor.position);
            cursor.anchor = rope.ensure_grapheme_boundary_next_byte(cursor.anchor);
            self.current_frame -= 1;

            if frame.finished {
                break;
            }

            if let Some(frame) = &mut self.stack.get_mut(self.current_frame as usize) {
                let earlier_class = frame.edit_class;
                if let Some(last_class) = last_class {
                    if !EditClass::mergeable(&earlier_class, &last_class) {
                        break;
                    }
                }
                last_class = Some(earlier_class);
            }
        }
    }

    pub fn redo(&mut self, rope: &mut Rope, cursor: &mut Cursor, dirty: &mut bool) {
        let mut last_class = None;

        loop {
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
            cursor.position = rope.ensure_grapheme_boundary_next_byte(cursor.position);
            cursor.anchor = rope.ensure_grapheme_boundary_next_byte(cursor.anchor);

            if frame.finished {
                break;
            }

            if let Some(frame) = &mut self.stack.get_mut(self.current_frame as usize + 1) {
                let earlier_class = frame.edit_class;
                if let Some(last_class) = last_class {
                    if !EditClass::mergeable(&last_class, &earlier_class) {
                        break;
                    }
                }
                last_class = Some(earlier_class);
            }
        }
    }

    pub fn save(&mut self) {
        if self.current_frame.is_negative() {
            return;
        }
        for frame in &mut self.stack {
            frame.dirty = true;
        }
        self.stack[self.current_frame as usize].dirty = false;
    }

    pub fn mark_all_dirty(&mut self) {
        for frame in &mut self.stack {
            frame.dirty = true;
        }
    }
}
