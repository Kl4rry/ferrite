use ferrite_geom::point::Point;
use serde::{Deserialize, Serialize};

pub fn intersects(start1: usize, end1: usize, start2: usize, end2: usize) -> bool {
    !(start1 > end2 || end1 < start2)
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Cursor {
    pub position: usize,
    pub anchor: usize,
    pub affinity: usize,
}

impl Cursor {
    pub fn has_selection(&self) -> bool {
        self.position != self.anchor
    }

    pub fn intersects(&self, other: Cursor) -> bool {
        let start1 = self.start();
        let end1 = self.end();
        let start2 = other.start();
        let end2 = other.end();
        intersects(start1, end1, start2, end2)
    }

    pub fn coalesce(self, other: Cursor) -> Self {
        if self.position >= self.anchor {
            Self {
                position: self.position.max(other.position),
                anchor: self.anchor.min(other.anchor),
                affinity: self.affinity,
            }
        } else {
            Self {
                position: self.position.min(other.position),
                anchor: self.anchor.max(other.anchor),
                affinity: self.affinity,
            }
        }
    }

    pub fn start(&self) -> usize {
        self.position.min(self.anchor)
    }

    pub fn end(&self) -> usize {
        self.position.max(self.anchor)
    }

    pub fn collapse(&mut self) {
        self.anchor = self.position;
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Selection {
    pub start: Point<i64>,
    pub end: Point<i64>,
}
