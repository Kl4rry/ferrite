use std::path::PathBuf;

use ferrite_utility::vec1::Vec1;

use crate::{buffer::cursor::Cursor, workspace::BufferId};

#[derive(Clone, Debug, PartialEq)]
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
}

impl JumpPoint {
    pub fn is_similar(&self, other: &Self) -> bool {
        match (self, other) {
            (
                JumpPoint::Buffer {
                    buffer_id,
                    cursors,
                    line_pos,
                    col_pos,
                },
                JumpPoint::Buffer {
                    buffer_id: other_buffer_id,
                    cursors: other_cursors,
                    line_pos: other_line_pos,
                    col_pos: other_col_pos,
                },
            ) => {
                buffer_id == other_buffer_id
                    && cursors == other_cursors
                    && line_pos.floor() as i64 == other_line_pos.floor() as i64
                    && col_pos.floor() as i64 == other_col_pos.floor() as i64
            }
            (
                JumpPoint::File {
                    file,
                    cursors,
                    line_pos,
                    col_pos,
                },
                JumpPoint::File {
                    file: other_file,
                    cursors: other_cursors,
                    line_pos: other_line_pos,
                    col_pos: other_col_pos,
                },
            ) => {
                file == other_file
                    && cursors == other_cursors
                    && line_pos.floor() as i64 == other_line_pos.floor() as i64
                    && col_pos.floor() as i64 == other_col_pos.floor() as i64
            }
            (JumpPoint::FileExplorer(file), JumpPoint::FileExplorer(other_file)) => {
                file == other_file
            }
            _ => false,
        }
    }
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
        // Check if jump point is the same as the last one and don't save it if they are too similar
        if self.current_point >= 0
            && self
                .stack
                .get((self.current_point) as usize)
                .map(|current| current.is_similar(&jump_point))
                .unwrap_or(false)
        {
            return;
        }

        self.stack.truncate((self.current_point + 1) as usize);
        self.stack.push(jump_point);
        self.current_point = self.stack.len() as i64 - 1;
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

    pub fn remove_current(&mut self) {
        if let Ok(current_point) = (self.current_point + 1).try_into() {
            self.stack.remove(current_point);
        }
    }

    // These functions should only be used for persistance
    pub fn as_slice(&self) -> &[JumpPoint] {
        &self.stack
    }

    pub fn current_point(&self) -> i64 {
        self.current_point
    }

    pub fn from(jump_points: Vec<JumpPoint>, current_point: i64) -> Self {
        Self {
            current_point: current_point.clamp(-1, jump_points.len() as i64 - 1),
            stack: jump_points,
        }
    }
}

impl Default for JumpList {
    fn default() -> Self {
        Self::new()
    }
}
