use core::fmt;

bitflags::bitflags! {
    #[derive(Debug, PartialOrd, PartialEq, Eq, Clone, Copy, Hash)]
    pub struct KeyModifiers: u8 {
        const SHIFT = 0b0000_0001;
        const CONTROL = 0b0000_0010;
        const ALT = 0b0000_0100;
        const SUPER = 0b0000_1000;
        const HYPER = 0b0001_0000;
        const META = 0b0010_0000;
        const NONE = 0b0000_0000;
    }
}

impl fmt::Display for KeyModifiers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if *self == Self::NONE {
            return Ok(());
        }

        if self.contains(Self::SHIFT) {
            " + Shift".fmt(f)?;
        }

        if self.contains(Self::CONTROL) {
            " + Ctrl".fmt(f)?;
        }

        if self.contains(Self::ALT) {
            " + Alt".fmt(f)?;
        }

        if self.contains(Self::SUPER) {
            " + Super".fmt(f)?;
        }

        if self.contains(Self::HYPER) {
            " + Hyper".fmt(f)?;
        }

        if self.contains(Self::META) {
            " + Meta".fmt(f)?;
        }

        Ok(())
    }
}

#[derive(
    Debug, PartialOrd, PartialEq, Eq, Clone, Copy, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum MediaKeyCode {
    Play,
    Pause,
    PlayPause,
    Reverse,
    Stop,
    FastForward,
    Rewind,
    TrackNext,
    TrackPrevious,
    Record,
    LowerVolume,
    RaiseVolume,
    MuteVolume,
}

#[derive(
    Debug, PartialOrd, PartialEq, Eq, Clone, Copy, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum ModifierKeyCode {
    LeftShift,
    LeftControl,
    LeftAlt,
    LeftSuper,
    LeftHyper,
    LeftMeta,
    RightShift,
    RightControl,
    RightAlt,
    RightSuper,
    RightHyper,
    RightMeta,
    IsoLevel3Shift,
    IsoLevel5Shift,
}

/// Represents a key.
#[derive(
    Debug, PartialOrd, PartialEq, Eq, Clone, Copy, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum KeyCode {
    Backspace,
    Enter,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    Tab,
    BackTab,
    Delete,
    Insert,
    F(u8),
    Char(char),
    Null,
    Esc,
    CapsLock,
    ScrollLock,
    NumLock,
    PrintScreen,
    Pause,
    Menu,
    KeypadBegin,
    Media(MediaKeyCode),
    Modifier(ModifierKeyCode),
}

impl fmt::Display for KeyCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KeyCode::F(n) => write!(f, "F{n}"),
            KeyCode::Char(ch) => ch.fmt(f),
            keycode => write!(f, "{keycode:?}"),
        }
    }
}
