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
    Insert(String),
    Char(char),
    MoveLine(LineMoveDir),
    Backspace,
    Delete,
    PromptGoto,
    Home { shift: bool },
    End { shift: bool },
    Eof { shift: bool },
    Start { shift: bool },
    SelectAll,
    SelectLine,
    Copy,
    Tab { back: bool },
    Scroll(i64),
    FocusPalette,
    Escape,
    Save,
    Quit,
}

pub fn get_command_from_input(
    keycode: KeyCode,
    modifiers: KeyModifiers,
    mappings: &[(Mapping, InputCommand)],
) -> Option<InputCommand> {
    for (mapping, cmd) in mappings {
        if *mapping == (Mapping { keycode, modifiers }) {
            return Some(cmd.clone());
        }
    }

    None
}

pub fn get_default_mappings() -> Vec<(Mapping, InputCommand)> {
    vec![
        (
            Mapping::new(KeyCode::Esc, KeyModifiers::NONE),
            InputCommand::Escape,
        ),
        (
            Mapping::new(KeyCode::Char('q'), KeyModifiers::CONTROL),
            InputCommand::Quit,
        ),
        (
            Mapping::new(KeyCode::Char('s'), KeyModifiers::CONTROL),
            InputCommand::Save,
        ),
        (
            Mapping::new(KeyCode::Char('a'), KeyModifiers::CONTROL),
            InputCommand::SelectAll,
        ),
        (
            Mapping::new(KeyCode::Char('l'), KeyModifiers::CONTROL),
            InputCommand::SelectLine,
        ),
        (
            Mapping::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
            InputCommand::Copy,
        ),
        (
            Mapping::new(KeyCode::Char('p'), KeyModifiers::CONTROL),
            InputCommand::FocusPalette,
        ),
        (
            Mapping::new(KeyCode::Char('g'), KeyModifiers::CONTROL),
            InputCommand::PromptGoto,
        ),
        (
            Mapping::new(KeyCode::Tab, KeyModifiers::NONE),
            InputCommand::Tab { back: false },
        ),
        (
            Mapping::new(KeyCode::BackTab, KeyModifiers::SHIFT),
            InputCommand::Tab { back: true },
        ),
        (
            Mapping::new(KeyCode::Enter, KeyModifiers::NONE),
            InputCommand::Char('\n'),
        ),
        (
            Mapping::new(KeyCode::Backspace, KeyModifiers::NONE),
            InputCommand::Backspace,
        ),
        (
            Mapping::new(KeyCode::Delete, KeyModifiers::NONE),
            InputCommand::Delete,
        ),
        (
            Mapping::new(KeyCode::Home, KeyModifiers::NONE),
            InputCommand::Home { shift: false },
        ),
        (
            Mapping::new(KeyCode::End, KeyModifiers::NONE),
            InputCommand::End { shift: false },
        ),
        (
            Mapping::new(KeyCode::Home, KeyModifiers::SHIFT),
            InputCommand::Home { shift: true },
        ),
        (
            Mapping::new(KeyCode::End, KeyModifiers::SHIFT),
            InputCommand::End { shift: true },
        ),
        (
            Mapping::new(KeyCode::Home, KeyModifiers::CONTROL),
            InputCommand::Start { shift: false },
        ),
        (
            Mapping::new(KeyCode::End, KeyModifiers::CONTROL),
            InputCommand::Eof { shift: false },
        ),
        (
            Mapping::new(KeyCode::Home, KeyModifiers::SHIFT | KeyModifiers::CONTROL),
            InputCommand::Start { shift: true },
        ),
        (
            Mapping::new(KeyCode::End, KeyModifiers::SHIFT | KeyModifiers::CONTROL),
            InputCommand::Eof { shift: true },
        ),
        (
            Mapping::new(KeyCode::PageUp, KeyModifiers::NONE),
            InputCommand::Scroll(-50),
        ),
        (
            Mapping::new(KeyCode::PageDown, KeyModifiers::NONE),
            InputCommand::Scroll(50),
        ),
        (
            Mapping::new(KeyCode::Right, KeyModifiers::NONE),
            InputCommand::MoveRight { shift: false },
        ),
        (
            Mapping::new(KeyCode::Left, KeyModifiers::NONE),
            InputCommand::MoveLeft { shift: false },
        ),
        (
            Mapping::new(KeyCode::Up, KeyModifiers::NONE),
            InputCommand::MoveUp { shift: false },
        ),
        (
            Mapping::new(KeyCode::Down, KeyModifiers::NONE),
            InputCommand::MoveDown { shift: false },
        ),
        (
            Mapping::new(KeyCode::Right, KeyModifiers::SHIFT),
            InputCommand::MoveRight { shift: true },
        ),
        (
            Mapping::new(KeyCode::Left, KeyModifiers::SHIFT),
            InputCommand::MoveLeft { shift: true },
        ),
        (
            Mapping::new(KeyCode::Up, KeyModifiers::SHIFT),
            InputCommand::MoveUp { shift: true },
        ),
        (
            Mapping::new(KeyCode::Down, KeyModifiers::SHIFT),
            InputCommand::MoveDown { shift: true },
        ),
        (
            Mapping::new(KeyCode::Up, KeyModifiers::ALT),
            InputCommand::MoveLine(LineMoveDir::Up),
        ),
        (
            Mapping::new(KeyCode::Down, KeyModifiers::ALT),
            InputCommand::MoveLine(LineMoveDir::Down),
        ),
    ]
}
