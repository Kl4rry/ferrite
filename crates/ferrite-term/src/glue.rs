pub fn convert_keycode(
    keycode: crossterm::event::KeyCode,
) -> ferrite_core::keymap::keycode::KeyCode {
    match keycode {
        crossterm::event::KeyCode::Backspace => ferrite_core::keymap::keycode::KeyCode::Backspace,
        crossterm::event::KeyCode::Enter => ferrite_core::keymap::keycode::KeyCode::Enter,
        crossterm::event::KeyCode::Left => ferrite_core::keymap::keycode::KeyCode::Left,
        crossterm::event::KeyCode::Right => ferrite_core::keymap::keycode::KeyCode::Right,
        crossterm::event::KeyCode::Up => ferrite_core::keymap::keycode::KeyCode::Up,
        crossterm::event::KeyCode::Down => ferrite_core::keymap::keycode::KeyCode::Down,
        crossterm::event::KeyCode::Home => ferrite_core::keymap::keycode::KeyCode::Home,
        crossterm::event::KeyCode::End => ferrite_core::keymap::keycode::KeyCode::End,
        crossterm::event::KeyCode::PageUp => ferrite_core::keymap::keycode::KeyCode::PageUp,
        crossterm::event::KeyCode::PageDown => ferrite_core::keymap::keycode::KeyCode::PageDown,
        crossterm::event::KeyCode::Tab => ferrite_core::keymap::keycode::KeyCode::Tab,
        crossterm::event::KeyCode::BackTab => ferrite_core::keymap::keycode::KeyCode::BackTab,
        crossterm::event::KeyCode::Delete => ferrite_core::keymap::keycode::KeyCode::Delete,
        crossterm::event::KeyCode::Insert => ferrite_core::keymap::keycode::KeyCode::Insert,
        crossterm::event::KeyCode::F(1) => ferrite_core::keymap::keycode::KeyCode::F1,
        crossterm::event::KeyCode::F(2) => ferrite_core::keymap::keycode::KeyCode::F2,
        crossterm::event::KeyCode::F(3) => ferrite_core::keymap::keycode::KeyCode::F3,
        crossterm::event::KeyCode::F(4) => ferrite_core::keymap::keycode::KeyCode::F4,
        crossterm::event::KeyCode::F(5) => ferrite_core::keymap::keycode::KeyCode::F5,
        crossterm::event::KeyCode::F(6) => ferrite_core::keymap::keycode::KeyCode::F6,
        crossterm::event::KeyCode::F(7) => ferrite_core::keymap::keycode::KeyCode::F7,
        crossterm::event::KeyCode::F(8) => ferrite_core::keymap::keycode::KeyCode::F8,
        crossterm::event::KeyCode::F(9) => ferrite_core::keymap::keycode::KeyCode::F9,
        crossterm::event::KeyCode::F(10) => ferrite_core::keymap::keycode::KeyCode::F10,
        crossterm::event::KeyCode::F(11) => ferrite_core::keymap::keycode::KeyCode::F11,
        crossterm::event::KeyCode::F(12) => ferrite_core::keymap::keycode::KeyCode::F12,
        crossterm::event::KeyCode::F(13) => ferrite_core::keymap::keycode::KeyCode::F13,
        crossterm::event::KeyCode::F(14) => ferrite_core::keymap::keycode::KeyCode::F14,
        crossterm::event::KeyCode::F(15) => ferrite_core::keymap::keycode::KeyCode::F15,
        crossterm::event::KeyCode::F(16) => ferrite_core::keymap::keycode::KeyCode::F16,
        crossterm::event::KeyCode::F(17) => ferrite_core::keymap::keycode::KeyCode::F17,
        crossterm::event::KeyCode::F(18) => ferrite_core::keymap::keycode::KeyCode::F18,
        crossterm::event::KeyCode::F(19) => ferrite_core::keymap::keycode::KeyCode::F19,
        crossterm::event::KeyCode::F(20) => ferrite_core::keymap::keycode::KeyCode::F20,
        crossterm::event::KeyCode::F(_) => panic!("Function key with higher number then 20"),
        crossterm::event::KeyCode::Char(ch) => ferrite_core::keymap::keycode::KeyCode::Char(ch),
        crossterm::event::KeyCode::Null => ferrite_core::keymap::keycode::KeyCode::Null,
        crossterm::event::KeyCode::Esc => ferrite_core::keymap::keycode::KeyCode::Esc,
        crossterm::event::KeyCode::CapsLock => ferrite_core::keymap::keycode::KeyCode::CapsLock,
        crossterm::event::KeyCode::ScrollLock => ferrite_core::keymap::keycode::KeyCode::ScrollLock,
        crossterm::event::KeyCode::NumLock => ferrite_core::keymap::keycode::KeyCode::NumLock,
        crossterm::event::KeyCode::PrintScreen => {
            ferrite_core::keymap::keycode::KeyCode::PrintScreen
        }
        crossterm::event::KeyCode::Pause => ferrite_core::keymap::keycode::KeyCode::Pause,
        crossterm::event::KeyCode::Menu => ferrite_core::keymap::keycode::KeyCode::Menu,
        crossterm::event::KeyCode::KeypadBegin => {
            ferrite_core::keymap::keycode::KeyCode::KeypadBegin
        }
        crossterm::event::KeyCode::Media(media) => convert_media(media),
        crossterm::event::KeyCode::Modifier(modifier) => convert_modifier_keycode(modifier),
    }
}

