use std::{mem, ops::Range};

use ferrite_utility::{graphemes::RopeGraphemeExt, vec1::Vec1};
use ropey::Rope;
use slotmap::{Key, SecondaryMap};

use super::{Cursor, ViewId};

/// The history stores the edit history of a file as series of edits with metadata like
/// cursor positions. The history does not store copies of the state instead is stores
/// the data needed to undo each transition. That inverse data is then applied and swapped
/// with the incomming edit when undoing. This is a bit messy but it has the advantage of
/// only having to store a single copy of the edit data instead of both the edit and its inverse.
#[derive(Debug, Clone)]
pub struct History {
    stack: Vec<Frame>,
    current_frame: usize,
}

impl Default for History {
    fn default() -> Self {
        Self {
            stack: vec![Frame {
                finished: false,
                edit_class: EditClass::Other,
                view_id: ViewId::null(),
                cursors: SecondaryMap::default(),
                edits: Vec::new(),
                dirty: false,
                id: rand::random(),
            }],
            current_frame: 0,
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

    pub fn insert(&mut self, rope: &mut Rope, byte_idx: usize, text: impl Into<String>) -> bool {
        let text = text.into();
        if text.is_empty() {
            return false;
        }
        let insert = EditKind::Insert { byte_idx, text };
        self.edit(rope, insert);
        true
    }

    pub fn remove(&mut self, rope: &mut Rope, byte_range: Range<usize>) -> bool {
        if byte_range.start == byte_range.end {
            return false;
        }
        let remove = EditKind::Remove { range: byte_range };
        self.edit(rope, remove);
        true
    }

    pub fn replace(
        &mut self,
        rope: &mut Rope,
        byte_range: Range<usize>,
        text: impl Into<String>,
    ) -> bool {
        let text = text.into();
        if byte_range.start == byte_range.end && text.is_empty() {
            return false;
        }
        let replace = EditKind::Replace {
            range: byte_range,
            text,
        };
        self.edit(rope, replace);
        true
    }

    pub fn begin(
        &mut self,
        view_id: ViewId,
        cursors: SecondaryMap<ViewId, Vec1<Cursor>>,
        dirty: bool,
    ) {
        self.stack.truncate(self.current_frame + 1);

        self.stack.push(Frame {
            finished: false,
            edit_class: EditClass::Other,
            view_id,
            cursors: cursors.clone(),
            edits: Vec::new(),
            dirty,
            id: rand::random(),
        });
        self.current_frame += 1;

        self.stack[self.current_frame].cursors = cursors;
    }

    pub fn finish(&mut self) {
        if let Some(frame) = self.stack.get_mut(self.current_frame) {
            frame.finished = true;
            if frame.edits.is_empty() && self.current_frame != 0 {
                self.stack.remove(self.current_frame);
                self.current_frame -= 1;
            }
        }
    }

    pub fn undo(
        &mut self,
        rope: &mut Rope,
        view_id: &mut ViewId,
        cursors: &mut SecondaryMap<ViewId, Vec1<Cursor>>,
        dirty: &mut bool,
    ) {
        if self.current_frame == 0 {
            return;
        }

        let mut last_class = None;

        while let Some(frame) = &mut self.stack.get_mut(self.current_frame) {
            for edit in frame.edits.iter_mut().rev() {
                *edit = edit.apply(rope);
            }
            mem::swap(&mut frame.cursors, cursors);
            mem::swap(&mut frame.dirty, dirty);
            mem::swap(&mut frame.view_id, view_id);
            self.current_frame -= 1;

            if let Some(frame) = &mut self.stack.get_mut(self.current_frame) {
                if frame.finished {
                    break;
                }
                let earlier_class = frame.edit_class;
                if let Some(last_class) = last_class
                    && !EditClass::mergeable(&earlier_class, &last_class)
                {
                    break;
                }
                last_class = Some(earlier_class);
            }
        }
    }

    pub fn redo(
        &mut self,
        rope: &mut Rope,
        view_id: &mut ViewId,
        cursors: &mut SecondaryMap<ViewId, Vec1<Cursor>>,
        dirty: &mut bool,
    ) {
        let mut last_class = None;
        let mut running = true;

        while running {
            if self.current_frame + 1 >= self.stack.len() {
                break;
            }
            self.current_frame += 1;
            let frame = &mut self.stack[self.current_frame];

            for edit in &mut frame.edits {
                *edit = edit.apply(rope);
            }
            mem::swap(&mut frame.cursors, cursors);
            mem::swap(&mut frame.dirty, dirty);
            mem::swap(&mut frame.view_id, view_id);

            if frame.finished {
                running = false;
            }

            if let Some(frame) = &mut self.stack.get_mut(self.current_frame + 1) {
                let earlier_class = frame.edit_class;
                if let Some(last_class) = last_class
                    && !EditClass::mergeable(&last_class, &earlier_class)
                {
                    break;
                }
                last_class = Some(earlier_class);
            }
        }
    }

    /// returns true if the current position in history is dirty
    pub fn save(&mut self, id: u64) -> bool {
        self.mark_all_dirty();
        if let Some(clean_idx) = self.stack.iter().position(|frame| frame.id == id) {
            // This is a ugly hack to make sure the correct frame is dirty
            // it is needed because the history does not store revisions
            // but transitions between them. It is a bit harder to understand
            // but it has the upside of only having to store one copy of the edit
            // operation instead of two as you swap them instead
            if clean_idx == self.current_frame {
                self.stack[clean_idx].dirty = false;
                return false;
            } else if clean_idx < self.current_frame {
                self.stack[clean_idx + 1].dirty = false;
            } else if clean_idx > self.current_frame {
                self.stack[clean_idx].dirty = false;
            }
        }
        true
    }

    /// returns the current frames id, if there is no current frame return 0
    pub fn current_id(&self) -> u64 {
        self.stack[self.current_frame].id
    }

    pub fn mark_all_dirty(&mut self) {
        for frame in &mut self.stack {
            frame.dirty = true;
        }
    }
}

#[derive(Debug, Clone)]
struct Frame {
    finished: bool,
    edit_class: EditClass,
    view_id: ViewId,
    cursors: SecondaryMap<ViewId, Vec1<Cursor>>,
    edits: Vec<EditKind>,
    dirty: bool,
    id: u64,
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

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
enum EditClass {
    Word,
    WhiteSpace,
    Other,
    Remove,
}

impl EditClass {
    fn mergeable(first: &EditClass, second: &EditClass) -> bool {
        matches!(
            (first, second),
            (EditClass::WhiteSpace, EditClass::WhiteSpace)
                | (EditClass::Word, EditClass::Word)
                | (EditClass::Remove, EditClass::Remove)
                | (EditClass::WhiteSpace, EditClass::Word)
        )
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
