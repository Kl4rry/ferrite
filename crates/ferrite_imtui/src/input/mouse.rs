use std::time::Instant;

use ferrite_geom::rect::Vec2;

use crate::input::keycode::KeyModifiers;

#[derive(Debug, Clone)]
pub struct MouseButtonState {
    pub pressed: bool,
    pub last_press: Instant,
    pub clicks: usize,
    pub drag_start: Option<Vec2<f32>>,
}

impl Default for MouseButtonState {
    fn default() -> Self {
        Self {
            pressed: false,
            last_press: Instant::now(),
            clicks: 1,
            drag_start: None,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct MouseState {
    pub left: MouseButtonState,
    pub right: MouseButtonState,
    pub middle: MouseButtonState,
    pub position: Vec2<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MouseInterctionKind {
    Click(usize),
    Drag {
        drag_start: Vec2<f32>,
        last_pos: Vec2<f32>,
    },
    DragStop,
}

#[derive(Debug, Clone, Copy)]
pub struct MouseInterction {
    pub cell_size: Vec2<f32>,
    pub position: Vec2<f32>,
    pub button: MouseButton,
    pub kind: MouseInterctionKind,
    pub modifiers: KeyModifiers,
}

impl MouseInterction {
    pub fn cell_position(&self, offset: Vec2) -> Vec2<usize> {
        let offset = Vec2::new(offset.x as f32, offset.y as f32);
        if self.cell_size.x == 1.0 && self.cell_size.y == 1.0 {
            return Vec2::new(
                (self.position.x - offset.x) as usize,
                (self.position.y - offset.y) as usize,
            );
        }

        let cell_x = ((self.position.x - offset.x) / self.cell_size.x).round() as usize;
        let cell_y = ((self.position.y - offset.y) / self.cell_size.y) as usize;
        Vec2::new(cell_x, cell_y)
    }

    pub fn is_drag(&self) -> bool {
        matches!(
            self.kind,
            MouseInterctionKind::Drag { .. } | MouseInterctionKind::DragStop
        )
    }
}
