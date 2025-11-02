use std::time::{Duration, Instant};

use crate::{any_view::AnyView, id::Id, input::event::InputEvent};

pub mod any_view;
pub mod event_loop_proxy;
pub mod id;
pub mod input;
pub mod painter;
pub mod unique_id;

pub use painter::{Bounds, Painter};

pub type Input<S, E> = fn(state: &mut S, event: InputEvent<E>);
pub type Update<S> = fn(runtime: &mut Runtime<S>);
pub type Layout<S> = for<'a> fn(state: &'a mut S) -> AnyView<S>;

// pub struct MouseInterction {
//     cell_size: (f32, f32),
//     position: (f32, f32),
//     kind: i32, // (press, release, double click?, tripple click? drag?)
// }

pub trait View<State> {
    //fn handle_mouse(&self);
    fn render(&self, state: &mut State, bounds: Bounds, painter: &mut Painter);
}

pub struct Runtime<S> {
    pub state: S,
    pub scale: f32,
    pub font_weight: u16,
    pub font_family: String,
    pub start_of_events: Instant,
    pub last_render_time: Duration,
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
        }
    }

    pub fn request_render() {}
    pub fn submit_event() {}
}
