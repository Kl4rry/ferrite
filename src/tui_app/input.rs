use crossterm::event::{KeyCode, KeyModifiers};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mapping {
    keycode: KeyCode,
    modifiers: KeyModifiers,
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

#[derive(Debug, Clone)]
pub enum InputCommand {
    MoveRight { shift: bool },
    MoveLeft { shift: bool },
    MoveUp { shift: bool },
    MoveDown { shift: bool },
    MoveRightWord { shift: bool },
    MoveLeftWord { shift: bool },
    Insert(String),
    Char(char),
    MoveLine(LineMoveDir),
    Backspace,
    BackspaceWord,
    Delete,
    DeleteWord,
    PromptGoto,
    Home { shift: bool },
    End { shift: bool },
    Eof { shift: bool },
    Start { shift: bool },
    SelectAll,
    SelectLine,
    SelectWord,
    Copy,
    Cut,
    Paste,
    Tab { back: bool },
    Scroll(i64),
    FocusPalette,
    FindFile,
    FindBuffer,
    Escape,
    Save,
    Quit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Exclusiveness {
    Exclusive,
    NonExclusive,
    Ignores(KeyModifiers),
}

pub fn get_command_from_input(
    keycode: KeyCode,
    modifiers: KeyModifiers,
    mappings: &[(Mapping, InputCommand, Exclusiveness)],
) -> Option<InputCommand> {
    for (mapping, cmd, exclusiveness) in mappings {
        match exclusiveness {
            Exclusiveness::Exclusive => {
                if *mapping == (Mapping { keycode, modifiers }) {
                    return Some(cmd.clone());
                }
            }
            Exclusiveness::NonExclusive => {
                if mapping.keycode == keycode && modifiers.contains(mapping.modifiers) {
                    return Some(cmd.clone());
                }
            }
            Exclusiveness::Ignores(ignored) => {
                let mut non_ignored = modifiers;
                non_ignored.remove(*ignored);
                if mapping.keycode == keycode && non_ignored == mapping.modifiers {
                    return Some(cmd.clone());
                }
            }
        }
    }

    if let KeyCode::Char(ch) = keycode {
        return Some(InputCommand::Char(ch));
    }

    None
}

pub fn get_default_mappings() -> Vec<(Mapping, InputCommand, Exclusiveness)> {
    vec![
        (
            Mapping::new(KeyCode::Esc, KeyModifiers::NONE),
            InputCommand::Escape,
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
            InputCommand::FindFile,
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Char('b'), KeyModifiers::CONTROL),
            InputCommand::FindBuffer,
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
            Mapping::new(KeyCode::Enter, KeyModifiers::NONE),
            InputCommand::Char('\n'),
            Exclusiveness::NonExclusive,
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
            InputCommand::Scroll(-50),
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::PageDown, KeyModifiers::NONE),
            InputCommand::Scroll(50),
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Up, KeyModifiers::CONTROL),
            InputCommand::Scroll(-5),
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Down, KeyModifiers::CONTROL),
            InputCommand::Scroll(5),
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
            Mapping::new(KeyCode::Up, KeyModifiers::NONE),
            InputCommand::MoveUp { shift: false },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Down, KeyModifiers::NONE),
            InputCommand::MoveDown { shift: false },
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
            Mapping::new(KeyCode::Up, KeyModifiers::SHIFT),
            InputCommand::MoveUp { shift: true },
            Exclusiveness::Exclusive,
        ),
        (
            Mapping::new(KeyCode::Down, KeyModifiers::SHIFT),
            InputCommand::MoveDown { shift: true },
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
    ]
}
