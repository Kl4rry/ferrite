use crate::input::keycode::{KeyCode, KeyModifiers};

pub enum InputEvent<E> {
    Key(KeyCode, KeyModifiers),
    Text(String),
    Paste(String),
    Scroll(f32, f32),
    UserEvent(E),
}
