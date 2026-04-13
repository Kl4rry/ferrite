use std::path::PathBuf;

use ferrite_utility::vec1::Vec1;

use crate::{buffer::cursor::Cursor, workspace::BufferId};

#[derive(Clone, Debug)]
pub enum JumpPoint {
    Buffer {
        buffer_id: BufferId,
        cursors: Vec1<Cursor>,
        line_pos: f64,
        col_pos: f64,
    },
    File {
        file: PathBuf,
        cursors: Vec1<Cursor>,
        line_pos: f64,
        col_pos: f64,
    },
    FileExplorer(PathBuf),
    Logger,
}

pub struct JumpList {
    stack: Vec<JumpPoint>,
    current_point: i64,
}

impl JumpList {
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            current_point: -1,
        }
    }

    pub fn push(&mut self, jump_point: JumpPoint) {
        // TODO: check if jump point is the same as the last one
        // and don't save it if they are too similar
        self.stack.truncate((self.current_point + 1) as usize);
        self.stack.push(jump_point);
        self.current_point += 1;
    }

    pub fn jump_back(&mut self, mut jump_point: JumpPoint) -> Option<JumpPoint> {
        if self.current_point.is_negative() {
            return None;
        }
        let point = &mut self.stack[self.current_point as usize];
        std::mem::swap(point, &mut jump_point);
        self.current_point -= 1;
        Some(jump_point)
    }

    pub fn jump_forward(&mut self, mut jump_point: JumpPoint) -> Option<JumpPoint> {
        if self.current_point + 1 >= self.stack.len() as i64 {
            return None;
        }
        self.current_point += 1;
        let point = &mut self.stack[self.current_point as usize];
        std::mem::swap(point, &mut jump_point);
        Some(jump_point)
    }
}

impl Default for JumpList {
    fn default() -> Self {
        Self::new()
    }
}
