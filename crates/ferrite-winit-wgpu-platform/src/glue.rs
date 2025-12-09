use ferrite_runtime::{input::keycode::KeyModifiers, painter::CursorIcon};
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

pub fn convert_cursor_icon(cursor_icon: CursorIcon) -> winit::window::CursorIcon {
    match cursor_icon {
        CursorIcon::Default => winit::window::CursorIcon::Default,
        CursorIcon::ContextMenu => winit::window::CursorIcon::ContextMenu,
        CursorIcon::Help => winit::window::CursorIcon::Help,
        CursorIcon::Pointer => winit::window::CursorIcon::Pointer,
        CursorIcon::Progress => winit::window::CursorIcon::Progress,
        CursorIcon::Wait => winit::window::CursorIcon::Wait,
        CursorIcon::Cell => winit::window::CursorIcon::Cell,
        CursorIcon::Crosshair => winit::window::CursorIcon::Crosshair,
        CursorIcon::Text => winit::window::CursorIcon::Text,
        CursorIcon::VerticalText => winit::window::CursorIcon::VerticalText,
        CursorIcon::Alias => winit::window::CursorIcon::Alias,
        CursorIcon::Copy => winit::window::CursorIcon::Copy,
        CursorIcon::Move => winit::window::CursorIcon::Move,
        CursorIcon::NoDrop => winit::window::CursorIcon::NoDrop,
        CursorIcon::NotAllowed => winit::window::CursorIcon::NotAllowed,
        CursorIcon::Grab => winit::window::CursorIcon::Grab,
        CursorIcon::Grabbing => winit::window::CursorIcon::Grabbing,
        CursorIcon::EResize => winit::window::CursorIcon::EResize,
        CursorIcon::NResize => winit::window::CursorIcon::NResize,
        CursorIcon::NeResize => winit::window::CursorIcon::NeResize,
        CursorIcon::NwResize => winit::window::CursorIcon::NwResize,
        CursorIcon::SResize => winit::window::CursorIcon::SResize,
        CursorIcon::SeResize => winit::window::CursorIcon::SeResize,
        CursorIcon::SwResize => winit::window::CursorIcon::SwResize,
        CursorIcon::WResize => winit::window::CursorIcon::WResize,
        CursorIcon::EwResize => winit::window::CursorIcon::EwResize,
        CursorIcon::NsResize => winit::window::CursorIcon::NsResize,
        CursorIcon::NeswResize => winit::window::CursorIcon::NeswResize,
        CursorIcon::NwseResize => winit::window::CursorIcon::NwseResize,
        CursorIcon::ColResize => winit::window::CursorIcon::ColResize,
        CursorIcon::RowResize => winit::window::CursorIcon::RowResize,
        CursorIcon::AllScroll => winit::window::CursorIcon::AllScroll,
        CursorIcon::ZoomIn => winit::window::CursorIcon::ZoomIn,
        CursorIcon::ZoomOut => winit::window::CursorIcon::ZoomOut,
        _ => winit::window::CursorIcon::Default,
    }
}
