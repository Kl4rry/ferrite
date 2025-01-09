use core::fmt;

use serde::{
    de::{self, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};

bitflags::bitflags! {
    // TODO make custom impl of Serialize and Deserialize to match to and from str
    #[derive(Debug, PartialOrd, PartialEq, Eq, Clone, Copy, Hash)]
    pub struct KeyModifiers: u8 {
        const SHIFT = 0b0000_0001;
        const CONTROL = 0b0000_0010;
        const ALT = 0b0000_0100;
        const SUPER = 0b0000_1000;
        const HYPER = 0b0001_0000;
        const META = 0b0010_0000;
    }
}

impl KeyModifiers {
    pub fn try_from_str(s: &str) -> Option<Self> {
        Some(match s {
            "<Shift>" => Self::SHIFT,
            "<Control>" => Self::CONTROL,
            "<Alt>" => Self::ALT,
            "<Super>" => Self::SUPER,
            "<Hypr>" => Self::HYPER,
            "<Meta>" => Self::META,
            _ => return None,
        })
    }

    pub fn try_to_string(&self) -> Option<String> {
        if *self == Self::empty() {
            return None;
        }
        let mut output = String::new();
        if self.contains(Self::SHIFT) {
            if !output.is_empty() {
                output.push('-');
            }
            output.push_str("<Shift>");
        }
        if self.contains(Self::CONTROL) {
            if !output.is_empty() {
                output.push('-');
            }
            output.push_str("<Control>");
        }
        if self.contains(Self::ALT) {
            if !output.is_empty() {
                output.push('-');
            }
            output.push_str("<Alt>");
        }
        if self.contains(Self::SUPER) {
            if !output.is_empty() {
                output.push('-');
            }
            output.push_str("<Super>");
        }
        if self.contains(Self::HYPER) {
            if !output.is_empty() {
                output.push('-');
            }
            output.push_str("<Hyper>");
        }
        if self.contains(Self::META) {
            if !output.is_empty() {
                output.push('-');
            }
            output.push_str("<Meta>");
        }
        Some(output)
    }
}

impl fmt::Display for KeyModifiers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if *self == Self::empty() {
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

/// Represents a key.
#[derive(Debug, PartialOrd, PartialEq, Eq, Clone, Copy, Hash)]
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
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,
    Char(char),
    Null,
    Esc,
    CapsLock,
    ScrollLock,
    NumLock,
    PrintScreen,
    Menu,
    KeypadBegin,

    // media
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

    // modifiers
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

impl KeyCode {
    pub fn try_from_str(s: &str) -> anyhow::Result<Self> {
        Ok(match s {
            "Backspace" => KeyCode::Backspace,
            "Enter" => KeyCode::Enter,
            "Left" => KeyCode::Left,
            "Right" => KeyCode::Right,
            "Up" => KeyCode::Up,
            "Down" => KeyCode::Down,
            "Home" => KeyCode::Home,
            "End" => KeyCode::End,
            "PageUp" => KeyCode::PageUp,
            "PageDown" => KeyCode::PageDown,
            "Tab" => KeyCode::Tab,
            "BackTab" => KeyCode::BackTab,
            "Delete" => KeyCode::Delete,
            "Insert" => KeyCode::Insert,
            "F1" => KeyCode::F1,
            "F2" => KeyCode::F2,
            "F3" => KeyCode::F3,
            "F4" => KeyCode::F4,
            "F5" => KeyCode::F5,
            "F6" => KeyCode::F6,
            "F7" => KeyCode::F7,
            "F8" => KeyCode::F8,
            "F9" => KeyCode::F9,
            "F10" => KeyCode::F10,
            "F11" => KeyCode::F11,
            "F12" => KeyCode::F12,
            "F13" => KeyCode::F13,
            "F14" => KeyCode::F14,
            "F15" => KeyCode::F15,
            "F16" => KeyCode::F16,
            "F17" => KeyCode::F17,
            "F18" => KeyCode::F18,
            "F19" => KeyCode::F19,
            "F20" => KeyCode::F20,
            "Null" => KeyCode::Null,
            "Esc" => KeyCode::Esc,
            "CapsLock" => KeyCode::CapsLock,
            "ScrollLock" => KeyCode::ScrollLock,
            "NumLock" => KeyCode::NumLock,
            "PrintScreen" => KeyCode::PrintScreen,
            "Menu" => KeyCode::Menu,
            "KeypadBegin" => KeyCode::KeypadBegin,

            // media
            "Play" => KeyCode::Play,
            "Pause" => KeyCode::Pause,
            "PlayPause" => KeyCode::PlayPause,
            "Reverse" => KeyCode::Reverse,
            "Stop" => KeyCode::Stop,
            "FastForward" => KeyCode::FastForward,
            "Rewind" => KeyCode::Rewind,
            "TrackNext" => KeyCode::TrackNext,
            "TrackPrevious" => KeyCode::TrackPrevious,
            "Record" => KeyCode::Record,
            "LowerVolume" => KeyCode::LowerVolume,
            "RaiseVolume" => KeyCode::RaiseVolume,
            "MuteVolume" => KeyCode::MuteVolume,

            // modifiers
            "LeftShift" => KeyCode::LeftShift,
            "LeftControl" => KeyCode::LeftControl,
            "LeftAlt" => KeyCode::LeftAlt,
            "LeftSuper" => KeyCode::LeftSuper,
            "LeftHyper" => KeyCode::LeftHyper,
            "LeftMeta" => KeyCode::LeftMeta,
            "RightShift" => KeyCode::RightShift,
            "RightControl" => KeyCode::RightControl,
            "RightAlt" => KeyCode::RightAlt,
            "RightSuper" => KeyCode::RightSuper,
            "RightHyper" => KeyCode::RightHyper,
            "RightMeta" => KeyCode::RightMeta,
            "IsoLevel3Shift" => KeyCode::IsoLevel3Shift,
            "IsoLevel5Shift" => KeyCode::IsoLevel5Shift,

            // special
            "Space" => KeyCode::Char(' '),
            s => {
                let Some(ch) = s.chars().next() else {
                    anyhow::bail!("keybinds must be atleast one char long");
                };
                if s.chars().count() != 1 {
                    anyhow::bail!("unrecognized keybind: `{s}`");
                }

                KeyCode::Char(ch)
            }
        })
    }
}

#[allow(clippy::to_string_trait_impl)]
impl ToString for KeyCode {
    fn to_string(&self) -> String {
        match self {
            KeyCode::Backspace => "Backspace",
            KeyCode::Enter => "Enter",
            KeyCode::Left => "Left",
            KeyCode::Right => "Right",
            KeyCode::Up => "Up",
            KeyCode::Down => "Down",
            KeyCode::Home => "Home",
            KeyCode::End => "End",
            KeyCode::PageUp => "PageUp",
            KeyCode::PageDown => "PageDown",
            KeyCode::Tab => "Tab",
            KeyCode::BackTab => "BackTab",
            KeyCode::Delete => "Delete",
            KeyCode::Insert => "Insert",
            KeyCode::F1 => "F1",
            KeyCode::F2 => "F2",
            KeyCode::F3 => "F3",
            KeyCode::F4 => "F4",
            KeyCode::F5 => "F5",
            KeyCode::F6 => "F6",
            KeyCode::F7 => "F7",
            KeyCode::F8 => "F8",
            KeyCode::F9 => "F9",
            KeyCode::F10 => "F10",
            KeyCode::F11 => "F11",
            KeyCode::F12 => "F12",
            KeyCode::F13 => "F13",
            KeyCode::F14 => "F14",
            KeyCode::F15 => "F15",
            KeyCode::F16 => "F16",
            KeyCode::F17 => "F17",
            KeyCode::F18 => "F18",
            KeyCode::F19 => "F19",
            KeyCode::F20 => "F20",
            KeyCode::Null => "Null",
            KeyCode::Esc => "Esc",
            KeyCode::CapsLock => "CapsLock",
            KeyCode::ScrollLock => "ScrollLock",
            KeyCode::NumLock => "NumLock",
            KeyCode::PrintScreen => "PrintScreen",
            KeyCode::Menu => "Menu",
            KeyCode::KeypadBegin => "KeypadBegin",

            // media
            KeyCode::Play => "Play",
            KeyCode::Pause => "Pause",
            KeyCode::PlayPause => "PlayPause",
            KeyCode::Reverse => "Reverse",
            KeyCode::Stop => "Stop",
            KeyCode::FastForward => "FastForward",
            KeyCode::Rewind => "Rewind",
            KeyCode::TrackNext => "TrackNext",
            KeyCode::TrackPrevious => "TrackPrevious",
            KeyCode::Record => "Record",
            KeyCode::LowerVolume => "LowerVolume",
            KeyCode::RaiseVolume => "RaiseVolume",
            KeyCode::MuteVolume => "MuteVolume",

            // modifiers
            KeyCode::LeftShift => "LeftShift",
            KeyCode::LeftControl => "LeftControl",
            KeyCode::LeftAlt => "LeftAlt",
            KeyCode::LeftSuper => "LeftSuper",
            KeyCode::LeftHyper => "LeftHyper",
            KeyCode::LeftMeta => "LeftMeta",
            KeyCode::RightShift => "RightShift",
            KeyCode::RightControl => "RightControl",
            KeyCode::RightAlt => "RightAlt",
            KeyCode::RightSuper => "RightSuper",
            KeyCode::RightHyper => "RightHyper",
            KeyCode::RightMeta => "RightMeta",
            KeyCode::IsoLevel3Shift => "IsoLevel3Shift",
            KeyCode::IsoLevel5Shift => "IsoLevel5Shift",
            KeyCode::Char(' ') => "Space",
            KeyCode::Char(ch) => return ch.to_string(),
        }
        .to_string()
    }
}

impl<'de> Deserialize<'de> for KeyModifiers {
    fn deserialize<D>(deserializer: D) -> Result<KeyModifiers, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct KeyModifiersVisitor;

        impl Visitor<'_> for KeyModifiersVisitor {
            type Value = KeyModifiers;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("key mapping")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if value.is_empty() {
                    return Ok(KeyModifiers::empty());
                }
                let strs = value.split("-");
                let mut modifiers = KeyModifiers::empty();
                for s in strs {
                    match KeyModifiers::try_from_str(s) {
                        Some(modifier) => modifiers |= modifier,
                        None => {
                            return Err(de::Error::custom(format!("unrecognized modifier {}", s)))
                        }
                    }
                }
                Ok(modifiers)
            }
        }

        deserializer.deserialize_string(KeyModifiersVisitor)
    }
}

impl Serialize for KeyModifiers {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.try_to_string().unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_mult_modifiers() {
        let modifiers = KeyModifiers::ALT | KeyModifiers::SHIFT | KeyModifiers::CONTROL;
        let s = serde_json::to_string(&modifiers).unwrap();
        let parsed = serde_json::from_str(&s);
        assert!(parsed.is_ok());
        assert_eq!(modifiers, parsed.unwrap());
    }

    #[test]
    fn serde_single_modifiers() {
        let modifiers = KeyModifiers::ALT;
        let s = serde_json::to_string(&modifiers).unwrap();
        let parsed = serde_json::from_str(&s);
        assert!(parsed.is_ok());
        assert_eq!(modifiers, parsed.unwrap());
    }

    #[test]
    fn serde_no_modifiers() {
        let modifiers = KeyModifiers::empty();
        let s = serde_json::to_string(&modifiers).unwrap();
        let parsed = serde_json::from_str(&s);
        assert!(parsed.is_ok());
        assert_eq!(modifiers, parsed.unwrap());
    }
}
