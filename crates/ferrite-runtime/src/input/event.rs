use crate::input::keycode::{KeyCode, KeyModifiers};

pub enum ScrollDelta {
    Line(f32, f32),
    Pixel(f32, f32),
}

pub enum InputEvent<E> {
    Key(KeyCode, KeyModifiers),
    Text(String),
    Paste(String),
    Scroll(ScrollDelta),
    UserEvent(E),
}
