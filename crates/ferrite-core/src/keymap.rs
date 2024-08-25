use core::fmt;

use ferrite_utility::point::Point;

pub mod keycode;
use keycode::{KeyCode, KeyModifiers};

use crate::panes::Direction;

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
pub enum LineMoveDir {
    Up,
    Down,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputCommand {
    Repeat,
    OpenUrl,
    MoveRight {
        shift: bool,
    },
    MoveLeft {
        shift: bool,
    },
    MoveUp {
        shift: bool,
        distance: usize,
    },
    MoveDown {
        shift: bool,
        distance: usize,
    },
    MoveRightWord {
        shift: bool,
    },
    MoveLeftWord {
        shift: bool,
    },
    Insert(String),
    Char(char),
    NewLine,
    MoveLine(LineMoveDir),
    Backspace,
    BackspaceWord,
    Delete,
    DeleteWord,
    ClickCell(usize, usize),
    SelectArea {
        cursor: Point<usize>,
        anchor: Point<usize>,
    },
    PromptGoto,
    Home {
        shift: bool,
    },
    End {
        shift: bool,
    },
    Eof {
        shift: bool,
    },
    Start {
        shift: bool,
    },
    SelectAll,
    SelectLine,
    SelectWord,
    Copy,
    Cut,
    Paste,
    PastePrimary(usize, usize),
    Tab {
        back: bool,
    },
    Undo,
    Redo,
    RevertBuffer,
    VerticalScroll(i64),
    FileSearch,
    CaseInsensitive,
    NextMatch,
    PrevMatch,
    FocusPalette,
    OpenFileBrowser,
    OpenBufferBrowser,
    Escape,
    Save,
    Quit,
    Close,
    GrowPane,
    ShrinkPane,
    Choord,
    Format,
    Shell,
    Split {
        direction: Direction,
    },
    ReopenBuffer,
    New,
    RotateFile,
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
    mappings: &[(Mapping, InputCommand, Exclusiveness)],
) -> Option<InputCommand> {
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
            return Some(InputCommand::Char(ch));
        }
    }

    None
}

pub fn get_default_choords() -> Vec<(Mapping, InputCommand, Exclusiveness)> {
    vec![
        (
            Mapping::new(KeyCode::Esc, KeyModifiers::NONE),
            InputCommand::Escape,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('k'), KeyModifiers::CONTROL),
            InputCommand::Choord,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('f'), KeyModifiers::CONTROL),
            InputCommand::Format,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('s'), KeyModifiers::CONTROL),
            InputCommand::Shell,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('o'), KeyModifiers::CONTROL),
            InputCommand::OpenUrl,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('r'), KeyModifiers::CONTROL),
            InputCommand::Split {
                direction: Direction::Right,
            },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('l'), KeyModifiers::CONTROL),
            InputCommand::Split {
                direction: Direction::Left,
            },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('u'), KeyModifiers::CONTROL),
            InputCommand::Split {
                direction: Direction::Up,
            },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('d'), KeyModifiers::CONTROL),
            InputCommand::Split {
                direction: Direction::Down,
            },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('j'), KeyModifiers::CONTROL),
            InputCommand::RotateFile,
            Exclusiveness::Exclusive,
        ),
    ]
}

