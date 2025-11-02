use ferrite_runtime::input::keycode::KeyModifiers;
use winit::keyboard::NamedKey;

pub fn convert_keycode(
    named_key: winit::keyboard::NamedKey,
    modifiers: KeyModifiers,
) -> Option<ferrite_runtime::input::keycode::KeyCode> {
    let key = match named_key {
        NamedKey::Backspace => ferrite_runtime::input::keycode::KeyCode::Backspace,
        NamedKey::Enter => ferrite_runtime::input::keycode::KeyCode::Enter,
        NamedKey::ArrowLeft => ferrite_runtime::input::keycode::KeyCode::Left,
        NamedKey::ArrowRight => ferrite_runtime::input::keycode::KeyCode::Right,
        NamedKey::ArrowUp => ferrite_runtime::input::keycode::KeyCode::Up,
        NamedKey::ArrowDown => ferrite_runtime::input::keycode::KeyCode::Down,
        NamedKey::Home => ferrite_runtime::input::keycode::KeyCode::Home,
        NamedKey::End => ferrite_runtime::input::keycode::KeyCode::End,
        NamedKey::PageUp => ferrite_runtime::input::keycode::KeyCode::PageUp,
        NamedKey::PageDown => ferrite_runtime::input::keycode::KeyCode::PageDown,
        NamedKey::Tab if !modifiers.contains(KeyModifiers::SHIFT) => {
            ferrite_runtime::input::keycode::KeyCode::Tab
        }
        NamedKey::Tab if modifiers.contains(KeyModifiers::SHIFT) => {
            ferrite_runtime::input::keycode::KeyCode::BackTab
        }
        NamedKey::Delete => ferrite_runtime::input::keycode::KeyCode::Delete,
        NamedKey::Insert => ferrite_runtime::input::keycode::KeyCode::Insert,
        NamedKey::Escape => ferrite_runtime::input::keycode::KeyCode::Esc,
        NamedKey::CapsLock => ferrite_runtime::input::keycode::KeyCode::CapsLock,
        NamedKey::ScrollLock => ferrite_runtime::input::keycode::KeyCode::ScrollLock,
        NamedKey::NumLock => ferrite_runtime::input::keycode::KeyCode::NumLock,
        NamedKey::PrintScreen => ferrite_runtime::input::keycode::KeyCode::PrintScreen,
        NamedKey::Pause => ferrite_runtime::input::keycode::KeyCode::Pause,
        NamedKey::F1 => ferrite_runtime::input::keycode::KeyCode::F1,
        NamedKey::F2 => ferrite_runtime::input::keycode::KeyCode::F2,
        NamedKey::F3 => ferrite_runtime::input::keycode::KeyCode::F3,
        NamedKey::F4 => ferrite_runtime::input::keycode::KeyCode::F4,
        NamedKey::F5 => ferrite_runtime::input::keycode::KeyCode::F5,
        NamedKey::F6 => ferrite_runtime::input::keycode::KeyCode::F6,
        NamedKey::F7 => ferrite_runtime::input::keycode::KeyCode::F7,
        NamedKey::F8 => ferrite_runtime::input::keycode::KeyCode::F8,
        NamedKey::F9 => ferrite_runtime::input::keycode::KeyCode::F9,
        NamedKey::F10 => ferrite_runtime::input::keycode::KeyCode::F10,
        NamedKey::F11 => ferrite_runtime::input::keycode::KeyCode::F11,
        NamedKey::F12 => ferrite_runtime::input::keycode::KeyCode::F12,
        NamedKey::F13 => ferrite_runtime::input::keycode::KeyCode::F13,
        NamedKey::F14 => ferrite_runtime::input::keycode::KeyCode::F14,
        NamedKey::F15 => ferrite_runtime::input::keycode::KeyCode::F15,
        NamedKey::F16 => ferrite_runtime::input::keycode::KeyCode::F16,
        NamedKey::F17 => ferrite_runtime::input::keycode::KeyCode::F17,
        NamedKey::F18 => ferrite_runtime::input::keycode::KeyCode::F18,
        NamedKey::F19 => ferrite_runtime::input::keycode::KeyCode::F19,
        NamedKey::F20 => ferrite_runtime::input::keycode::KeyCode::F20,
        _ => return None,
    };
    Some(key)
}
