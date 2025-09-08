use ferrite_geom::rect::Rect;

pub type Update<S> = fn(state: &mut S);
pub type Layout<S> = fn(state: &mut S);

pub struct Painter {
    buf: tui::buffer::Buffer,
}

pub trait Platform<S> {}

// pub struct MouseInterction {
//     grid_size: (f32, f32),
//     position: (f32, f32),
//     kind: i32, // (press, release, double click?, tripple click?)
// }

// View is a lightway wrapper around a handle pointing to a ui element
pub trait View<S> {
    // fn layout(&self, state: &mut S); runs if resized or state changed
    fn handle_mouse(&self, state: &mut S);
    fn render(&self, state: &S, rect: Rect, painter: &mut Painter);
}

pub struct Runtime<S> {
    pub state: S,
}

impl<S> Runtime<S> {
    pub fn new(state: S) -> Self {
        Self { state }
    }

    pub fn request_render() {}
    pub fn submit_event() {}
}
