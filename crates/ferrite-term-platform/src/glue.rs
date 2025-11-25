pub fn convert_keycode(
    keycode: crossterm::event::KeyCode,
) -> ferrite_runtime::input::keycode::KeyCode {
    match keycode {
        crossterm::event::KeyCode::Backspace => ferrite_runtime::input::keycode::KeyCode::Backspace,
        crossterm::event::KeyCode::Enter => ferrite_runtime::input::keycode::KeyCode::Enter,
        crossterm::event::KeyCode::Left => ferrite_runtime::input::keycode::KeyCode::Left,
        crossterm::event::KeyCode::Right => ferrite_runtime::input::keycode::KeyCode::Right,
        crossterm::event::KeyCode::Up => ferrite_runtime::input::keycode::KeyCode::Up,
        crossterm::event::KeyCode::Down => ferrite_runtime::input::keycode::KeyCode::Down,
        crossterm::event::KeyCode::Home => ferrite_runtime::input::keycode::KeyCode::Home,
        crossterm::event::KeyCode::End => ferrite_runtime::input::keycode::KeyCode::End,
        crossterm::event::KeyCode::PageUp => ferrite_runtime::input::keycode::KeyCode::PageUp,
        crossterm::event::KeyCode::PageDown => ferrite_runtime::input::keycode::KeyCode::PageDown,
        crossterm::event::KeyCode::Tab => ferrite_runtime::input::keycode::KeyCode::Tab,
        crossterm::event::KeyCode::BackTab => ferrite_runtime::input::keycode::KeyCode::BackTab,
        crossterm::event::KeyCode::Delete => ferrite_runtime::input::keycode::KeyCode::Delete,
        crossterm::event::KeyCode::Insert => ferrite_runtime::input::keycode::KeyCode::Insert,
        crossterm::event::KeyCode::F(1) => ferrite_runtime::input::keycode::KeyCode::F1,
        crossterm::event::KeyCode::F(2) => ferrite_runtime::input::keycode::KeyCode::F2,
        crossterm::event::KeyCode::F(3) => ferrite_runtime::input::keycode::KeyCode::F3,
        crossterm::event::KeyCode::F(4) => ferrite_runtime::input::keycode::KeyCode::F4,
        crossterm::event::KeyCode::F(5) => ferrite_runtime::input::keycode::KeyCode::F5,
        crossterm::event::KeyCode::F(6) => ferrite_runtime::input::keycode::KeyCode::F6,
        crossterm::event::KeyCode::F(7) => ferrite_runtime::input::keycode::KeyCode::F7,
        crossterm::event::KeyCode::F(8) => ferrite_runtime::input::keycode::KeyCode::F8,
        crossterm::event::KeyCode::F(9) => ferrite_runtime::input::keycode::KeyCode::F9,
        crossterm::event::KeyCode::F(10) => ferrite_runtime::input::keycode::KeyCode::F10,
        crossterm::event::KeyCode::F(11) => ferrite_runtime::input::keycode::KeyCode::F11,
        crossterm::event::KeyCode::F(12) => ferrite_runtime::input::keycode::KeyCode::F12,
        crossterm::event::KeyCode::F(13) => ferrite_runtime::input::keycode::KeyCode::F13,
        crossterm::event::KeyCode::F(14) => ferrite_runtime::input::keycode::KeyCode::F14,
        crossterm::event::KeyCode::F(15) => ferrite_runtime::input::keycode::KeyCode::F15,
        crossterm::event::KeyCode::F(16) => ferrite_runtime::input::keycode::KeyCode::F16,
        crossterm::event::KeyCode::F(17) => ferrite_runtime::input::keycode::KeyCode::F17,
        crossterm::event::KeyCode::F(18) => ferrite_runtime::input::keycode::KeyCode::F18,
        crossterm::event::KeyCode::F(19) => ferrite_runtime::input::keycode::KeyCode::F19,
        crossterm::event::KeyCode::F(20) => ferrite_runtime::input::keycode::KeyCode::F20,
        crossterm::event::KeyCode::F(_) => panic!("Function key with higher number then 20"),
        crossterm::event::KeyCode::Char(ch) => ferrite_runtime::input::keycode::KeyCode::Char(ch),
        crossterm::event::KeyCode::Null => ferrite_runtime::input::keycode::KeyCode::Null,
        crossterm::event::KeyCode::Esc => ferrite_runtime::input::keycode::KeyCode::Esc,
        crossterm::event::KeyCode::CapsLock => ferrite_runtime::input::keycode::KeyCode::CapsLock,
        crossterm::event::KeyCode::ScrollLock => {
            ferrite_runtime::input::keycode::KeyCode::ScrollLock
        }
        crossterm::event::KeyCode::NumLock => ferrite_runtime::input::keycode::KeyCode::NumLock,
        crossterm::event::KeyCode::PrintScreen => {
            ferrite_runtime::input::keycode::KeyCode::PrintScreen
        }
        crossterm::event::KeyCode::Pause => ferrite_runtime::input::keycode::KeyCode::Pause,
        crossterm::event::KeyCode::Menu => ferrite_runtime::input::keycode::KeyCode::Menu,
        crossterm::event::KeyCode::KeypadBegin => {
            ferrite_runtime::input::keycode::KeyCode::KeypadBegin
        }
        crossterm::event::KeyCode::Media(media) => convert_media(media),
        crossterm::event::KeyCode::Modifier(modifier) => convert_modifier_keycode(modifier),
    }
}

