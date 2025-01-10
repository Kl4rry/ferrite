pub mod keycode;
use std::fmt;

use keycode::{KeyCode, KeyModifiers};
use serde::{
    de::{self, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};

use crate::{
    cmd::{Cmd, LineMoveDir},
    config::keymap::Keymapping,
    layout::panes::Direction,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Key {
    pub keycode: KeyCode,
    pub modifiers: KeyModifiers,
}

impl Key {
    pub const fn new(keycode: KeyCode, modifiers: KeyModifiers) -> Self {
        Self { keycode, modifiers }
    }
}

pub fn get_command_from_input(
    keycode: KeyCode,
    modifiers: KeyModifiers,
    mappings: &[Keymapping],
) -> Option<Cmd> {
    let normalized_keycode = match keycode {
        KeyCode::Char(ch) => KeyCode::Char(ch.to_ascii_lowercase()),
        keycode => keycode,
    };
    for Keymapping {
        key,
        cmd,
        ignore_modifiers,
    } in mappings
    {
        if *ignore_modifiers {
            if key.keycode == normalized_keycode && modifiers.contains(key.modifiers) {
                return Some(cmd.clone());
            }
        } else if *key
            == (Key {
                keycode: normalized_keycode,
                modifiers,
            })
        {
            return Some(cmd.clone());
        }
    }

    if let KeyCode::Char(ch) = keycode {
        if !ch.is_ascii_alphanumeric()
            || modifiers == KeyModifiers::empty()
            || modifiers == KeyModifiers::SHIFT
        {
            return Some(Cmd::Char { ch });
        }
    }

    None
}

pub fn get_default_chords() -> Vec<(Key, Cmd, bool)> {
    vec![
        (
            Key::new(KeyCode::Esc, KeyModifiers::empty()),
            Cmd::Escape,
            false,
        ),
        (
            Key::new(KeyCode::Char('k'), KeyModifiers::CONTROL),
            Cmd::InputMode {
                name: "normal".into(),
            },
            false,
        ),
        (
            Key::new(KeyCode::Char('f'), KeyModifiers::CONTROL),
            Cmd::Format,
            false,
        ),
        (
            Key::new(KeyCode::Char('s'), KeyModifiers::CONTROL),
            Cmd::OpenShellPalette,
            false,
        ),
        (
            Key::new(KeyCode::Char('o'), KeyModifiers::CONTROL),
            Cmd::UrlOpen,
            false,
        ),
        (
            Key::new(KeyCode::Char('r'), KeyModifiers::CONTROL),
            Cmd::Split {
                direction: Direction::Right,
            },
            false,
        ),
        (
            Key::new(KeyCode::Char('l'), KeyModifiers::CONTROL),
            Cmd::Split {
                direction: Direction::Left,
            },
            false,
        ),
        (
            Key::new(KeyCode::Char('u'), KeyModifiers::CONTROL),
            Cmd::Split {
                direction: Direction::Up,
            },
            false,
        ),
        (
            Key::new(KeyCode::Char('d'), KeyModifiers::CONTROL),
            Cmd::Split {
                direction: Direction::Down,
            },
            false,
        ),
        (
            Key::new(KeyCode::Char('j'), KeyModifiers::CONTROL),
            Cmd::RotateFile,
            false,
        ),
        (
            Key::new(KeyCode::Char('w'), KeyModifiers::CONTROL),
            Cmd::ClosePane,
            false,
        ),
        (
            Key::new(KeyCode::Char('g'), KeyModifiers::CONTROL),
            Cmd::GlobalSearch,
            false,
        ),
        (
            Key::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
            Cmd::KillJob,
            false,
        ),
    ]
}

pub fn get_default_mappings() -> Vec<(Key, Cmd, bool)> {
    vec![
        (
            Key::new(KeyCode::Esc, KeyModifiers::empty()),
            Cmd::Escape,
            false,
        ),
        (
            Key::new(
                KeyCode::Char('t'),
                KeyModifiers::CONTROL | KeyModifiers::SHIFT,
            ),
            Cmd::ReopenBuffer,
            false,
        ),
        (
            Key::new(KeyCode::Char('r'), KeyModifiers::CONTROL),
            Cmd::Repeat,
            false,
        ),
        (
            Key::new(KeyCode::Char('w'), KeyModifiers::CONTROL),
            Cmd::Close,
            false,
        ),
        (
            Key::new(KeyCode::Char('n'), KeyModifiers::CONTROL),
            Cmd::New { path: None },
            false,
        ),
        (
            Key::new(KeyCode::Char('q'), KeyModifiers::CONTROL),
            Cmd::Quit,
            false,
        ),
        (
            Key::new(KeyCode::Char('s'), KeyModifiers::CONTROL),
            Cmd::Save { path: None },
            false,
        ),
        (
            Key::new(KeyCode::Char('a'), KeyModifiers::CONTROL),
            Cmd::SelectAll,
            false,
        ),
        (
            Key::new(KeyCode::Char('l'), KeyModifiers::CONTROL),
            Cmd::SelectLine,
            false,
        ),
        (
            Key::new(KeyCode::Char('d'), KeyModifiers::CONTROL),
            Cmd::SelectWord,
            false,
        ),
        (
            Key::new(
                KeyCode::Char('a'),
                KeyModifiers::CONTROL | KeyModifiers::SHIFT,
            ),
            Cmd::SelectAllMatching,
            false,
        ),
        (
            Key::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
            Cmd::Copy,
            false,
        ),
        (
            Key::new(KeyCode::Char('v'), KeyModifiers::CONTROL),
            Cmd::Paste,
            false,
        ),
        (
            Key::new(KeyCode::Char('x'), KeyModifiers::CONTROL),
            Cmd::Cut,
            false,
        ),
        (
            Key::new(KeyCode::Char('p'), KeyModifiers::CONTROL),
            Cmd::FocusPalette,
            false,
        ),
        (
            Key::new(KeyCode::Char('g'), KeyModifiers::CONTROL),
            Cmd::PromptGoto,
            false,
        ),
        (
            Key::new(KeyCode::Char('o'), KeyModifiers::CONTROL),
            Cmd::OpenFilePicker,
            false,
        ),
        (
            Key::new(KeyCode::Char('b'), KeyModifiers::CONTROL),
            Cmd::OpenBufferPicker,
            false,
        ),
        (
            Key::new(KeyCode::Tab, KeyModifiers::CONTROL),
            Cmd::OpenBufferPicker,
            false,
        ),
        (
            Key::new(KeyCode::Char('z'), KeyModifiers::CONTROL),
            Cmd::Undo,
            false,
        ),
        (
            Key::new(KeyCode::Char('y'), KeyModifiers::CONTROL),
            Cmd::Redo,
            false,
        ),
        (
            Key::new(KeyCode::Char('f'), KeyModifiers::CONTROL),
            Cmd::Search,
            false,
        ),
        (
            Key::new(
                KeyCode::Char('f'),
                KeyModifiers::CONTROL | KeyModifiers::SHIFT,
            ),
            Cmd::GlobalSearch,
            false,
        ),
        (
            Key::new(KeyCode::Char('r'), KeyModifiers::ALT),
            Cmd::Replace,
            false,
        ),
        (
            Key::new(KeyCode::Char('m'), KeyModifiers::ALT),
            Cmd::ReplaceCurrentMatch,
            false,
        ),
        (
            Key::new(KeyCode::Char('i'), KeyModifiers::ALT),
            Cmd::CaseInsensitive,
            false,
        ),
        (
            Key::new(KeyCode::Char('p'), KeyModifiers::ALT),
            Cmd::PrevMatch,
            false,
        ),
        (
            Key::new(KeyCode::Char('n'), KeyModifiers::ALT),
            Cmd::NextMatch,
            false,
        ),
        (
            Key::new(KeyCode::Tab, KeyModifiers::empty()),
            Cmd::TabOrIndent { back: false },
            false,
        ),
        (
            Key::new(KeyCode::BackTab, KeyModifiers::SHIFT),
            Cmd::TabOrIndent { back: true },
            false,
        ),
        (
            Key::new(KeyCode::Enter, KeyModifiers::SHIFT | KeyModifiers::CONTROL),
            Cmd::NewLineAboveWithoutBreaking,
            false,
        ),
        (
            Key::new(KeyCode::Enter, KeyModifiers::CONTROL),
            Cmd::NewLineWithoutBreaking,
            false,
        ),
        (
            Key::new(KeyCode::Enter, KeyModifiers::empty()),
            Cmd::Char { ch: '\n' },
            false,
        ),
        (
            Key::new(KeyCode::Delete, KeyModifiers::CONTROL | KeyModifiers::SHIFT),
            Cmd::DeleteToEndOfLine,
            true,
        ),
        (
            Key::new(KeyCode::Delete, KeyModifiers::CONTROL),
            Cmd::DeleteWord,
            true,
        ),
        (
            Key::new(KeyCode::Delete, KeyModifiers::empty()),
            Cmd::Delete,
            true,
        ),
        (
            Key::new(
                KeyCode::Backspace,
                KeyModifiers::CONTROL | KeyModifiers::SHIFT,
            ),
            Cmd::BackspaceToStartOfLine,
            true,
        ),
        (
            Key::new(KeyCode::Backspace, KeyModifiers::CONTROL),
            Cmd::BackspaceWord,
            true,
        ),
        (
            Key::new(KeyCode::Backspace, KeyModifiers::empty()),
            Cmd::Backspace,
            true,
        ),
        (
            Key::new(KeyCode::Home, KeyModifiers::empty()),
            Cmd::Home {
                expand_selection: false,
            },
            false,
        ),
        (
            Key::new(KeyCode::End, KeyModifiers::empty()),
            Cmd::End {
                expand_selection: false,
            },
            false,
        ),
        (
            Key::new(KeyCode::Home, KeyModifiers::SHIFT),
            Cmd::Home {
                expand_selection: true,
            },
            false,
        ),
        (
            Key::new(KeyCode::End, KeyModifiers::SHIFT),
            Cmd::End {
                expand_selection: true,
            },
            false,
        ),
        (
            Key::new(KeyCode::Home, KeyModifiers::CONTROL),
            Cmd::Start {
                expand_selection: false,
            },
            false,
        ),
        (
            Key::new(KeyCode::End, KeyModifiers::CONTROL),
            Cmd::Eof {
                expand_selection: false,
            },
            false,
        ),
        (
            Key::new(KeyCode::Home, KeyModifiers::SHIFT | KeyModifiers::CONTROL),
            Cmd::Start {
                expand_selection: true,
            },
            false,
        ),
        (
            Key::new(KeyCode::End, KeyModifiers::SHIFT | KeyModifiers::CONTROL),
            Cmd::Eof {
                expand_selection: true,
            },
            false,
        ),
        (
            Key::new(KeyCode::PageUp, KeyModifiers::empty()),
            Cmd::VerticalScroll { distance: -50 },
            false,
        ),
        (
            Key::new(KeyCode::PageDown, KeyModifiers::empty()),
            Cmd::VerticalScroll { distance: 50 },
            false,
        ),
        (
            Key::new(KeyCode::Up, KeyModifiers::ALT | KeyModifiers::SHIFT),
            Cmd::MoveUp {
                expand_selection: false,
                create_cursor: true,
                distance: 1,
            },
            false,
        ),
        (
            Key::new(KeyCode::Down, KeyModifiers::ALT | KeyModifiers::SHIFT),
            Cmd::MoveDown {
                expand_selection: false,
                create_cursor: true,
                distance: 1,
            },
            false,
        ),
        (
            Key::new(KeyCode::Up, KeyModifiers::CONTROL),
            Cmd::MoveUp {
                expand_selection: false,
                create_cursor: false,
                distance: 10,
            },
            false,
        ),
        (
            Key::new(KeyCode::Down, KeyModifiers::CONTROL),
            Cmd::MoveDown {
                expand_selection: false,
                create_cursor: false,
                distance: 10,
            },
            false,
        ),
        (
            Key::new(KeyCode::Up, KeyModifiers::CONTROL | KeyModifiers::SHIFT),
            Cmd::MoveUp {
                expand_selection: true,
                create_cursor: false,
                distance: 10,
            },
            false,
        ),
        (
            Key::new(KeyCode::Down, KeyModifiers::CONTROL | KeyModifiers::SHIFT),
            Cmd::MoveDown {
                expand_selection: true,
                create_cursor: false,
                distance: 10,
            },
            false,
        ),
        (
            Key::new(KeyCode::Up, KeyModifiers::CONTROL),
            Cmd::MoveUp {
                expand_selection: false,
                create_cursor: false,
                distance: 10,
            },
            false,
        ),
        (
            Key::new(KeyCode::Down, KeyModifiers::CONTROL),
            Cmd::MoveDown {
                expand_selection: false,
                create_cursor: false,
                distance: 10,
            },
            false,
        ),
        (
            Key::new(KeyCode::Up, KeyModifiers::CONTROL | KeyModifiers::SHIFT),
            Cmd::MoveUp {
                expand_selection: true,
                create_cursor: false,
                distance: 10,
            },
            false,
        ),
        (
            Key::new(KeyCode::Down, KeyModifiers::CONTROL | KeyModifiers::SHIFT),
            Cmd::MoveDown {
                expand_selection: true,
                create_cursor: false,
                distance: 10,
            },
            false,
        ),
        (
            Key::new(KeyCode::Up, KeyModifiers::empty()),
            Cmd::MoveUp {
                expand_selection: false,
                create_cursor: false,
                distance: 1,
            },
            false,
        ),
        (
            Key::new(KeyCode::Down, KeyModifiers::empty()),
            Cmd::MoveDown {
                expand_selection: false,
                create_cursor: false,
                distance: 1,
            },
            false,
        ),
        (
            Key::new(KeyCode::Up, KeyModifiers::SHIFT),
            Cmd::MoveUp {
                expand_selection: true,
                create_cursor: false,
                distance: 1,
            },
            false,
        ),
        (
            Key::new(KeyCode::Down, KeyModifiers::SHIFT),
            Cmd::MoveDown {
                expand_selection: true,
                create_cursor: false,
                distance: 1,
            },
            false,
        ),
        (
            Key::new(KeyCode::Right, KeyModifiers::empty()),
            Cmd::MoveRight {
                expand_selection: false,
            },
            false,
        ),
        (
            Key::new(KeyCode::Left, KeyModifiers::empty()),
            Cmd::MoveLeft {
                expand_selection: false,
            },
            false,
        ),
        (
            Key::new(KeyCode::Right, KeyModifiers::SHIFT),
            Cmd::MoveRight {
                expand_selection: true,
            },
            false,
        ),
        (
            Key::new(KeyCode::Left, KeyModifiers::SHIFT),
            Cmd::MoveLeft {
                expand_selection: true,
            },
            false,
        ),
        (
            Key::new(KeyCode::Right, KeyModifiers::CONTROL),
            Cmd::MoveRightWord {
                expand_selection: false,
            },
            false,
        ),
        (
            Key::new(KeyCode::Left, KeyModifiers::CONTROL),
            Cmd::MoveLeftWord {
                expand_selection: false,
            },
            false,
        ),
        (
            Key::new(KeyCode::Right, KeyModifiers::SHIFT | KeyModifiers::CONTROL),
            Cmd::MoveRightWord {
                expand_selection: true,
            },
            false,
        ),
        (
            Key::new(KeyCode::Left, KeyModifiers::SHIFT | KeyModifiers::CONTROL),
            Cmd::MoveLeftWord {
                expand_selection: true,
            },
            false,
        ),
        (
            Key::new(KeyCode::Up, KeyModifiers::ALT),
            Cmd::MoveLine {
                direction: LineMoveDir::Up,
            },
            false,
        ),
        (
            Key::new(KeyCode::Down, KeyModifiers::ALT),
            Cmd::MoveLine {
                direction: LineMoveDir::Down,
            },
            false,
        ),
        (
            Key::new(KeyCode::Char('+'), KeyModifiers::ALT),
            Cmd::GrowPane,
            false,
        ),
        (
            Key::new(KeyCode::Char('-'), KeyModifiers::ALT),
            Cmd::ShrinkPane,
            false,
        ),
        (
            Key::new(KeyCode::Up, KeyModifiers::CONTROL | KeyModifiers::ALT),
            Cmd::SwitchPane {
                direction: Direction::Up,
            },
            false,
        ),
        (
            Key::new(KeyCode::Down, KeyModifiers::CONTROL | KeyModifiers::ALT),
            Cmd::SwitchPane {
                direction: Direction::Down,
            },
            false,
        ),
        (
            Key::new(KeyCode::Right, KeyModifiers::CONTROL | KeyModifiers::ALT),
            Cmd::SwitchPane {
                direction: Direction::Right,
            },
            false,
        ),
        (
            Key::new(KeyCode::Left, KeyModifiers::CONTROL | KeyModifiers::ALT),
            Cmd::SwitchPane {
                direction: Direction::Left,
            },
            false,
        ),
        (
            Key::new(KeyCode::Char('e'), KeyModifiers::CONTROL),
            Cmd::OpenFileExplorer { path: None },
            false,
        ),
        (
            Key::new(KeyCode::Char('+'), KeyModifiers::CONTROL),
            Cmd::ZoomIn,
            false,
        ),
        (
            Key::new(KeyCode::Char('-'), KeyModifiers::CONTROL),
            Cmd::ZoomOut,
            false,
        ),
        (
            Key::new(KeyCode::F5, KeyModifiers::empty()),
            Cmd::RunAction {
                name: "build".into(),
            },
            false,
        ),
        (
            Key::new(
                KeyCode::Char('k'),
                KeyModifiers::CONTROL | KeyModifiers::SHIFT,
            ),
            Cmd::RemoveLine,
            false,
        ),
        (
            Key::new(KeyCode::Char('k'), KeyModifiers::CONTROL),
            Cmd::InputMode {
                name: "chords".into(),
            },
            false,
        ),
    ]
}

impl Serialize for Key {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut output = String::new();
        if let Some(modifiers) = self.modifiers.try_to_string() {
            output.push_str(&modifiers);
        }
        if self.modifiers != KeyModifiers::empty() {
            output.push('-');
        }
        output.push_str(&self.keycode.to_string());
        serializer.serialize_str(&output)
    }
}

impl<'de> Deserialize<'de> for Key {
    fn deserialize<D>(deserializer: D) -> Result<Key, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct KeyVisitor;

        impl Visitor<'_> for KeyVisitor {
            type Value = Key;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("key mapping")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let strs = value.split("-");
                let mut keycode = None;
                let mut modifiers = KeyModifiers::empty();
                for s in strs {
                    if let Some(modifier) = KeyModifiers::try_from_str(s) {
                        modifiers |= modifier;
                        continue;
                    }
                    let k = match KeyCode::try_from_str(s) {
                        Ok(k) => k,
                        Err(err) => return Err(de::Error::custom(err)),
                    };
                    if keycode.is_some() {
                        return Err(de::Error::custom(
                            "only one non modifier key per keybinding",
                        ));
                    }
                    keycode = Some(k);
                }

                let keycode = match keycode {
                    Some(keycode) => keycode,
                    None => {
                        return Err(de::Error::custom(
                            "every keybinding must have a non modifier key",
                        ))
                    }
                };

                Ok(Key { keycode, modifiers })
            }
        }

        deserializer.deserialize_string(KeyVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_char_key() {
        let key = Key {
            keycode: KeyCode::Char('A'),
            modifiers: KeyModifiers::ALT | KeyModifiers::SHIFT,
        };
        let s = serde_json::to_string(&key).unwrap();
        let parsed = serde_json::from_str(&s);
        assert!(parsed.is_ok());
        assert_eq!(key, parsed.unwrap());
    }

    #[test]
    fn serde_esc_key() {
        let key = Key {
            keycode: KeyCode::Esc,
            modifiers: KeyModifiers::META | KeyModifiers::CONTROL,
        };
        let s = serde_json::to_string(&key).unwrap();
        let parsed = serde_json::from_str(&s);
        assert!(parsed.is_ok());
        assert_eq!(key, parsed.unwrap());
    }

    #[test]
    fn serde_space_key() {
        let key = Key {
            keycode: KeyCode::Char('b'),
            modifiers: KeyModifiers::empty(),
        };
        let s = serde_json::to_string(&key).unwrap();
        let parsed = serde_json::from_str(&s);
        assert!(parsed.is_ok());
        assert_eq!(key, parsed.unwrap());
    }
}
