use std::time::{Duration, Instant};

use ferrite_geom::rect::Vec2;
use ferrite_style::Color;

use crate::{
    any_view::AnyView,
    event_loop_proxy::EventLoopControlFlow,
    id::Id,
    input::{event::InputEvent, keycode::KeyModifiers},
};

pub mod any_view;
pub mod event_loop_proxy;
pub mod id;
pub mod input;
pub mod painter;
pub mod unique_id;

pub use painter::{Bounds, Painter};

pub type Input<S, E> =
    fn(state: &mut S, event: InputEvent<E>, control_flow: &mut EventLoopControlFlow);
pub type Update<S> = fn(runtime: &mut Runtime<S>, control_flow: &mut EventLoopControlFlow);
pub type Layout<S> = for<'a> fn(state: &'a mut S) -> AnyView<S>;
pub type StartOfFrame<S> = fn(runtime: &mut Runtime<S>);

pub trait View<State> {
    fn handle_mouse(
        &self,
        _state: &mut State,
        _bounds: Bounds,
        _mouse_interaction: MouseInterction,
    ) -> bool {
        false
    }
    fn render(&self, state: &mut State, bounds: Bounds, painter: &mut Painter);
}

pub struct Runtime<S> {
    pub state: S,
    pub scale: f32,
    pub font_weight: u16,
    pub font_family: String,
    pub default_bg: Color,
    pub start_of_events: Instant,
    pub last_render_time: Duration,
    pub force_redraw: bool,
}

impl<S> Runtime<S> {
    pub fn new(state: S) -> Self {
        Self {
            state,
            scale: 1.0,
            font_weight: 400,
            font_family: "FiraCode Nerd Font Mono".into(),
            start_of_events: Instant::now(),
            last_render_time: Duration::ZERO,
            force_redraw: false,
            default_bg: Color::new(0.0, 0.0, 0.0),
        }
    }
}

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
}