pub fn convert_media(
    media: crossterm::event::MediaKeyCode,
) -> ferrite_runtime::input::keycode::KeyCode {
    match media {
        crossterm::event::MediaKeyCode::Play => ferrite_runtime::input::keycode::KeyCode::Play,
        crossterm::event::MediaKeyCode::Pause => ferrite_runtime::input::keycode::KeyCode::Pause,
        crossterm::event::MediaKeyCode::PlayPause => {
            ferrite_runtime::input::keycode::KeyCode::PlayPause
        }
        crossterm::event::MediaKeyCode::Reverse => {
            ferrite_runtime::input::keycode::KeyCode::Reverse
        }
        crossterm::event::MediaKeyCode::Stop => ferrite_runtime::input::keycode::KeyCode::Stop,
        crossterm::event::MediaKeyCode::FastForward => {
            ferrite_runtime::input::keycode::KeyCode::FastForward
        }
        crossterm::event::MediaKeyCode::Rewind => ferrite_runtime::input::keycode::KeyCode::Rewind,
        crossterm::event::MediaKeyCode::TrackNext => {
            ferrite_runtime::input::keycode::KeyCode::TrackNext
        }
        crossterm::event::MediaKeyCode::TrackPrevious => {
            ferrite_runtime::input::keycode::KeyCode::TrackPrevious
        }
        crossterm::event::MediaKeyCode::Record => ferrite_runtime::input::keycode::KeyCode::Record,
        crossterm::event::MediaKeyCode::LowerVolume => {
            ferrite_runtime::input::keycode::KeyCode::LowerVolume
        }
        crossterm::event::MediaKeyCode::RaiseVolume => {
            ferrite_runtime::input::keycode::KeyCode::RaiseVolume
        }
        crossterm::event::MediaKeyCode::MuteVolume => {
            ferrite_runtime::input::keycode::KeyCode::MuteVolume
        }
    }
}

pub fn convert_modifier_keycode(
    modifier: crossterm::event::ModifierKeyCode,
) -> ferrite_runtime::input::keycode::KeyCode {
    match modifier {
        crossterm::event::ModifierKeyCode::LeftShift => {
            ferrite_runtime::input::keycode::KeyCode::LeftShift
        }
        crossterm::event::ModifierKeyCode::LeftControl => {
            ferrite_runtime::input::keycode::KeyCode::LeftControl
        }
        crossterm::event::ModifierKeyCode::LeftAlt => {
            ferrite_runtime::input::keycode::KeyCode::LeftAlt
        }
        crossterm::event::ModifierKeyCode::LeftSuper => {
            ferrite_runtime::input::keycode::KeyCode::LeftSuper
        }
        crossterm::event::ModifierKeyCode::LeftHyper => {
            ferrite_runtime::input::keycode::KeyCode::LeftHyper
        }
        crossterm::event::ModifierKeyCode::LeftMeta => {
            ferrite_runtime::input::keycode::KeyCode::LeftMeta
        }
        crossterm::event::ModifierKeyCode::RightShift => {
            ferrite_runtime::input::keycode::KeyCode::RightShift
        }
        crossterm::event::ModifierKeyCode::RightControl => {
            ferrite_runtime::input::keycode::KeyCode::RightControl
        }
        crossterm::event::ModifierKeyCode::RightAlt => {
            ferrite_runtime::input::keycode::KeyCode::RightAlt
        }
        crossterm::event::ModifierKeyCode::RightSuper => {
            ferrite_runtime::input::keycode::KeyCode::RightSuper
        }
        crossterm::event::ModifierKeyCode::RightHyper => {
            ferrite_runtime::input::keycode::KeyCode::RightHyper
        }
        crossterm::event::ModifierKeyCode::RightMeta => {
            ferrite_runtime::input::keycode::KeyCode::RightMeta
        }
        crossterm::event::ModifierKeyCode::IsoLevel3Shift => {
            ferrite_runtime::input::keycode::KeyCode::IsoLevel3Shift
        }
        crossterm::event::ModifierKeyCode::IsoLevel5Shift => {
            ferrite_runtime::input::keycode::KeyCode::IsoLevel5Shift
        }
    }
}

pub fn convert_modifier(
    modifier: crossterm::event::KeyModifiers,
) -> ferrite_runtime::input::keycode::KeyModifiers {
    let mut output = ferrite_runtime::input::keycode::KeyModifiers::empty();
    if modifier.contains(crossterm::event::KeyModifiers::SHIFT) {
        output |= ferrite_runtime::input::keycode::KeyModifiers::SHIFT;
    }

    if modifier.contains(crossterm::event::KeyModifiers::CONTROL) {
        output |= ferrite_runtime::input::keycode::KeyModifiers::CONTROL;
    }

    if modifier.contains(crossterm::event::KeyModifiers::ALT) {
        output |= ferrite_runtime::input::keycode::KeyModifiers::ALT;
    }

    if modifier.contains(crossterm::event::KeyModifiers::SUPER) {
        output |= ferrite_runtime::input::keycode::KeyModifiers::SUPER;
    }

    if modifier.contains(crossterm::event::KeyModifiers::HYPER) {
        output |= ferrite_runtime::input::keycode::KeyModifiers::HYPER;
    }

    if modifier.contains(crossterm::event::KeyModifiers::META) {
        output |= ferrite_runtime::input::keycode::KeyModifiers::META;
    }

    output
}
