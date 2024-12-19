use ferrite_core::keymap::keycode::KeyModifiers;
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
    modifiers: KeyModifiers,
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
        NamedKey::Tab if !modifiers.contains(KeyModifiers::SHIFT) => {
            ferrite_core::keymap::keycode::KeyCode::Tab
        }
        NamedKey::Tab if modifiers.contains(KeyModifiers::SHIFT) => {
            ferrite_core::keymap::keycode::KeyCode::BackTab
        }
        NamedKey::Delete => ferrite_core::keymap::keycode::KeyCode::Delete,
        NamedKey::Insert => ferrite_core::keymap::keycode::KeyCode::Insert,
        NamedKey::Escape => ferrite_core::keymap::keycode::KeyCode::Esc,
        NamedKey::CapsLock => ferrite_core::keymap::keycode::KeyCode::CapsLock,
        NamedKey::ScrollLock => ferrite_core::keymap::keycode::KeyCode::ScrollLock,
        NamedKey::NumLock => ferrite_core::keymap::keycode::KeyCode::NumLock,
        NamedKey::PrintScreen => ferrite_core::keymap::keycode::KeyCode::PrintScreen,
        NamedKey::Pause => ferrite_core::keymap::keycode::KeyCode::Pause,
        NamedKey::F1 => ferrite_core::keymap::keycode::KeyCode::F(1),
        NamedKey::F2 => ferrite_core::keymap::keycode::KeyCode::F(2),
        NamedKey::F3 => ferrite_core::keymap::keycode::KeyCode::F(3),
        NamedKey::F4 => ferrite_core::keymap::keycode::KeyCode::F(4),
        NamedKey::F5 => ferrite_core::keymap::keycode::KeyCode::F(5),
        NamedKey::F6 => ferrite_core::keymap::keycode::KeyCode::F(6),
        NamedKey::F7 => ferrite_core::keymap::keycode::KeyCode::F(7),
        NamedKey::F8 => ferrite_core::keymap::keycode::KeyCode::F(8),
        NamedKey::F9 => ferrite_core::keymap::keycode::KeyCode::F(9),
        NamedKey::F10 => ferrite_core::keymap::keycode::KeyCode::F(10),
        NamedKey::F11 => ferrite_core::keymap::keycode::KeyCode::F(11),
        NamedKey::F12 => ferrite_core::keymap::keycode::KeyCode::F(12),
        NamedKey::F13 => ferrite_core::keymap::keycode::KeyCode::F(13),
        NamedKey::F14 => ferrite_core::keymap::keycode::KeyCode::F(14),
        NamedKey::F15 => ferrite_core::keymap::keycode::KeyCode::F(15),
        NamedKey::F16 => ferrite_core::keymap::keycode::KeyCode::F(16),
        NamedKey::F17 => ferrite_core::keymap::keycode::KeyCode::F(17),
        NamedKey::F18 => ferrite_core::keymap::keycode::KeyCode::F(18),
        NamedKey::F19 => ferrite_core::keymap::keycode::KeyCode::F(19),
        NamedKey::F20 => ferrite_core::keymap::keycode::KeyCode::F(20),
        NamedKey::F21 => ferrite_core::keymap::keycode::KeyCode::F(21),
        NamedKey::F22 => ferrite_core::keymap::keycode::KeyCode::F(22),
        NamedKey::F23 => ferrite_core::keymap::keycode::KeyCode::F(23),
        NamedKey::F24 => ferrite_core::keymap::keycode::KeyCode::F(24),
        _ => return None,
    };
    Some(key)
}
