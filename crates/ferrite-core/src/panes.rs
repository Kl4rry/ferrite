use std::{mem, str::FromStr};

use anyhow::bail;
use serde::{Deserialize, Serialize};
use slotmap::Key;

use crate::workspace::BufferId;

#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
}

impl Rect {
    pub fn new(x: usize, y: usize, width: usize, height: usize) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneKind {
    Buffer(BufferId),
    Logger,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum Split {
    Horizontal,
    Vertical,
}

impl From<Direction> for Split {
    fn from(value: Direction) -> Self {
        match value {
            Direction::Up | Direction::Down => Split::Horizontal,
            Direction::Right | Direction::Left => Split::Vertical,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Right,
    Left,
}

impl FromStr for Direction {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "up" => Direction::Up,
            "down" => Direction::Down,
            "left" => Direction::Left,
            "right" => Direction::Right,
            _ => bail!("Unkown direction: {s}"),
        })
    }
}

#[derive(Debug)]
enum Node {
    Leaf(PaneKind),
    Internal {
        left: Box<Node>,
        right: Box<Node>,
        split: Split,
        ratio: f32,
    },
}

impl Node {
    pub fn replace(&mut self, old: PaneKind, new: PaneKind) -> bool {
        match self {
            Node::Leaf(leaf) => {
                if *leaf == old {
                    *leaf = new;
                    true
                } else {
                    false
                }
            }
            Node::Internal { left, right, .. } => left.replace(old, new) || right.replace(old, new),
        }
    }

    fn get_first_leaf(&self) -> PaneKind {
        match self {
            Node::Leaf(leaf) => *leaf,
            Node::Internal { left, .. } => left.get_first_leaf(),
        }
    }

    pub fn remove(&mut self, pane: PaneKind) -> Option<PaneKind> {
        let mut new = None;
        let mut output = None;
        'block: {
            match self {
                Node::Leaf(_) => return None,
                Node::Internal { left, right, .. } => {
                    match &mut **left {
                        Node::Leaf(leaf) => {
                            if *leaf == pane {
                                let mut dummy = Node::Leaf(PaneKind::Buffer(BufferId::null()));
                                mem::swap(&mut dummy, &mut **right);
                                output = Some(dummy.get_first_leaf());
                                new = Some(dummy);
                                break 'block;
                            }
                        }
                        node => {
                            if let Some(pane) = node.remove(pane) {
                                output.replace(pane);
                                break 'block;
                            }
                        }
                    }

                    match &mut **right {
                        Node::Leaf(leaf) => {
                            if *leaf == pane {
                                let mut dummy = Node::Leaf(PaneKind::Buffer(BufferId::null()));
                                mem::swap(&mut dummy, &mut **left);
                                output = Some(dummy.get_first_leaf());
                                new = Some(dummy);
                                break 'block;
                            }
                        }
                        node => {
                            if let Some(pane) = node.remove(pane) {
                                output.replace(pane);
                                break 'block;
                            }
                        }
                    }
                }
            }
        }
        if let Some(new) = new {
            *self = new;
        }
        output
    }

    pub fn split(&mut self, current: PaneKind, new_pane: PaneKind, direction: Direction) -> bool {
        match self {
            Node::Leaf(pane_kind) => {
                if current == *pane_kind {
                    let mut left = *pane_kind;
                    let mut right = new_pane;

                    if matches!(direction, Direction::Left | Direction::Up) {
                        mem::swap(&mut left, &mut right);
                    }

                    let split = Split::from(direction);

                    *self = Node::Internal {
                        left: Box::new(Node::Leaf(left)),
                        right: Box::new(Node::Leaf(right)),
                        split,
                        ratio: 0.5,
                    };
                    true
                } else {
                    false
                }
            }

            Node::Internal { left, right, .. } => {
                left.split(current, new_pane, direction)
                    || right.split(current, new_pane, direction)
            }
        }
    }

    pub fn num_panes(&self) -> usize {
        match self {
            Node::Leaf(_) => 1,
            Node::Internal { left, right, .. } => left.num_panes() + right.num_panes(),
        }
    }

    pub fn get_pane_bounds(&self, bounds: &mut Vec<(PaneKind, Rect)>, rect: Rect) {
        match self {
            Node::Leaf(pane) => bounds.push((*pane, rect)),
            Node::Internal {
                left,
                right,
                split,
                ratio,
            } => match split {
                Split::Horizontal => {
                    let first = (rect.height as f32 * ratio) as usize;
                    let second = rect.height - first;
                    left.get_pane_bounds(bounds, Rect::new(rect.x, rect.y, rect.width, first));
                    right.get_pane_bounds(
                        bounds,
                        Rect::new(rect.x, rect.y + first, rect.width, second),
                    );
                }
                Split::Vertical => {
                    let first = (rect.width as f32 * ratio) as usize;
                    let second = rect.width - first;
                    left.get_pane_bounds(bounds, Rect::new(rect.x, rect.y, first, rect.height));
                    right.get_pane_bounds(
                        bounds,
                        Rect::new(rect.x + first, rect.y, second, rect.height),
                    );
                }
            },
        }
    }

