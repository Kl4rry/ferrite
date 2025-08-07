pub mod keycode;
use std::fmt;

use keycode::{KeyCode, KeyModifiers};
use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{self, Visitor},
};

use crate::{
    cmd::{Cmd, LineMoveDir},
    config::keymap::Keymapping,
    layout::panes::Direction,
};

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputContext {
    #[default]
    All,
    Edit,
    FileExplorer,
}

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
    input_ctx: InputContext,
) -> Option<Cmd> {
    let normalized_keycode = match keycode {
        KeyCode::Char(ch) => KeyCode::Char(ch.to_ascii_lowercase()),
        keycode => keycode,
    };
    for Keymapping {
        key,
        cmd,
        ignore_modifiers,
        ctx,
    } in mappings
    {
        let ctx_match = *ctx == input_ctx || *ctx == InputContext::All;
        if !ctx_match {
            continue;
        }
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

    if let KeyCode::Char(ch) = keycode
        && (!ch.is_ascii_alphanumeric()
            || modifiers == KeyModifiers::empty()
            || modifiers == KeyModifiers::SHIFT)
    {
        return Some(Cmd::Char { ch });
    }

    None
}

pub fn get_default_chords() -> Vec<(Key, Cmd, bool, InputContext)> {
    vec![
        (
            Key::new(KeyCode::Esc, KeyModifiers::empty()),
            Cmd::Escape,
            false,
            InputContext::All,
        ),
        (
            Key::new(KeyCode::Char('k'), KeyModifiers::CONTROL),
            Cmd::InputMode {
                name: "normal".into(),
            },
            false,
            InputContext::All,
        ),
        (
            Key::new(KeyCode::Char('r'), KeyModifiers::CONTROL),
            Cmd::Repeat,
            false,
            InputContext::Edit,
        ),
        (
            Key::new(KeyCode::Char('f'), KeyModifiers::CONTROL),
            Cmd::Format,
            false,
            InputContext::All,
        ),
        (
            Key::new(KeyCode::Char('s'), KeyModifiers::CONTROL),
            Cmd::OpenShellPalette,
            false,
            InputContext::All,
        ),
        (
            Key::new(KeyCode::Char('o'), KeyModifiers::CONTROL),
            Cmd::UrlOpen,
            false,
            InputContext::Edit,
        ),
        (
            Key::new(KeyCode::Char('v'), KeyModifiers::CONTROL),
            Cmd::Split {
                direction: Direction::Right,
            },
            false,
            InputContext::All,
        ),
        (
            Key::new(KeyCode::Char('h'), KeyModifiers::CONTROL),
            Cmd::Split {
                direction: Direction::Down,
            },
            false,
            InputContext::All,
        ),
        (
            Key::new(KeyCode::Char('j'), KeyModifiers::CONTROL),
            Cmd::RotateFile,
            false,
            InputContext::Edit,
        ),
        (
            Key::new(KeyCode::Char('w'), KeyModifiers::CONTROL),
            Cmd::ClosePane,
            false,
            InputContext::All,
        ),
        (
            Key::new(KeyCode::Char('g'), KeyModifiers::CONTROL),
            Cmd::GlobalSearch,
            false,
            InputContext::All,
        ),
        (
            Key::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
            Cmd::KillJob,
            false,
            InputContext::Edit,
        ),
    ]
}

pub fn get_default_mappings() -> Vec<(Key, Cmd, bool, InputContext)> {
    vec![
        (
            Key::new(KeyCode::Esc, KeyModifiers::empty()),
            Cmd::Escape,
            false,
            InputContext::All,
        ),
        (
            Key::new(
                KeyCode::Char('t'),
                KeyModifiers::CONTROL | KeyModifiers::SHIFT,
            ),
            Cmd::ReopenBuffer,
            false,
            InputContext::All,
        ),
        (
            Key::new(KeyCode::Char('r'), KeyModifiers::empty()),
            Cmd::OpenRename,
            false,
            InputContext::FileExplorer,
        ),
        (
            Key::new(KeyCode::Char('w'), KeyModifiers::CONTROL),
            Cmd::Close,
            false,
            InputContext::All,
        ),
        (
            Key::new(KeyCode::Char('n'), KeyModifiers::CONTROL),
            Cmd::New { path: None },
            false,
            InputContext::All,
        ),
        (
            Key::new(KeyCode::Char('q'), KeyModifiers::CONTROL),
            Cmd::Quit,
            false,
            InputContext::All,
        ),
        (
            Key::new(KeyCode::Char('s'), KeyModifiers::CONTROL),
            Cmd::Save { path: None },
            false,
            InputContext::Edit,
        ),
        (
            Key::new(KeyCode::Char('a'), KeyModifiers::CONTROL),
            Cmd::SelectAll,
            false,
            InputContext::Edit,
        ),
        (
            Key::new(KeyCode::Char('l'), KeyModifiers::CONTROL),
            Cmd::SelectLine,
            false,
            InputContext::Edit,
        ),
        (
            Key::new(KeyCode::Char('d'), KeyModifiers::CONTROL),
            Cmd::SelectWord,
            false,
            InputContext::Edit,
        ),
        (
            Key::new(
                KeyCode::Char('a'),
                KeyModifiers::CONTROL | KeyModifiers::SHIFT,
            ),
            Cmd::SelectAllMatching,
            false,
            InputContext::Edit,
        ),
        (
            Key::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
            Cmd::Copy,
            false,
            InputContext::All,
        ),
        (
            Key::new(KeyCode::Char('v'), KeyModifiers::CONTROL),
            Cmd::Paste,
            false,
            InputContext::All,
        ),
        (
            Key::new(KeyCode::Char('x'), KeyModifiers::CONTROL),
            Cmd::Cut,
            false,
            InputContext::All,
        ),
        (
            Key::new(KeyCode::Char('p'), KeyModifiers::CONTROL),
            Cmd::FocusPalette,
            false,
            InputContext::All,
        ),
        (
            Key::new(KeyCode::Char('g'), KeyModifiers::CONTROL),
            Cmd::PromptGoto,
            false,
            InputContext::Edit,
        ),
        (
            Key::new(KeyCode::Char('o'), KeyModifiers::CONTROL),
            Cmd::OpenFilePicker,
            false,
            InputContext::All,
        ),
        (
            Key::new(KeyCode::Char('b'), KeyModifiers::CONTROL),
            Cmd::OpenBufferPicker,
            false,
            InputContext::All,
        ),
        (
            Key::new(KeyCode::Tab, KeyModifiers::CONTROL),
            Cmd::OpenBufferPicker,
            false,
            InputContext::All,
        ),
        (
            Key::new(KeyCode::Char('z'), KeyModifiers::CONTROL),
            Cmd::Undo,
            false,
            InputContext::Edit,
        ),
        (
            Key::new(KeyCode::Char('y'), KeyModifiers::CONTROL),
            Cmd::Redo,
            false,
            InputContext::Edit,
        ),
        (
            Key::new(KeyCode::Char('f'), KeyModifiers::CONTROL),
            Cmd::Search,
            false,
            InputContext::All,
        ),
        (
            Key::new(
                KeyCode::Char('f'),
                KeyModifiers::CONTROL | KeyModifiers::SHIFT,
            ),
            Cmd::GlobalSearch,
            false,
            InputContext::All,
        ),
        (
            Key::new(KeyCode::Char('r'), KeyModifiers::CONTROL),
            Cmd::Replace,
            false,
            InputContext::Edit,
        ),
        (
            Key::new(KeyCode::Char('m'), KeyModifiers::ALT),
            Cmd::ReplaceCurrentMatch,
            false,
            InputContext::Edit,
        ),
        (
            Key::new(KeyCode::Char('i'), KeyModifiers::ALT),
            Cmd::CaseInsensitive,
            false,
            InputContext::All,
        ),
        (
            Key::new(KeyCode::Char('p'), KeyModifiers::ALT),
            Cmd::PrevMatch,
            false,
            InputContext::Edit,
        ),
        (
            Key::new(KeyCode::Char('n'), KeyModifiers::ALT),
            Cmd::NextMatch,
            false,
            InputContext::Edit,
        ),
        (
            Key::new(KeyCode::Tab, KeyModifiers::empty()),
            Cmd::TabOrIndent { back: false },
            false,
            InputContext::Edit,
        ),
        (
            Key::new(KeyCode::BackTab, KeyModifiers::SHIFT),
            Cmd::TabOrIndent { back: true },
            false,
            InputContext::Edit,
        ),
        (
            Key::new(KeyCode::Enter, KeyModifiers::SHIFT | KeyModifiers::CONTROL),
            Cmd::NewLineAboveWithoutBreaking,
            false,
            InputContext::Edit,
        ),
        (
            Key::new(KeyCode::Enter, KeyModifiers::CONTROL),
            Cmd::NewLineWithoutBreaking,
            false,
            InputContext::Edit,
        ),
        (
            Key::new(KeyCode::Enter, KeyModifiers::empty()),
            Cmd::Enter,
            false,
            InputContext::All,
        ),
        (
            Key::new(KeyCode::Delete, KeyModifiers::CONTROL | KeyModifiers::SHIFT),
            Cmd::DeleteToEndOfLine,
            true,
            InputContext::Edit,
        ),
        (
            Key::new(KeyCode::Delete, KeyModifiers::CONTROL),
            Cmd::DeleteWord,
            true,
            InputContext::Edit,
        ),
        (
            Key::new(KeyCode::Delete, KeyModifiers::empty()),
            Cmd::Delete,
            true,
            InputContext::All,
        ),
        (
            Key::new(
                KeyCode::Backspace,
                KeyModifiers::CONTROL | KeyModifiers::SHIFT,
            ),
            Cmd::BackspaceToStartOfLine,
            true,
            InputContext::All,
        ),
        (
            Key::new(KeyCode::Backspace, KeyModifiers::CONTROL),
            Cmd::BackspaceWord,
            true,
            InputContext::All,
        ),
        (
            Key::new(KeyCode::Backspace, KeyModifiers::empty()),
            Cmd::Backspace,
            true,
            InputContext::All,
        ),
        (
            Key::new(KeyCode::Home, KeyModifiers::empty()),
            Cmd::Home {
                expand_selection: false,
            },
            false,
            InputContext::Edit,
        ),
        (
            Key::new(KeyCode::End, KeyModifiers::empty()),
            Cmd::End {
                expand_selection: false,
            },
            false,
            InputContext::Edit,
        ),
        (
            Key::new(KeyCode::Home, KeyModifiers::SHIFT),
            Cmd::Home {
                expand_selection: true,
            },
            false,
            InputContext::Edit,
        ),
        (
            Key::new(KeyCode::End, KeyModifiers::SHIFT),
            Cmd::End {
                expand_selection: true,
            },
            false,
            InputContext::Edit,
        ),
        (
            Key::new(KeyCode::Home, KeyModifiers::CONTROL),
            Cmd::Start {
                expand_selection: false,
            },
            false,
            InputContext::Edit,
        ),
        (
            Key::new(KeyCode::End, KeyModifiers::CONTROL),
            Cmd::Eof {
                expand_selection: false,
            },
            false,
            InputContext::Edit,
        ),
        (
            Key::new(KeyCode::Home, KeyModifiers::SHIFT | KeyModifiers::CONTROL),
            Cmd::Start {
                expand_selection: true,
            },
            false,
            InputContext::Edit,
        ),
        (
            Key::new(KeyCode::End, KeyModifiers::SHIFT | KeyModifiers::CONTROL),
            Cmd::Eof {
                expand_selection: true,
            },
            false,
            InputContext::Edit,
        ),
        (
            Key::new(KeyCode::PageUp, KeyModifiers::empty()),
            Cmd::PageUp,
            false,
            InputContext::Edit,
        ),
        (
            Key::new(KeyCode::PageDown, KeyModifiers::empty()),
            Cmd::PageDown,
            false,
            InputContext::Edit,
        ),
        (
            Key::new(KeyCode::Up, KeyModifiers::ALT | KeyModifiers::SHIFT),
            Cmd::MoveUp {
                expand_selection: false,
                create_cursor: true,
                distance: 1,
            },
            false,
            InputContext::Edit,
        ),
        (
            Key::new(KeyCode::Down, KeyModifiers::ALT | KeyModifiers::SHIFT),
            Cmd::MoveDown {
                expand_selection: false,
                create_cursor: true,
                distance: 1,
            },
            false,
            InputContext::Edit,
        ),
        (
            Key::new(KeyCode::Up, KeyModifiers::CONTROL),
            Cmd::MoveUp {
                expand_selection: false,
                create_cursor: false,
                distance: 10,
            },
            false,
            InputContext::All,
        ),
        (
            Key::new(KeyCode::Down, KeyModifiers::CONTROL),
            Cmd::MoveDown {
                expand_selection: false,
                create_cursor: false,
                distance: 10,
            },
            false,
            InputContext::All,
        ),
        (
            Key::new(KeyCode::Up, KeyModifiers::CONTROL | KeyModifiers::SHIFT),
            Cmd::MoveUp {
                expand_selection: true,
                create_cursor: false,
                distance: 10,
            },
            false,
            InputContext::All,
        ),
        (
            Key::new(KeyCode::Down, KeyModifiers::CONTROL | KeyModifiers::SHIFT),
            Cmd::MoveDown {
                expand_selection: true,
                create_cursor: false,
                distance: 10,
            },
            false,
            InputContext::All,
        ),
        (
            Key::new(KeyCode::Up, KeyModifiers::CONTROL),
            Cmd::MoveUp {
                expand_selection: false,
                create_cursor: false,
                distance: 10,
            },
            false,
            InputContext::All,
        ),
        (
            Key::new(KeyCode::Down, KeyModifiers::CONTROL),
            Cmd::MoveDown {
                expand_selection: false,
                create_cursor: false,
                distance: 10,
            },
            false,
            InputContext::All,
        ),
        (
            Key::new(KeyCode::Up, KeyModifiers::CONTROL | KeyModifiers::SHIFT),
            Cmd::MoveUp {
                expand_selection: true,
                create_cursor: false,
                distance: 10,
            },
            false,
            InputContext::All,
        ),
        (
            Key::new(KeyCode::Down, KeyModifiers::CONTROL | KeyModifiers::SHIFT),
            Cmd::MoveDown {
                expand_selection: true,
                create_cursor: false,
                distance: 10,
            },
            false,
            InputContext::All,
        ),
        (
            Key::new(KeyCode::Up, KeyModifiers::empty()),
            Cmd::MoveUp {
                expand_selection: false,
                create_cursor: false,
                distance: 1,
            },
            false,
            InputContext::All,
        ),
        (
            Key::new(KeyCode::Down, KeyModifiers::empty()),
            Cmd::MoveDown {
                expand_selection: false,
                create_cursor: false,
                distance: 1,
            },
            false,
            InputContext::All,
        ),
        (
            Key::new(KeyCode::Up, KeyModifiers::SHIFT),
            Cmd::MoveUp {
                expand_selection: true,
                create_cursor: false,
                distance: 1,
            },
            false,
            InputContext::All,
        ),
        (
            Key::new(KeyCode::Down, KeyModifiers::SHIFT),
            Cmd::MoveDown {
                expand_selection: true,
                create_cursor: false,
                distance: 1,
            },
            false,
            InputContext::All,
        ),
        (
            Key::new(KeyCode::Right, KeyModifiers::empty()),
            Cmd::MoveRight {
                expand_selection: false,
            },
            false,
            InputContext::All,
        ),
        (
            Key::new(KeyCode::Left, KeyModifiers::empty()),
            Cmd::MoveLeft {
                expand_selection: false,
            },
            false,
            InputContext::All,
        ),
        (
            Key::new(KeyCode::Right, KeyModifiers::SHIFT),
            Cmd::MoveRight {
                expand_selection: true,
            },
            false,
            InputContext::All,
        ),
        (
            Key::new(KeyCode::Left, KeyModifiers::SHIFT),
            Cmd::MoveLeft {
                expand_selection: true,
            },
            false,
            InputContext::All,
        ),
        (
            Key::new(KeyCode::Right, KeyModifiers::CONTROL),
            Cmd::MoveRightWord {
                expand_selection: false,
            },
            false,
            InputContext::All,
        ),
        (
            Key::new(KeyCode::Left, KeyModifiers::CONTROL),
            Cmd::MoveLeftWord {
                expand_selection: false,
            },
            false,
            InputContext::All,
        ),
        (
            Key::new(KeyCode::Right, KeyModifiers::SHIFT | KeyModifiers::CONTROL),
            Cmd::MoveRightWord {
                expand_selection: true,
            },
            false,
            InputContext::All,
        ),
        (
            Key::new(KeyCode::Left, KeyModifiers::SHIFT | KeyModifiers::CONTROL),
            Cmd::MoveLeftWord {
                expand_selection: true,
            },
            false,
            InputContext::All,
        ),
        (
            Key::new(KeyCode::Up, KeyModifiers::ALT),
            Cmd::MoveLine {
                direction: LineMoveDir::Up,
            },
            false,
            InputContext::Edit,
        ),
        (
            Key::new(KeyCode::Down, KeyModifiers::ALT),
            Cmd::MoveLine {
                direction: LineMoveDir::Down,
            },
            false,
            InputContext::Edit,
        ),
        (
            Key::new(KeyCode::Char('+'), KeyModifiers::ALT),
            Cmd::GrowPane,
            false,
            InputContext::All,
        ),
        (
            Key::new(KeyCode::Char('-'), KeyModifiers::ALT),
            Cmd::ShrinkPane,
            false,
            InputContext::All,
        ),
        (
            Key::new(KeyCode::Up, KeyModifiers::CONTROL | KeyModifiers::ALT),
            Cmd::SwitchPane {
                direction: Direction::Up,
            },
            false,
            InputContext::All,
        ),
        (
            Key::new(KeyCode::Down, KeyModifiers::CONTROL | KeyModifiers::ALT),
            Cmd::SwitchPane {
                direction: Direction::Down,
            },
            false,
            InputContext::All,
        ),
        (
            Key::new(KeyCode::Right, KeyModifiers::CONTROL | KeyModifiers::ALT),
            Cmd::SwitchPane {
                direction: Direction::Right,
            },
            false,
            InputContext::All,
        ),
        (
            Key::new(KeyCode::Left, KeyModifiers::CONTROL | KeyModifiers::ALT),
            Cmd::SwitchPane {
                direction: Direction::Left,
            },
            false,
            InputContext::All,
        ),
        (
            Key::new(KeyCode::Char('e'), KeyModifiers::CONTROL),
            Cmd::OpenFileExplorer { path: None },
            false,
            InputContext::All,
        ),
        (
            Key::new(KeyCode::Char('+'), KeyModifiers::CONTROL),
            Cmd::ZoomIn,
            false,
            InputContext::All,
        ),
        (
            Key::new(KeyCode::Char('-'), KeyModifiers::CONTROL),
            Cmd::ZoomOut,
            false,
            InputContext::All,
        ),
        (
            Key::new(KeyCode::F5, KeyModifiers::empty()),
            Cmd::RunAction {
                name: "build".into(),
            },
            false,
            InputContext::All,
        ),
        (
            Key::new(
                KeyCode::Char('k'),
                KeyModifiers::CONTROL | KeyModifiers::SHIFT,
            ),
            Cmd::RemoveLine,
            false,
            InputContext::Edit,
        ),
        (
            Key::new(KeyCode::Char('k'), KeyModifiers::CONTROL),
            Cmd::InputMode {
                name: "chords".into(),
            },
            false,
            InputContext::All,
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
                        ));
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