pub fn get_default_mappings() -> Vec<(Mapping, InputCommand, Exclusiveness)> {
    vec![
        (
            Mapping::new(KeyCode::Esc, KeyModifiers::NONE),
            InputCommand::Escape,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(
                KeyCode::Char('t'),
                KeyModifiers::CONTROL | KeyModifiers::SHIFT,
            ),
            InputCommand::ReopenBuffer,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('r'), KeyModifiers::CONTROL),
            InputCommand::Repeat,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('w'), KeyModifiers::CONTROL),
            InputCommand::Close,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('n'), KeyModifiers::CONTROL),
            InputCommand::New,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('q'), KeyModifiers::CONTROL),
            InputCommand::Quit,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('s'), KeyModifiers::CONTROL),
            InputCommand::Save,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('a'), KeyModifiers::CONTROL),
            InputCommand::SelectAll,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('l'), KeyModifiers::CONTROL),
            InputCommand::SelectLine,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('d'), KeyModifiers::CONTROL),
            InputCommand::SelectWord,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
            InputCommand::Copy,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('v'), KeyModifiers::CONTROL),
            InputCommand::Paste,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('x'), KeyModifiers::CONTROL),
            InputCommand::Cut,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('p'), KeyModifiers::CONTROL),
            InputCommand::FocusPalette,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('g'), KeyModifiers::CONTROL),
            InputCommand::PromptGoto,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('o'), KeyModifiers::CONTROL),
            InputCommand::OpenFileBrowser,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('b'), KeyModifiers::CONTROL),
            InputCommand::OpenBufferBrowser,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('z'), KeyModifiers::CONTROL),
            InputCommand::Undo,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('y'), KeyModifiers::CONTROL),
            InputCommand::Redo,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('f'), KeyModifiers::CONTROL),
            InputCommand::FileSearch,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('i'), KeyModifiers::ALT),
            InputCommand::CaseInsensitive,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('p'), KeyModifiers::ALT),
            InputCommand::PrevMatch,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('n'), KeyModifiers::ALT),
            InputCommand::NextMatch,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Tab, KeyModifiers::NONE),
            InputCommand::Tab { back: false },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::BackTab, KeyModifiers::SHIFT),
            InputCommand::Tab { back: true },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Enter, KeyModifiers::CONTROL),
            InputCommand::NewLine,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Enter, KeyModifiers::NONE),
            InputCommand::Char('\n'),
            Exclusiveness::Ignores(KeyModifiers::SHIFT | KeyModifiers::SUPER | KeyModifiers::ALT),
        ),
        (
            Mapping::new(KeyCode::Backspace, KeyModifiers::NONE),
            InputCommand::Backspace,
            Exclusiveness::Ignores(KeyModifiers::SHIFT | KeyModifiers::SUPER | KeyModifiers::ALT),
        ),
        (
            Mapping::new(KeyCode::Delete, KeyModifiers::NONE),
            InputCommand::Delete,
            Exclusiveness::Ignores(KeyModifiers::SHIFT | KeyModifiers::SUPER | KeyModifiers::ALT),
        ),
        (
            Mapping::new(KeyCode::Backspace, KeyModifiers::CONTROL),
            InputCommand::BackspaceWord,
            Exclusiveness::Ignores(KeyModifiers::SHIFT | KeyModifiers::SUPER | KeyModifiers::ALT),
        ),
        (
            Mapping::new(KeyCode::Delete, KeyModifiers::CONTROL),
            InputCommand::DeleteWord,
            Exclusiveness::Ignores(KeyModifiers::SHIFT | KeyModifiers::SUPER | KeyModifiers::ALT),
        ),
        (
            Mapping::new(KeyCode::Home, KeyModifiers::NONE),
            InputCommand::Home { shift: false },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::End, KeyModifiers::NONE),
            InputCommand::End { shift: false },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Home, KeyModifiers::SHIFT),
            InputCommand::Home { shift: true },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::End, KeyModifiers::SHIFT),
            InputCommand::End { shift: true },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Home, KeyModifiers::CONTROL),
            InputCommand::Start { shift: false },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::End, KeyModifiers::CONTROL),
            InputCommand::Eof { shift: false },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Home, KeyModifiers::SHIFT | KeyModifiers::CONTROL),
            InputCommand::Start { shift: true },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::End, KeyModifiers::SHIFT | KeyModifiers::CONTROL),
            InputCommand::Eof { shift: true },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::PageUp, KeyModifiers::NONE),
            InputCommand::VerticalScroll(-50),
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::PageDown, KeyModifiers::NONE),
            InputCommand::VerticalScroll(50),
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Up, KeyModifiers::CONTROL),
            InputCommand::MoveUp {
                shift: false,
                distance: 10,
            },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Down, KeyModifiers::CONTROL),
            InputCommand::MoveDown {
                shift: false,
                distance: 10,
            },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Up, KeyModifiers::CONTROL | KeyModifiers::SHIFT),
            InputCommand::MoveUp {
                shift: true,
                distance: 10,
            },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Down, KeyModifiers::CONTROL | KeyModifiers::SHIFT),
            InputCommand::MoveDown {
                shift: true,
                distance: 10,
            },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Up, KeyModifiers::CONTROL),
            InputCommand::MoveUp {
                shift: false,
                distance: 10,
            },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Down, KeyModifiers::CONTROL),
            InputCommand::MoveDown {
                shift: false,
                distance: 10,
            },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Up, KeyModifiers::CONTROL | KeyModifiers::SHIFT),
            InputCommand::MoveUp {
                shift: true,
                distance: 10,
            },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Down, KeyModifiers::CONTROL | KeyModifiers::SHIFT),
            InputCommand::MoveDown {
                shift: true,
                distance: 10,
            },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Up, KeyModifiers::NONE),
            InputCommand::MoveUp {
                shift: false,
                distance: 1,
            },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Down, KeyModifiers::NONE),
            InputCommand::MoveDown {
                shift: false,
                distance: 1,
            },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Up, KeyModifiers::SHIFT),
            InputCommand::MoveUp {
                shift: true,
                distance: 1,
            },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Down, KeyModifiers::SHIFT),
            InputCommand::MoveDown {
                shift: true,
                distance: 1,
            },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('i'), KeyModifiers::CONTROL),
            InputCommand::MoveUp {
                shift: false,
                distance: 1,
            },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('j'), KeyModifiers::CONTROL),
            InputCommand::MoveDown {
                shift: false,
                distance: 1,
            },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(
                KeyCode::Char('i'),
                KeyModifiers::SHIFT | KeyModifiers::CONTROL,
            ),
            InputCommand::MoveUp {
                shift: true,
                distance: 1,
            },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(
                KeyCode::Char('j'),
                KeyModifiers::SHIFT | KeyModifiers::CONTROL,
            ),
            InputCommand::MoveDown {
                shift: true,
                distance: 1,
            },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Right, KeyModifiers::NONE),
            InputCommand::MoveRight { shift: false },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Left, KeyModifiers::NONE),
            InputCommand::MoveLeft { shift: false },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Right, KeyModifiers::SHIFT),
            InputCommand::MoveRight { shift: true },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Left, KeyModifiers::SHIFT),
            InputCommand::MoveLeft { shift: true },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Right, KeyModifiers::CONTROL),
            InputCommand::MoveRightWord { shift: false },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Left, KeyModifiers::CONTROL),
            InputCommand::MoveLeftWord { shift: false },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Right, KeyModifiers::SHIFT | KeyModifiers::CONTROL),
            InputCommand::MoveRightWord { shift: true },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Left, KeyModifiers::SHIFT | KeyModifiers::CONTROL),
            InputCommand::MoveLeftWord { shift: true },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Up, KeyModifiers::ALT),
            InputCommand::MoveLine(LineMoveDir::Up),
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Down, KeyModifiers::ALT),
            InputCommand::MoveLine(LineMoveDir::Down),
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('+'), KeyModifiers::ALT),
            InputCommand::GrowPane,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('-'), KeyModifiers::ALT),
            InputCommand::ShrinkPane,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('k'), KeyModifiers::CONTROL),
            InputCommand::Choord,
            Exclusiveness::Exclusive,
        ),
    ]
}

