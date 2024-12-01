use winit::keyboard::NamedKey;

pub fn convert_style(
    style: &ferrite_core::theme::style::Style,
) -> (Option<glyphon::Color>, Option<glyphon::Color>) {
    (
        style.fg.as_ref().map(|color| {
            glyphon::Color::rgb(
                (color.r * 255.0) as u8,
                (color.g * 255.0) as u8,
                (color.b * 255.0) as u8,
            )
        }),
        style.bg.as_ref().map(|color| {
            glyphon::Color::rgb(
                (color.r * 255.0) as u8,
                (color.g * 255.0) as u8,
                (color.b * 255.0) as u8,
            )
        }),
    )
}

pub fn convert_keycode(
    named_key: winit::keyboard::NamedKey,
) -> Option<ferrite_core::keymap::keycode::KeyCode> {
    let key = match named_key {
        NamedKey::Backspace => ferrite_core::keymap::keycode::KeyCode::Backspace,
        NamedKey::Enter => ferrite_core::keymap::keycode::KeyCode::Enter,
        NamedKey::ArrowLeft => ferrite_core::keymap::keycode::KeyCode::Left,
        NamedKey::ArrowRight => ferrite_core::keymap::keycode::KeyCode::Right,
        NamedKey::ArrowUp => ferrite_core::keymap::keycode::KeyCode::Up,
        NamedKey::ArrowDown => ferrite_core::keymap::keycode::KeyCode::Down,
        NamedKey::Home => ferrite_core::keymap::keycode::KeyCode::Home,
        NamedKey::End => ferrite_core::keymap::keycode::KeyCode::End,
        NamedKey::PageUp => ferrite_core::keymap::keycode::KeyCode::PageUp,
        NamedKey::PageDown => ferrite_core::keymap::keycode::KeyCode::PageDown,
        NamedKey::Tab => ferrite_core::keymap::keycode::KeyCode::Tab,
        NamedKey::Delete => ferrite_core::keymap::keycode::KeyCode::Delete,
        NamedKey::Insert => ferrite_core::keymap::keycode::KeyCode::Insert,
        NamedKey::Escape => ferrite_core::keymap::keycode::KeyCode::Esc,
        NamedKey::CapsLock => ferrite_core::keymap::keycode::KeyCode::CapsLock,
        NamedKey::ScrollLock => ferrite_core::keymap::keycode::KeyCode::ScrollLock,
        NamedKey::NumLock => ferrite_core::keymap::keycode::KeyCode::NumLock,
        NamedKey::PrintScreen => ferrite_core::keymap::keycode::KeyCode::PrintScreen,
        NamedKey::Pause => ferrite_core::keymap::keycode::KeyCode::Pause,
        _ => return None,
    };
    Some(key)
}
