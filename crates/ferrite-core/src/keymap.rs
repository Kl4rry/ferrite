pub mod keycode;
use keycode::{KeyCode, KeyModifiers};

use crate::{
    cmd::{Cmd, LineMoveDir},
    layout::panes::Direction,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mapping {
    pub keycode: KeyCode,
    pub modifiers: KeyModifiers,
}

impl Mapping {
    pub const fn new(keycode: KeyCode, modifiers: KeyModifiers) -> Self {
        Self { keycode, modifiers }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Exclusiveness {
    Exclusive,
    #[allow(dead_code)]
    NonExclusive,
    Ignores(KeyModifiers),
}

pub fn get_command_from_input(
    keycode: KeyCode,
    modifiers: KeyModifiers,
    mappings: &[(Mapping, Cmd, Exclusiveness)],
) -> Option<Cmd> {
    let normalized_keycode = match keycode {
        KeyCode::Char(ch) => KeyCode::Char(ch.to_ascii_lowercase()),
        keycode => keycode,
    };
    for (mapping, cmd, exclusiveness) in mappings {
        match exclusiveness {
            Exclusiveness::Exclusive => {
                if *mapping
                    == (Mapping {
                        keycode: normalized_keycode,
                        modifiers,
                    })
                {
                    return Some(cmd.clone());
                }
            }
            Exclusiveness::NonExclusive => {
                if mapping.keycode == normalized_keycode && modifiers.contains(mapping.modifiers) {
                    return Some(cmd.clone());
                }
            }
            Exclusiveness::Ignores(ignored) => {
                let mut non_ignored = modifiers;
                non_ignored.remove(*ignored);
                if mapping.keycode == normalized_keycode && non_ignored == mapping.modifiers {
                    return Some(cmd.clone());
                }
            }
        }
    }

    if let KeyCode::Char(ch) = keycode {
        if !ch.is_ascii_alphanumeric()
            || modifiers == KeyModifiers::empty()
            || modifiers == KeyModifiers::SHIFT
        {
            return Some(Cmd::Char(ch));
        }
    }

    None
}

pub fn get_default_choords() -> Vec<(Mapping, Cmd, Exclusiveness)> {
    vec![
        (
            Mapping::new(KeyCode::Esc, KeyModifiers::NONE),
            Cmd::Escape,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('k'), KeyModifiers::CONTROL),
            Cmd::Choord,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('f'), KeyModifiers::CONTROL),
            Cmd::Format,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('s'), KeyModifiers::CONTROL),
            Cmd::OpenShellPalette,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('o'), KeyModifiers::CONTROL),
            Cmd::UrlOpen,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('r'), KeyModifiers::CONTROL),
            Cmd::Split(Direction::Right),
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('l'), KeyModifiers::CONTROL),
            Cmd::Split(Direction::Left),
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('u'), KeyModifiers::CONTROL),
            Cmd::Split(Direction::Up),
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('d'), KeyModifiers::CONTROL),
            Cmd::Split(Direction::Down),
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('j'), KeyModifiers::CONTROL),
            Cmd::RotateFile,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('w'), KeyModifiers::CONTROL),
            Cmd::ClosePane,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('g'), KeyModifiers::CONTROL),
            Cmd::GlobalSearch,
            Exclusiveness::Exclusive,
        ),
    ]
}