pub fn convert_media(
    media: crossterm::event::MediaKeyCode,
) -> ferrite_core::keymap::keycode::KeyCode {
    match media {
        crossterm::event::MediaKeyCode::Play => ferrite_core::keymap::keycode::KeyCode::Play,
        crossterm::event::MediaKeyCode::Pause => ferrite_core::keymap::keycode::KeyCode::Pause,
        crossterm::event::MediaKeyCode::PlayPause => {
            ferrite_core::keymap::keycode::KeyCode::PlayPause
        }
        crossterm::event::MediaKeyCode::Reverse => ferrite_core::keymap::keycode::KeyCode::Reverse,
        crossterm::event::MediaKeyCode::Stop => ferrite_core::keymap::keycode::KeyCode::Stop,
        crossterm::event::MediaKeyCode::FastForward => {
            ferrite_core::keymap::keycode::KeyCode::FastForward
        }
        crossterm::event::MediaKeyCode::Rewind => ferrite_core::keymap::keycode::KeyCode::Rewind,
        crossterm::event::MediaKeyCode::TrackNext => {
            ferrite_core::keymap::keycode::KeyCode::TrackNext
        }
        crossterm::event::MediaKeyCode::TrackPrevious => {
            ferrite_core::keymap::keycode::KeyCode::TrackPrevious
        }
        crossterm::event::MediaKeyCode::Record => ferrite_core::keymap::keycode::KeyCode::Record,
        crossterm::event::MediaKeyCode::LowerVolume => {
            ferrite_core::keymap::keycode::KeyCode::LowerVolume
        }
        crossterm::event::MediaKeyCode::RaiseVolume => {
            ferrite_core::keymap::keycode::KeyCode::RaiseVolume
        }
        crossterm::event::MediaKeyCode::MuteVolume => {
            ferrite_core::keymap::keycode::KeyCode::MuteVolume
        }
    }
}

pub fn convert_modifier_keycode(
    modifier: crossterm::event::ModifierKeyCode,
) -> ferrite_core::keymap::keycode::KeyCode {
    match modifier {
        crossterm::event::ModifierKeyCode::LeftShift => {
            ferrite_core::keymap::keycode::KeyCode::LeftShift
        }
        crossterm::event::ModifierKeyCode::LeftControl => {
            ferrite_core::keymap::keycode::KeyCode::LeftControl
        }
        crossterm::event::ModifierKeyCode::LeftAlt => {
            ferrite_core::keymap::keycode::KeyCode::LeftAlt
        }
        crossterm::event::ModifierKeyCode::LeftSuper => {
            ferrite_core::keymap::keycode::KeyCode::LeftSuper
        }
        crossterm::event::ModifierKeyCode::LeftHyper => {
            ferrite_core::keymap::keycode::KeyCode::LeftHyper
        }
        crossterm::event::ModifierKeyCode::LeftMeta => {
            ferrite_core::keymap::keycode::KeyCode::LeftMeta
        }
        crossterm::event::ModifierKeyCode::RightShift => {
            ferrite_core::keymap::keycode::KeyCode::RightShift
        }
        crossterm::event::ModifierKeyCode::RightControl => {
            ferrite_core::keymap::keycode::KeyCode::RightControl
        }
        crossterm::event::ModifierKeyCode::RightAlt => {
            ferrite_core::keymap::keycode::KeyCode::RightAlt
        }
        crossterm::event::ModifierKeyCode::RightSuper => {
            ferrite_core::keymap::keycode::KeyCode::RightSuper
        }
        crossterm::event::ModifierKeyCode::RightHyper => {
            ferrite_core::keymap::keycode::KeyCode::RightHyper
        }
        crossterm::event::ModifierKeyCode::RightMeta => {
            ferrite_core::keymap::keycode::KeyCode::RightMeta
        }
        crossterm::event::ModifierKeyCode::IsoLevel3Shift => {
            ferrite_core::keymap::keycode::KeyCode::IsoLevel3Shift
        }
        crossterm::event::ModifierKeyCode::IsoLevel5Shift => {
            ferrite_core::keymap::keycode::KeyCode::IsoLevel5Shift
        }
    }
}

pub fn convert_modifier(
    modifier: crossterm::event::KeyModifiers,
) -> ferrite_core::keymap::keycode::KeyModifiers {
    let mut output = ferrite_core::keymap::keycode::KeyModifiers::empty();
    if modifier.contains(crossterm::event::KeyModifiers::SHIFT) {
        output |= ferrite_core::keymap::keycode::KeyModifiers::SHIFT;
    }

    if modifier.contains(crossterm::event::KeyModifiers::CONTROL) {
        output |= ferrite_core::keymap::keycode::KeyModifiers::CONTROL;
    }

    if modifier.contains(crossterm::event::KeyModifiers::ALT) {
        output |= ferrite_core::keymap::keycode::KeyModifiers::ALT;
    }

    if modifier.contains(crossterm::event::KeyModifiers::SUPER) {
        output |= ferrite_core::keymap::keycode::KeyModifiers::SUPER;
    }

    if modifier.contains(crossterm::event::KeyModifiers::HYPER) {
        output |= ferrite_core::keymap::keycode::KeyModifiers::HYPER;
    }

    if modifier.contains(crossterm::event::KeyModifiers::META) {
        output |= ferrite_core::keymap::keycode::KeyModifiers::META;
    }

    output
}