impl InputCommand {
    fn as_str(&self) -> &str {
        match self {
            InputCommand::Repeat { .. } => "repeat",
            InputCommand::MoveRight { .. } => "move right",
            InputCommand::MoveLeft { .. } => "move left",
            InputCommand::MoveUp { .. } => "move up",
            InputCommand::MoveDown { .. } => "move down",
            InputCommand::MoveRightWord { .. } => "move right word",
            InputCommand::MoveLeftWord { .. } => "move left word",
            InputCommand::Insert(s) => s.as_str(),
            InputCommand::Char(..) => "char",
            InputCommand::NewLine => "newline",
            InputCommand::MoveLine(LineMoveDir::Up) => "move line up",
            InputCommand::MoveLine(LineMoveDir::Down) => "move line down",
            InputCommand::Backspace => "backspace",
            InputCommand::BackspaceWord => "backspace word",
            InputCommand::Delete => "delete",
            InputCommand::DeleteWord => "delete word",
            InputCommand::ClickCell(_, _) => "set cursor pos",
            InputCommand::SelectArea { .. } => "select area",
            InputCommand::PromptGoto => "goto",
            InputCommand::Home { .. } => "home",
            InputCommand::End { .. } => "end",
            InputCommand::Eof { .. } => "end of file",
            InputCommand::Start { .. } => "start",
            InputCommand::SelectAll => "select all",
            InputCommand::SelectLine => "select line",
            InputCommand::SelectWord => "select word",
            InputCommand::Copy => "copy",
            InputCommand::Cut => "cut",
            InputCommand::Paste => "paste",
            InputCommand::PastePrimary(_, _) => "paste primary",
            InputCommand::Tab { .. } => "tab",
            InputCommand::Undo => "undo",
            InputCommand::Redo => "redo",
            InputCommand::RevertBuffer => "revert buffer",
            InputCommand::VerticalScroll(_) => "vertical scroll",
            InputCommand::FileSearch => "search file",
            InputCommand::CaseInsensitive => "case insensitive",
            InputCommand::NextMatch => "next match",
            InputCommand::PrevMatch => "prev match",
            InputCommand::FocusPalette => "open palette",
            InputCommand::OpenFileBrowser => "file browser",
            InputCommand::OpenBufferBrowser => "buffer browser",
            InputCommand::Escape => "escape",
            InputCommand::Save => "save",
            InputCommand::Quit => "quit",
            InputCommand::Close => "close",
            InputCommand::GrowPane => "grow pane",
            InputCommand::ShrinkPane => "shrink pane",
            InputCommand::Choord => "choord",
            InputCommand::Format => "Format",
            InputCommand::Shell => "Run shell command",
            InputCommand::OpenUrl => "Open urls in selection",
            InputCommand::Split {
                direction: Direction::Right,
            } => "Split right",
            InputCommand::Split {
                direction: Direction::Left,
            } => "Split left",
            InputCommand::Split {
                direction: Direction::Up,
            } => "Split up",
            InputCommand::Split {
                direction: Direction::Down,
            } => "Split down",
            InputCommand::ReopenBuffer => "Reopen buffer",
            InputCommand::New => "New",
            InputCommand::RotateFile => "Rotate file",
        }
    }