    fn contains(&self, pane: PaneKind) -> bool {
        match self {
            Node::Leaf(leaf) => *leaf == pane,
            Node::Internal { left, right, .. } => left.contains(pane) || right.contains(pane),
        }
    }

    pub fn get_parent_size(&self, pane: PaneKind, rect: Rect) -> Rect {
        if let Node::Internal {
            left,
            right,
            split,
            ratio,
        } = self
        {
            for node in [left, right] {
                match &**node {
                    Node::Leaf(leaf) => {
                        if *leaf == pane {
                            return rect;
                        }
                    }
                    Node::Internal { left, right, .. } => match split {
                        Split::Horizontal => {
                            let first = (rect.height as f32 * ratio) as usize;
                            let second = rect.height - first;
                            left.get_parent_size(
                                pane,
                                Rect::new(rect.x, rect.y, rect.width, first),
                            );
                            right.get_parent_size(
                                pane,
                                Rect::new(rect.x, rect.y + first, rect.width, second),
                            );
                        }
                        Split::Vertical => {
                            let first = (rect.width as f32 * ratio) as usize;
                            let second = rect.width - first;
                            left.get_parent_size(
                                pane,
                                Rect::new(rect.x, rect.y, first, rect.height),
                            );
                            right.get_parent_size(
                                pane,
                                Rect::new(rect.x + first, rect.y, second, rect.height),
                            );
                        }
                    },
                }
            }
        }
        rect
    }

    pub fn resize_pane(&mut self, pane: PaneKind, rect: Rect, direction: f32) {
        debug_assert!(direction == -1.0 || direction == 1.0);
        let rect = self.get_parent_size(pane, rect);
        if let Node::Internal {
            left,
            right,
            split,
            ratio,
        } = self
        {
            let size = match split {
                Split::Horizontal => rect.height,
                Split::Vertical => rect.width,
            };

            let diff = (1.0 / size as f32) * direction;

            match &mut **left {
                Node::Leaf(leaf) => {
                    if *leaf == pane {
                        *ratio += diff;
                        *ratio = ratio.clamp(0.0, 1.0);
                        return;
                    }
                }
                node => node.resize_pane(pane, rect, direction),
            }

            match &mut **right {
                Node::Leaf(leaf) => {
                    if *leaf == pane {
                        *ratio -= diff;
                        *ratio = ratio.clamp(0.0, 1.0);
                    }
                }
                node => node.resize_pane(pane, rect, direction),
            }
        }
    }
}

#[derive(Debug)]
pub struct Panes {
    node: Node,
    current_pane: PaneKind,
}

impl Panes {
    pub fn new(buffer_id: BufferId) -> Panes {
        Self {
            node: Node::Leaf(PaneKind::Buffer(buffer_id)),
            current_pane: PaneKind::Buffer(buffer_id),
        }
    }

    pub fn get_current_pane(&self) -> PaneKind {
        self.current_pane
    }

    pub fn replace_current(&mut self, pane: PaneKind) -> PaneKind {
        if self.node.contains(pane) {
            self.node.remove(pane);
        }

        self.node.replace(self.current_pane, pane);
        let old = self.current_pane;
        self.current_pane = pane;
        old
    }

    pub fn remove_pane(&mut self, pane: PaneKind) -> bool {
        if self.node.num_panes() > 1 {
            self.current_pane = self.node.remove(pane).unwrap();
            true
        } else {
            false
        }
    }

    pub fn split(&mut self, new_pane: PaneKind, direction: Direction) {
        if self.node.split(self.current_pane, new_pane, direction) {
            self.current_pane = new_pane;
        }
    }

    pub fn num_panes(&self) -> usize {
        self.node.num_panes()
    }

    pub fn get_pane_bounds(&self, rect: Rect) -> Vec<(PaneKind, Rect)> {
        let mut bounds = Vec::new();
        self.node.get_pane_bounds(&mut bounds, rect);
        bounds
    }

    pub fn make_current(&mut self, pane: PaneKind) {
        if self.node.contains(pane) {
            self.current_pane = pane;
        } else {
            tracing::error!("Tried to make non existant pane `{pane:?}` current");
        }
    }

    pub fn grow_current(&mut self, rect: Rect) {
        self.node.resize_pane(self.current_pane, rect, 1.0);
    }

    pub fn shrink_current(&mut self, rect: Rect) {
        self.node.resize_pane(self.current_pane, rect, -1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replace_current() {
        let mut panes = Panes::new(0);
        panes.replace_current(PaneKind::Buffer(1));
        assert_eq!(panes.get_current_pane(), PaneKind::Buffer(1));
    }
}
