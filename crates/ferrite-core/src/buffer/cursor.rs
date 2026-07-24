use ferrite_geom::point::Point;
use ferrite_utility::vec1::Vec1;
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
    pub fn new(position: usize, anchor: usize) -> Self {
        Self {
            position,
            anchor,
            affinity: position,
        }
    }

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

pub fn coalesce_cursors(cursors: &mut Vec1<Cursor>) {
    let mut i = 0;
    while i < cursors.len() {
        let mut j = 0;
        while j < cursors.len() {
            if i != j && cursors[i].intersects(cursors[j]) {
                if i < j {
                    let cursor = cursors.remove(j).unwrap();
                    cursors[i] = cursors[i].coalesce(cursor);
                    j -= 1;
                } else
                /* i > j */
                {
                    let cursor = cursors.remove(i).unwrap();
                    cursors[j] = cursors[j].coalesce(cursor);
                    i -= 1;
                }
            }
            j += 1;
        }
        i += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple() {
        let mut vec = Vec1::from_vec(vec![Cursor::new(10, 20), Cursor::new(15, 30)]).unwrap();
        coalesce_cursors(&mut vec);
        assert!(vec.len() == 1);
        assert!(vec[0].position == 10);
        assert!(vec[0].anchor == 30);
    }

    #[test]
    fn simple_out_of_order() {
        let mut vec = Vec1::from_vec(vec![Cursor::new(15, 30), Cursor::new(10, 20)]).unwrap();
        coalesce_cursors(&mut vec);
        assert!(vec.len() == 1);
        assert!(vec[0].position == 10);
        assert!(vec[0].anchor == 30);
    }

    #[test]
    fn touching() {
        let mut vec = Vec1::from_vec(vec![Cursor::new(10, 20), Cursor::new(20, 30)]).unwrap();
        coalesce_cursors(&mut vec);
        assert!(vec.len() == 1);
        assert!(vec[0].position == 10);
        assert!(vec[0].anchor == 30);
    }

    #[test]
    fn with_gap() {
        let mut vec = Vec1::from_vec(vec![
            Cursor::new(10, 20),
            Cursor::new(35, 40),
            Cursor::new(15, 30),
        ])
        .unwrap();
        coalesce_cursors(&mut vec);
        assert!(vec.len() == 2);
        assert!(vec[0].position == 10);
        assert!(vec[0].anchor == 30);
        assert!(vec[1].position == 35);
        assert!(vec[1].anchor == 40);
    }

    #[test]
    fn many() {
        let mut vec = Vec1::from_vec(vec![
            Cursor::new(10, 20),
            Cursor::new(30, 40),
            Cursor::new(15, 30),
            Cursor::new(7, 26),
            Cursor::new(40, 60),
        ])
        .unwrap();
        coalesce_cursors(&mut vec);
        assert!(vec.len() == 1);
        assert!(vec[0].position == 7);
        assert!(vec[0].anchor == 60);
    }
}