    pub fn is_repeatable(&self) -> bool {
        use InputCommand::*;
        match self {
            Repeat => false,
            OpenUrl => false,
            MoveRight { .. } => true,
            MoveLeft { .. } => true,
            MoveUp { .. } => true,
            MoveDown { .. } => true,
            MoveRightWord { .. } => true,
            MoveLeftWord { .. } => true,
            Insert(..) => true,
            Char(..) => true,
            NewLine => true,
            MoveLine(..) => true,
            Backspace => true,
            BackspaceWord => true,
            Delete => true,
            DeleteWord => true,
            ClickCell(..) => false,
            SelectArea { .. } => false,
            PromptGoto => false,
            Home { .. } => true,
            End { .. } => true,
            Eof { .. } => false,
            Start { .. } => false,
            SelectAll => false,
            SelectLine => true,
            SelectWord => true,
            Copy => false,
            Cut => false,
            Paste => true,
            PastePrimary(..) => true,
            Tab { .. } => true,
            Undo => true,
            Redo => true,
            RevertBuffer => false,
            VerticalScroll(..) => true,
            FileSearch => false,
            CaseInsensitive => false,
            NextMatch => true,
            PrevMatch => true,
            FocusPalette => false,
            OpenFileBrowser => false,
            OpenBufferBrowser => false,
            Escape => false,
            Save => false,
            Quit => false,
            Close => false,
            GrowPane => true,
            ShrinkPane => true,
            Choord => false,
            Format => false,
            Shell => false,
            Split { .. } => false,
            ReopenBuffer => false,
            New => true,
            RotateFile => false,
        }
    }
}

impl fmt::Display for InputCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_str().fmt(f)
    }
}