pub fn get_default_mappings() -> Vec<(Mapping, Cmd, Exclusiveness)> {
    vec![
        (
            Mapping::new(KeyCode::Esc, KeyModifiers::NONE),
            Cmd::Escape,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(
                KeyCode::Char('t'),
                KeyModifiers::CONTROL | KeyModifiers::SHIFT,
            ),
            Cmd::ReopenBuffer,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('r'), KeyModifiers::CONTROL),
            Cmd::Repeat,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('w'), KeyModifiers::CONTROL),
            Cmd::Close,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('n'), KeyModifiers::CONTROL),
            Cmd::New(None),
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('q'), KeyModifiers::CONTROL),
            Cmd::Quit,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('s'), KeyModifiers::CONTROL),
            Cmd::Save,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('a'), KeyModifiers::CONTROL),
            Cmd::SelectAll,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('l'), KeyModifiers::CONTROL),
            Cmd::SelectLine,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('d'), KeyModifiers::CONTROL),
            Cmd::SelectWord,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
            Cmd::Copy,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('v'), KeyModifiers::CONTROL),
            Cmd::Paste,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('x'), KeyModifiers::CONTROL),
            Cmd::Cut,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('p'), KeyModifiers::CONTROL),
            Cmd::FocusPalette,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('g'), KeyModifiers::CONTROL),
            Cmd::PromptGoto,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('o'), KeyModifiers::CONTROL),
            Cmd::OpenFileBrowser,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('b'), KeyModifiers::CONTROL),
            Cmd::OpenBufferBrowser,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('z'), KeyModifiers::CONTROL),
            Cmd::Undo,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('y'), KeyModifiers::CONTROL),
            Cmd::Redo,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('f'), KeyModifiers::CONTROL),
            Cmd::Search,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('r'), KeyModifiers::ALT),
            Cmd::Replace,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('m'), KeyModifiers::ALT),
            Cmd::ReplaceCurrentMatch,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('i'), KeyModifiers::ALT),
            Cmd::CaseInsensitive,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('p'), KeyModifiers::ALT),
            Cmd::PrevMatch,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('n'), KeyModifiers::ALT),
            Cmd::NextMatch,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Tab, KeyModifiers::NONE),
            Cmd::Tab { back: false },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::BackTab, KeyModifiers::SHIFT),
            Cmd::Tab { back: true },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Enter, KeyModifiers::CONTROL),
            Cmd::NewLine,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Enter, KeyModifiers::NONE),
            Cmd::Char('\n'),
            Exclusiveness::Ignores(KeyModifiers::SHIFT | KeyModifiers::SUPER | KeyModifiers::ALT),
        ),
        (
            Mapping::new(KeyCode::Backspace, KeyModifiers::NONE),
            Cmd::Backspace,
            Exclusiveness::Ignores(KeyModifiers::SHIFT | KeyModifiers::SUPER | KeyModifiers::ALT),
        ),
        (
            Mapping::new(KeyCode::Delete, KeyModifiers::NONE),
            Cmd::Delete,
            Exclusiveness::Ignores(KeyModifiers::SHIFT | KeyModifiers::SUPER | KeyModifiers::ALT),
        ),
        (
            Mapping::new(KeyCode::Backspace, KeyModifiers::CONTROL),
            Cmd::BackspaceWord,
            Exclusiveness::Ignores(KeyModifiers::SHIFT | KeyModifiers::SUPER | KeyModifiers::ALT),
        ),
        (
            Mapping::new(KeyCode::Delete, KeyModifiers::CONTROL),
            Cmd::DeleteWord,
            Exclusiveness::Ignores(KeyModifiers::SHIFT | KeyModifiers::SUPER | KeyModifiers::ALT),
        ),
        (
            Mapping::new(KeyCode::Home, KeyModifiers::NONE),
            Cmd::Home {
                expand_selection: false,
            },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::End, KeyModifiers::NONE),
            Cmd::End {
                expand_selection: false,
            },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Home, KeyModifiers::SHIFT),
            Cmd::Home {
                expand_selection: true,
            },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::End, KeyModifiers::SHIFT),
            Cmd::End {
                expand_selection: true,
            },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Home, KeyModifiers::CONTROL),
            Cmd::Start {
                expand_selection: false,
            },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::End, KeyModifiers::CONTROL),
            Cmd::Eof {
                expand_selection: false,
            },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Home, KeyModifiers::SHIFT | KeyModifiers::CONTROL),
            Cmd::Start {
                expand_selection: true,
            },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::End, KeyModifiers::SHIFT | KeyModifiers::CONTROL),
            Cmd::Eof {
                expand_selection: true,
            },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::PageUp, KeyModifiers::NONE),
            Cmd::VerticalScroll(-50),
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::PageDown, KeyModifiers::NONE),
            Cmd::VerticalScroll(50),
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Up, KeyModifiers::CONTROL),
            Cmd::MoveUp {
                expand_selection: false,
                distance: 10,
            },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Down, KeyModifiers::CONTROL),
            Cmd::MoveDown {
                expand_selection: false,
                distance: 10,
            },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Up, KeyModifiers::CONTROL | KeyModifiers::SHIFT),
            Cmd::MoveUp {
                expand_selection: true,
                distance: 10,
            },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Down, KeyModifiers::CONTROL | KeyModifiers::SHIFT),
            Cmd::MoveDown {
                expand_selection: true,
                distance: 10,
            },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Up, KeyModifiers::CONTROL),
            Cmd::MoveUp {
                expand_selection: false,
                distance: 10,
            },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Down, KeyModifiers::CONTROL),
            Cmd::MoveDown {
                expand_selection: false,
                distance: 10,
            },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Up, KeyModifiers::CONTROL | KeyModifiers::SHIFT),
            Cmd::MoveUp {
                expand_selection: true,
                distance: 10,
            },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Down, KeyModifiers::CONTROL | KeyModifiers::SHIFT),
            Cmd::MoveDown {
                expand_selection: true,
                distance: 10,
            },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Up, KeyModifiers::NONE),
            Cmd::MoveUp {
                expand_selection: false,
                distance: 1,
            },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Down, KeyModifiers::NONE),
            Cmd::MoveDown {
                expand_selection: false,
                distance: 1,
            },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Up, KeyModifiers::SHIFT),
            Cmd::MoveUp {
                expand_selection: true,
                distance: 1,
            },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Down, KeyModifiers::SHIFT),
            Cmd::MoveDown {
                expand_selection: true,
                distance: 1,
            },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('i'), KeyModifiers::CONTROL),
            Cmd::MoveUp {
                expand_selection: false,
                distance: 1,
            },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('j'), KeyModifiers::CONTROL),
            Cmd::MoveDown {
                expand_selection: false,
                distance: 1,
            },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(
                KeyCode::Char('i'),
                KeyModifiers::SHIFT | KeyModifiers::CONTROL,
            ),
            Cmd::MoveUp {
                expand_selection: true,
                distance: 1,
            },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(
                KeyCode::Char('j'),
                KeyModifiers::SHIFT | KeyModifiers::CONTROL,
            ),
            Cmd::MoveDown {
                expand_selection: true,
                distance: 1,
            },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Right, KeyModifiers::NONE),
            Cmd::MoveRight {
                expand_selection: false,
            },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Left, KeyModifiers::NONE),
            Cmd::MoveLeft {
                expand_selection: false,
            },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Right, KeyModifiers::SHIFT),
            Cmd::MoveRight {
                expand_selection: true,
            },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Left, KeyModifiers::SHIFT),
            Cmd::MoveLeft {
                expand_selection: true,
            },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Right, KeyModifiers::CONTROL),
            Cmd::MoveRightWord {
                expand_selection: false,
            },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Left, KeyModifiers::CONTROL),
            Cmd::MoveLeftWord {
                expand_selection: false,
            },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Right, KeyModifiers::SHIFT | KeyModifiers::CONTROL),
            Cmd::MoveRightWord {
                expand_selection: true,
            },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Left, KeyModifiers::SHIFT | KeyModifiers::CONTROL),
            Cmd::MoveLeftWord {
                expand_selection: true,
            },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Up, KeyModifiers::ALT),
            Cmd::MoveLine(LineMoveDir::Up),
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Down, KeyModifiers::ALT),
            Cmd::MoveLine(LineMoveDir::Down),
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('+'), KeyModifiers::ALT),
            Cmd::GrowPane,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('-'), KeyModifiers::ALT),
            Cmd::ShrinkPane,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('k'), KeyModifiers::CONTROL),
            Cmd::Choord,
            Exclusiveness::Exclusive,
        ),
    ]
}
