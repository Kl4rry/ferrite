use std::{mem, str::FromStr};

use anyhow::bail;
use serde::{Deserialize, Serialize};
use slotmap::Key;

use crate::{buffer::ViewId, file_explorer::FileExplorerId, workspace::BufferId};

// TODO: Remove this
pub type Rect = ferrite_utility::geom::Rect<usize>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PaneKind {
    Buffer(BufferId, ViewId),
    FileExplorer(FileExplorerId),
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

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
enum Pane {
    Leaf(PaneKind),
    Internal {
        left: Box<Pane>,
        right: Box<Pane>,
        split: Split,
        ratio: f32,
    },
}

impl Pane {
    pub fn replace(&mut self, old: PaneKind, new: PaneKind) -> bool {
        match self {
            Pane::Leaf(leaf) => {
                if *leaf == old {
                    *leaf = new;
                    true
                } else {
                    false
                }
            }
            Pane::Internal { left, right, .. } => left.replace(old, new) || right.replace(old, new),
        }
    }

    fn get_first_leaf(&self) -> PaneKind {
        match self {
            Pane::Leaf(leaf) => *leaf,
            Pane::Internal { left, .. } => left.get_first_leaf(),
        }
    }

    pub fn remove(&mut self, pane_kind: PaneKind) -> Option<PaneKind> {
        let mut new = None;
        let mut output = None;
        'block: {
            match self {
                Pane::Leaf(_) => return None,
                Pane::Internal { left, right, .. } => {
                    match &mut **left {
                        Pane::Leaf(leaf) => {
                            if *leaf == pane_kind {
                                let mut dummy =
                                    Pane::Leaf(PaneKind::Buffer(BufferId::null(), ViewId::null()));
                                mem::swap(&mut dummy, &mut **right);
                                output = Some(dummy.get_first_leaf());
                                new = Some(dummy);
                                break 'block;
                            }
                        }
                        node => {
                            if let Some(pane_kind) = node.remove(pane_kind) {
                                output.replace(pane_kind);
                                break 'block;
                            }
                        }
                    }

                    match &mut **right {
                        Pane::Leaf(leaf) => {
                            if *leaf == pane_kind {
                                let mut dummy =
                                    Pane::Leaf(PaneKind::Buffer(BufferId::null(), ViewId::null()));
                                mem::swap(&mut dummy, &mut **left);
                                output = Some(dummy.get_first_leaf());
                                new = Some(dummy);
                                break 'block;
                            }
                        }
                        node => {
                            if let Some(pane_kind) = node.remove(pane_kind) {
                                output.replace(pane_kind);
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
            Pane::Leaf(pane_kind) => {
                if current == *pane_kind {
                    let mut left = *pane_kind;
                    let mut right = new_pane;

                    if matches!(direction, Direction::Left | Direction::Up) {
                        mem::swap(&mut left, &mut right);
                    }

                    let split = Split::from(direction);

                    *self = Pane::Internal {
                        left: Box::new(Pane::Leaf(left)),
                        right: Box::new(Pane::Leaf(right)),
                        split,
                        ratio: 0.5,
                    };
                    true
                } else {
                    false
                }
            }

            Pane::Internal { left, right, .. } => {
                left.split(current, new_pane, direction)
                    || right.split(current, new_pane, direction)
            }
        }
    }

    pub fn num_panes(&self) -> usize {
        match self {
            Pane::Leaf(_) => 1,
            Pane::Internal { left, right, .. } => left.num_panes() + right.num_panes(),
        }
    }

    pub fn get_pane_bounds(&self, bounds: &mut Vec<(PaneKind, Rect)>, rect: Rect, padding: usize) {
        match self {
            Pane::Leaf(pane_kind) => bounds.push((*pane_kind, rect)),
            Pane::Internal {
                left,
                right,
                split,
                ratio,
            } => match split {
                Split::Horizontal => {
                    let first = (rect.height as f32 * ratio) as usize;
                    let second = rect.height - first;
                    left.get_pane_bounds(
                        bounds,
                        Rect::new(rect.x, rect.y, rect.width, first),
                        padding,
                    );
                    right.get_pane_bounds(
                        bounds,
                        Rect::new(rect.x, rect.y + first, rect.width, second),
                        padding,
                    );
                }
                Split::Vertical => {
                    let width = rect.width.saturating_sub(padding);
                    let first = (width as f32 * ratio) as usize;
                    let second = width - first;
                    left.get_pane_bounds(
                        bounds,
                        Rect::new(rect.x, rect.y, first, rect.height),
                        padding,
                    );
                    right.get_pane_bounds(
                        bounds,
                        Rect::new(rect.x + first + padding, rect.y, second, rect.height),
                        padding,
                    );
                }
            },
        }
    }

    fn contains(&self, pane_kind: PaneKind) -> bool {
        match self {
            Pane::Leaf(leaf) => *leaf == pane_kind,
            Pane::Internal { left, right, .. } => {
                left.contains(pane_kind) || right.contains(pane_kind)
            }
        }
    }

    fn contains_buffer(&self, id: BufferId) -> bool {
        match self {
            Pane::Leaf(leaf) => match leaf {
                PaneKind::Buffer(buffer_id, _) => *buffer_id == id,
                PaneKind::FileExplorer(_) => false,
                PaneKind::Logger => false,
            },
            Pane::Internal { left, right, .. } => {
                left.contains_buffer(id) || right.contains_buffer(id)
            }
        }
    }

    pub fn get_parent_size(&self, pane_kind: PaneKind, rect: Rect) -> Rect {
        if let Pane::Internal {
            left,
            right,
            split,
            ratio,
        } = self
        {
            for node in [left, right] {
                match &**node {
                    Pane::Leaf(leaf) => {
                        if *leaf == pane_kind {
                            return rect;
                        }
                    }
                    Pane::Internal { left, right, .. } => match split {
                        Split::Horizontal => {
                            let first = (rect.height as f32 * ratio) as usize;
                            let second = rect.height - first;
                            left.get_parent_size(
                                pane_kind,
                                Rect::new(rect.x, rect.y, rect.width, first),
                            );
                            right.get_parent_size(
                                pane_kind,
                                Rect::new(rect.x, rect.y + first, rect.width, second),
                            );
                        }
                        Split::Vertical => {
                            let first = (rect.width as f32 * ratio) as usize;
                            let second = rect.width - first;
                            left.get_parent_size(
                                pane_kind,
                                Rect::new(rect.x, rect.y, first, rect.height),
                            );
                            right.get_parent_size(
                                pane_kind,
                                Rect::new(rect.x + first, rect.y, second, rect.height),
                            );
                        }
                    },
                }
            }
        }
        rect
    }

    pub fn resize_pane(&mut self, pane_kind: PaneKind, rect: Rect, direction: f32) {
        debug_assert!(direction == -1.0 || direction == 1.0);
        let rect = self.get_parent_size(pane_kind, rect);
        if let Pane::Internal {
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
                Pane::Leaf(leaf) => {
                    if *leaf == pane_kind {
                        *ratio += diff;
                        *ratio = ratio.clamp(0.0, 1.0);
                        return;
                    }
                }
                node => node.resize_pane(pane_kind, rect, direction),
            }

            match &mut **right {
                Pane::Leaf(leaf) => {
                    if *leaf == pane_kind {
                        *ratio -= diff;
                        *ratio = ratio.clamp(0.0, 1.0);
                    }
                }
                node => node.resize_pane(pane_kind, rect, direction),
            }
        }
    }
}

#[derive(Debug)]
pub struct Panes {
    node: Pane,
    current_pane: PaneKind,
    pub padding: usize,
}

impl Panes {
    pub fn new(buffer_id: BufferId, view_id: ViewId) -> Panes {
        Self {
            node: Pane::Leaf(PaneKind::Buffer(buffer_id, view_id)),
            current_pane: PaneKind::Buffer(buffer_id, view_id),
            padding: 0,
        }
    }

    pub fn get_current_pane(&self) -> PaneKind {
        self.current_pane
    }

    pub fn replace_current(&mut self, pane_kind: PaneKind) -> PaneKind {
        if self.node.contains(pane_kind) {
            self.node.remove(pane_kind);
        }

        self.node.replace(self.current_pane, pane_kind);
        let old = self.current_pane;
        self.current_pane = pane_kind;
        old
    }

    pub fn replace(&mut self, old: PaneKind, new: PaneKind) {
        self.node.replace(old, new);
    }

    pub fn remove_pane(&mut self, pane_kind: PaneKind) -> bool {
        if self.node.num_panes() > 1 {
            self.current_pane = self.node.remove(pane_kind).unwrap();
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
        self.node.get_pane_bounds(&mut bounds, rect, self.padding);
        bounds
    }

    pub fn make_current(&mut self, pane_kind: PaneKind) {
        if self.node.contains(pane_kind) {
            self.current_pane = pane_kind;
        } else {
            tracing::error!("Tried to make non existant pane `{pane_kind:?}` current");
        }
    }

    pub fn grow_current(&mut self, rect: Rect) {
        self.node.resize_pane(self.current_pane, rect, 1.0);
    }

    pub fn shrink_current(&mut self, rect: Rect) {
        self.node.resize_pane(self.current_pane, rect, -1.0);
    }

    pub fn contains(&self, pane_kind: PaneKind) -> bool {
        self.node.contains(pane_kind)
    }

    pub fn contains_buffer(&self, buffer_id: BufferId) -> bool {
        self.node.contains_buffer(buffer_id)
    }

    pub fn ensure_current_pane_exists(&mut self) {
        if !self.contains(self.get_current_pane()) {
            let pane = self.node.get_first_leaf();
            self.make_current(pane);
        }
    }

    pub fn switch_pane_direction(&mut self, dir: Direction, rect: Rect) {
        let bounds = self.get_pane_bounds(rect);
        let current = self.get_current_pane();
        let (_, cb) = bounds.iter().find(|(pane, _)| *pane == current).unwrap();
        let bounds_check = match dir {
            Direction::Up => Rect::new(cb.x, cb.y.saturating_sub(1), 1, 1),
            Direction::Down => Rect::new(cb.x, cb.y + cb.height + 1, 1, 1),
            Direction::Right => Rect::new(cb.x + cb.width + 1, cb.y, 1, 1),
            Direction::Left => Rect::new(cb.x.saturating_sub(2), cb.y, 1, 1),
        };

        // This is retarded and I can't be bother to figure out why its needed
        if cb.y == 0 && dir == Direction::Up {
            return;
        }

        if let Some((new_pane, _)) = bounds
            .iter()
            .find(|(pane, bounds)| *pane != current && bounds.intersects(&bounds_check))
        {
            self.current_pane = *new_pane;
        };
    }
}

#[cfg(test)]
mod tests {
    use slotmap::KeyData;

    use super::*;

    #[test]
    fn replace_current() {
        let mut panes = Panes::new(
            BufferId::from(KeyData::from_ffi(0)),
            ViewId::from(KeyData::from_ffi(0)),
        );
        panes.replace_current(PaneKind::Buffer(
            BufferId::from(KeyData::from_ffi(1)),
            ViewId::from(KeyData::from_ffi(1)),
        ));
        assert_eq!(
            panes.get_current_pane(),
            PaneKind::Buffer(
                BufferId::from(KeyData::from_ffi(1)),
                ViewId::from(KeyData::from_ffi(1))
            )
        );
    }
}

pub mod layout {
    use std::{
        collections::HashMap,
        ffi::OsString,
        path::{Path, PathBuf},
    };

    use ferrite_utility::vec1::Vec1;
    use serde::{Deserialize, Serialize};
    use slotmap::SlotMap;

    use super::{Pane, Panes, Split};
    use crate::{
        buffer::{Buffer, cursor::Cursor},
        file_explorer::{FileExplorer, FileExplorerId},
        workspace::BufferId,
    };

    #[derive(Debug, Serialize, Deserialize)]
    pub struct Layout {
        node: Option<Node>,
        current_pane: Option<PaneKind>,
    }

    #[derive(Debug, Serialize, Deserialize)]
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
        fn contains_path(&self, path: &Path) -> bool {
            match self {
                Node::Leaf(PaneKind::Buffer { path: buffer, .. }) => buffer == path,
                Node::Leaf(_) => false,
                Node::Internal { left, right, .. } => {
                    left.contains_path(path) || right.contains_path(path)
                }
            }
        }

        fn from_pane_node(
            pane: &Pane,
            buffers: &SlotMap<BufferId, Buffer>,
            file_explorers: &SlotMap<FileExplorerId, FileExplorer>,
        ) -> Option<Self> {
            match pane {
                Pane::Leaf(pane_kind) => match pane_kind {
                    super::PaneKind::Buffer(buffer_id, view_id) => {
                        let buffer = buffers.get(*buffer_id)?;
                        let path = buffer.file()?.to_path_buf();
                        let view = &buffer.views[*view_id];
                        Some(Self::Leaf(PaneKind::Buffer {
                            path,
                            cursor: *view.cursors.first(),
                            line_pos: view.line_pos_floored(),
                            col_pos: view.col_pos_floored(),
                        }))
                    }
                    super::PaneKind::FileExplorer(file_explorer_id) => {
                        let file_explorer = &file_explorers[*file_explorer_id];
                        Some(Self::Leaf(PaneKind::FileExplorer {
                            path: file_explorer.directory().into(),
                            history: file_explorer.history.clone(),
                        }))
                    }
                    super::PaneKind::Logger => Some(Self::Leaf(PaneKind::Logger)),
                },
                Pane::Internal {
                    left,
                    right,
                    split,
                    ratio,
                } => {
                    let left = Node::from_pane_node(left, buffers, file_explorers);
                    let right = Node::from_pane_node(right, buffers, file_explorers);
                    match (left, right) {
                        (Some(left), Some(right)) => Some(Node::Internal {
                            left: Box::new(left),
                            right: Box::new(right),
                            split: *split,
                            ratio: *ratio,
                        }),
                        (Some(left), None) => Some(left),
                        (None, Some(right)) => Some(right),
                        (None, None) => None,
                    }
                }
            }
        }

        fn to_pane(
            &self,
            buffers: &mut SlotMap<BufferId, Buffer>,
            file_explorers: &mut SlotMap<FileExplorerId, FileExplorer>,
        ) -> Option<Pane> {
            match self {
                Node::Leaf(pane_kind) => match pane_kind {
                    PaneKind::Buffer {
                        path,
                        cursor,
                        line_pos,
                        col_pos,
                    } => {
                        let (buffer_id, buffer) =
                            buffers.iter_mut().find(|(_, buffer)| match buffer.file() {
                                Some(buffer_path) => buffer_path == path,
                                None => false,
                            })?;
                        let view_id = buffer.create_view();
                        let view = &mut buffer.views[view_id];
                        view.cursors = Vec1::new(*cursor);
                        view.line_pos = *line_pos as f64;
                        view.col_pos = *col_pos as f64;
                        buffer.ensure_cursors_are_valid(view_id);

                        Some(super::Pane::Leaf(super::PaneKind::Buffer(
                            buffer_id, view_id,
                        )))
                    }
                    PaneKind::FileExplorer { path, history } => {
                        Some(super::Pane::Leaf(super::PaneKind::FileExplorer({
                            let mut fe = FileExplorer::new(path.into());
                            fe.history = history.clone();
                            file_explorers.insert(fe)
                        })))
                    }
                    PaneKind::Logger => Some(super::Pane::Leaf(super::PaneKind::Logger)),
                },
                Node::Internal {
                    left,
                    right,
                    split,
                    ratio,
                } => {
                    let left = left.to_pane(buffers, file_explorers);
                    let right = right.to_pane(buffers, file_explorers);
                    match (left, right) {
                        (Some(left), Some(right)) => Some(super::Pane::Internal {
                            left: Box::new(left),
                            right: Box::new(right),
                            split: *split,
                            ratio: *ratio,
                        }),
                        (Some(left), None) => Some(left),
                        (None, Some(right)) => Some(right),
                        (None, None) => None,
                    }
                }
            }
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    enum PaneKind {
        Buffer {
            path: PathBuf,
            cursor: Cursor,
            line_pos: usize,
            col_pos: usize,
        },
        FileExplorer {
            path: PathBuf,
            history: HashMap<PathBuf, OsString>,
        },
        Logger,
    }

    impl Layout {
        pub fn to_panes(
            &self,
            buffers: &mut SlotMap<BufferId, Buffer>,
            file_explorers: &mut SlotMap<FileExplorerId, FileExplorer>,
        ) -> Option<super::Panes> {
            let pane = self.node.as_ref()?.to_pane(buffers, file_explorers)?;
            let pane_kind = match &self.current_pane {
                Some(PaneKind::Buffer {
                    path,
                    cursor,
                    line_pos,
                    col_pos,
                }) => {
                    match buffers
                        .iter_mut()
                        .find(|(_, buffer)| buffer.file() == Some(path))
                    {
                        Some((buffer_id, buffer)) => {
                            let view_id = buffer.create_view();
                            let view = &mut buffer.views[view_id];
                            view.cursors = Vec1::new(*cursor);
                            view.line_pos = *line_pos as f64;
                            view.col_pos = *col_pos as f64;
                            super::PaneKind::Buffer(buffer_id, view_id)
                        }
                        None => pane.get_first_leaf(),
                    }
                }
                Some(PaneKind::FileExplorer { path, history }) => {
                    let mut fe = FileExplorer::new(path.into());
                    fe.history = history.clone();
                    let file_explorer_id = file_explorers.insert(fe);
                    super::PaneKind::FileExplorer(file_explorer_id)
                }
                Some(PaneKind::Logger) => super::PaneKind::Logger,
                None => pane.get_first_leaf(),
            };
            let current_pane = pane_kind;
            Some(super::Panes {
                node: pane,
                current_pane,
                padding: 0,
            })
        }

        pub fn from_panes(
            panes: &Panes,
            buffers: &SlotMap<BufferId, Buffer>,
            file_explorers: &SlotMap<FileExplorerId, FileExplorer>,
        ) -> Self {
            let node = Node::from_pane_node(&panes.node, buffers, file_explorers);
            let current_pane = match panes.current_pane {
                super::PaneKind::Buffer(buffer_id, view_id) => {
                    let path = buffers[buffer_id].file();
                    path.and_then(|path| {
                        node.as_ref().map(|node| {
                            if node.contains_path(path) {
                                let view = &buffers[buffer_id].views[view_id];
                                Some(PaneKind::Buffer {
                                    path: path.into(),
                                    cursor: *view.cursors.first(),
                                    line_pos: view.line_pos_floored(),
                                    col_pos: view.col_pos_floored(),
                                })
                            } else {
                                None
                            }
                        })
                    })
                    .flatten()
                }
                super::PaneKind::FileExplorer(file_explorer_id) => {
                    let fe = &file_explorers[file_explorer_id];
                    let directory = fe.directory();
                    Some(PaneKind::FileExplorer {
                        path: directory.into(),
                        history: fe.history.clone(),
                    })
                }
                super::PaneKind::Logger => Some(PaneKind::Logger),
            };
            Self { node, current_pane }
        }
    }
}
