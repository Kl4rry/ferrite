
pub fn convert_style(style: &ferrite_core::theme::style::Style) -> tui::style::Style {
    tui::style::Style {
        fg: style.fg.as_ref().map(|color| {
            tui::style::Color::Rgb(
                (color.r * 255.0) as u8,
                (color.g * 255.0) as u8,
                (color.b * 255.0) as u8,
            )
        }),
        bg: style.bg.as_ref().map(|color| {
            tui::style::Color::Rgb(
                (color.r * 255.0) as u8,
                (color.g * 255.0) as u8,
                (color.b * 255.0) as u8,
            )
        }),
        ..Default::default()
    }
}

pub fn tui_to_ferrite_rect(rect: tui::layout::Rect) -> ferrite_core::panes::Rect {
    ferrite_core::panes::Rect {
        x: rect.x.into(),
        y: rect.y.into(),
        width: rect.width.into(),
        height: rect.height.into(),
    }
}

pub fn ferrite_to_tui_rect(rect: ferrite_core::panes::Rect) -> tui::layout::Rect {
    tui::layout::Rect {
        x: rect.x.try_into().unwrap(),
        y: rect.y.try_into().unwrap(),
        width: rect.width.try_into().unwrap(),
        height: rect.height.try_into().unwrap(),
    }
}

pub fn convert_keycode(keycode: crossterm::event::KeyCode) -> ferrite_core::keymap::keycode::KeyCode {
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
        crossterm::event::KeyCode::F(f) => ferrite_core::keymap::keycode::KeyCode::F(f),
        crossterm::event::KeyCode::Char(ch) => ferrite_core::keymap::keycode::KeyCode::Char(ch),
        crossterm::event::KeyCode::Null => ferrite_core::keymap::keycode::KeyCode::Null,
        crossterm::event::KeyCode::Esc => ferrite_core::keymap::keycode::KeyCode::Esc,
        crossterm::event::KeyCode::CapsLock => ferrite_core::keymap::keycode::KeyCode::CapsLock,
        crossterm::event::KeyCode::ScrollLock => ferrite_core::keymap::keycode::KeyCode::ScrollLock,
        crossterm::event::KeyCode::NumLock => ferrite_core::keymap::keycode::KeyCode::NumLock,
        crossterm::event::KeyCode::PrintScreen => ferrite_core::keymap::keycode::KeyCode::PrintScreen,
        crossterm::event::KeyCode::Pause => ferrite_core::keymap::keycode::KeyCode::Pause,
        crossterm::event::KeyCode::Menu => ferrite_core::keymap::keycode::KeyCode::Menu,
        crossterm::event::KeyCode::KeypadBegin => ferrite_core::keymap::keycode::KeyCode::KeypadBegin,
        crossterm::event::KeyCode::Media(media) => ferrite_core::keymap::keycode::KeyCode::Media(convert_media(media)),
        crossterm::event::KeyCode::Modifier(modifier) => ferrite_core::keymap::keycode::KeyCode::Modifier(convert_modifier_keycode(modifier)),
    }
}

pub fn convert_media(media: crossterm::event::MediaKeyCode) -> ferrite_core::keymap::keycode::MediaKeyCode {
    match media {
        crossterm::event::MediaKeyCode::Play => ferrite_core::keymap::keycode::MediaKeyCode::Play,
        crossterm::event::MediaKeyCode::Pause => ferrite_core::keymap::keycode::MediaKeyCode::Pause,
        crossterm::event::MediaKeyCode::PlayPause => ferrite_core::keymap::keycode::MediaKeyCode::PlayPause,
        crossterm::event::MediaKeyCode::Reverse => ferrite_core::keymap::keycode::MediaKeyCode::Reverse,
        crossterm::event::MediaKeyCode::Stop => ferrite_core::keymap::keycode::MediaKeyCode::Stop,
        crossterm::event::MediaKeyCode::FastForward => ferrite_core::keymap::keycode::MediaKeyCode::FastForward,
        crossterm::event::MediaKeyCode::Rewind => ferrite_core::keymap::keycode::MediaKeyCode::Rewind,
        crossterm::event::MediaKeyCode::TrackNext => ferrite_core::keymap::keycode::MediaKeyCode::TrackNext,
        crossterm::event::MediaKeyCode::TrackPrevious => ferrite_core::keymap::keycode::MediaKeyCode::TrackPrevious,
        crossterm::event::MediaKeyCode::Record => ferrite_core::keymap::keycode::MediaKeyCode::Record,
        crossterm::event::MediaKeyCode::LowerVolume => ferrite_core::keymap::keycode::MediaKeyCode::LowerVolume,
        crossterm::event::MediaKeyCode::RaiseVolume => ferrite_core::keymap::keycode::MediaKeyCode::RaiseVolume,
        crossterm::event::MediaKeyCode::MuteVolume => ferrite_core::keymap::keycode::MediaKeyCode::MuteVolume,
    }
}

pub fn convert_modifier_keycode(modifier: crossterm::event::ModifierKeyCode) -> ferrite_core::keymap::keycode::ModifierKeyCode {
    match modifier {
        crossterm::event::ModifierKeyCode::LeftShift => ferrite_core::keymap::keycode::ModifierKeyCode::LeftShift,
        crossterm::event::ModifierKeyCode::LeftControl => ferrite_core::keymap::keycode::ModifierKeyCode::LeftControl,
        crossterm::event::ModifierKeyCode::LeftAlt => ferrite_core::keymap::keycode::ModifierKeyCode::LeftAlt,
        crossterm::event::ModifierKeyCode::LeftSuper => ferrite_core::keymap::keycode::ModifierKeyCode::LeftSuper,
        crossterm::event::ModifierKeyCode::LeftHyper => ferrite_core::keymap::keycode::ModifierKeyCode::LeftHyper,
        crossterm::event::ModifierKeyCode::LeftMeta => ferrite_core::keymap::keycode::ModifierKeyCode::LeftMeta,
        crossterm::event::ModifierKeyCode::RightShift => ferrite_core::keymap::keycode::ModifierKeyCode::RightShift,
        crossterm::event::ModifierKeyCode::RightControl => ferrite_core::keymap::keycode::ModifierKeyCode::RightControl,
        crossterm::event::ModifierKeyCode::RightAlt => ferrite_core::keymap::keycode::ModifierKeyCode::RightAlt,
        crossterm::event::ModifierKeyCode::RightSuper => ferrite_core::keymap::keycode::ModifierKeyCode::RightSuper,
        crossterm::event::ModifierKeyCode::RightHyper => ferrite_core::keymap::keycode::ModifierKeyCode::RightHyper,
        crossterm::event::ModifierKeyCode::RightMeta => ferrite_core::keymap::keycode::ModifierKeyCode::RightMeta,
        crossterm::event::ModifierKeyCode::IsoLevel3Shift => ferrite_core::keymap::keycode::ModifierKeyCode::IsoLevel3Shift,
        crossterm::event::ModifierKeyCode::IsoLevel5Shift => ferrite_core::keymap::keycode::ModifierKeyCode::IsoLevel5Shift,
    }
}

pub fn convert_modifier(modifier: crossterm::event::KeyModifiers) -> ferrite_core::keymap::keycode::KeyModifiers {
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
