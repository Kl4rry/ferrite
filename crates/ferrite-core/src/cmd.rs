use core::fmt;
use std::path::PathBuf;

use ferrite_utility::{line_ending::LineEnding, point::Point};
use serde::{Deserialize, Serialize};

use crate::{buffer::case::Case, layout::panes::Direction};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LineMoveDir {
    Up,
    Down,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Cmd {
    Nop,
    OpenFile(PathBuf),
    Cd(PathBuf),
    Save(Option<PathBuf>),
    Language(Option<String>),
    Encoding(Option<String>),
    LineEnding(Option<LineEnding>),
    RunShellCmd {
        args: Vec<PathBuf>,
        pipe: bool,
    },
    OpenShellPalette,
    Case(Case),
    Split(Direction),
    ReplaceAll(String),
    Replace,
    Search,
    About,
    Path,
    Pwd,
    New(Option<PathBuf>),
    Reload,
    ReloadAll,
    Logger,
    ForceQuit,
    Quit,
    UrlOpen,
    Goto(i64),
    Indent(Option<String>),
    Theme(Option<String>),
    SortLines(bool),
    BufferPickerOpen,
    FilePickerOpen,
    FilePickerReload,
    OpenConfig,
    DefaultConfig,
    OpenLanguages,
    DefaultLanguages,
    OpenKeymap,
    DefaultKeymap,
    ForceClose,
    Close,
    ClosePane,
    Paste,
    Copy,
    Format,
    FormatSelection,
    GitReload,
    RevertBuffer,
    Trash,
    Repeat,
    MoveRight {
        expand_selection: bool,
    },
    MoveLeft {
        expand_selection: bool,
    },
    MoveUp {
        expand_selection: bool,
        create_cursor: bool,
        distance: usize,
    },
    MoveDown {
        expand_selection: bool,
        create_cursor: bool,
        distance: usize,
    },
    MoveRightWord {
        expand_selection: bool,
    },
    MoveLeftWord {
        expand_selection: bool,
    },
    Insert(String),
    Char(char),
    MoveLine(LineMoveDir),
    Backspace,
    BackspaceWord,
    Delete,
    DeleteWord,
    ClickCell(bool, usize, usize),
    SelectArea {
        cursor: Point<usize>,
        anchor: Point<usize>,
    },
    PromptGoto,
    Home {
        expand_selection: bool,
    },
    End {
        expand_selection: bool,
    },
    Eof {
        expand_selection: bool,
    },
    Start {
        expand_selection: bool,
    },
    SelectAll,
    SelectLine,
    SelectWord,
    RemoveLine,
    Cut,
    PastePrimary(usize, usize),
    Tab {
        back: bool,
    },
    Undo,
    Redo,
    VerticalScroll(i64),
    ReplaceCurrentMatch,
    GlobalSearch,
    CaseInsensitive,
    NextMatch,
    PrevMatch,
    FocusPalette,
    OpenFilePicker,
    OpenBufferPicker,
    Escape,
    SaveAll,
    GrowPane,
    ShrinkPane,
    InputMode {
        name: String,
    },
    ReopenBuffer,
    RotateFile,
    ForceRedraw,
    SwitchPane {
        direction: Direction,
    },
    Number(Option<i64>),
    OpenFileExplorer(Option<PathBuf>),
    TrimTrailingWhitespace,
    ZoomIn,
    ZoomOut,
    ResetZoom,
    KillJob,
    RunAction {
        name: String,
    },
}

impl Cmd {
    fn as_str(&self) -> &str {
        use Cmd::*;
        match self {
            Nop => "Nop",
            Repeat { .. } => "Repeat",
            MoveRight { .. } => "Move right",
            MoveLeft { .. } => "Move left",
            MoveUp { .. } => "Move up",
            MoveDown { .. } => "Move down",
            MoveRightWord { .. } => "Move right word",
            MoveLeftWord { .. } => "Move left word",
            Insert(s) => s.as_str(),
            Char(..) => "char",
            MoveLine(LineMoveDir::Up) => "Move line up",
            MoveLine(LineMoveDir::Down) => "Move line down",
            Backspace => "Backspace",
            BackspaceWord => "Backspace word",
            Delete => "Delete",
            DeleteWord => "Delete word",
            ClickCell(..) => "Set cursor pos",
            SelectArea { .. } => "Select area",
            PromptGoto => "Goto",
            Home { .. } => "Home",
            End { .. } => "End",
            Eof { .. } => "End of file",
            Start { .. } => "Start",
            SelectAll => "Select all",
            SelectLine => "Select line",
            RemoveLine => "Remove line",
            SelectWord => "Select word",
            Copy => "Cpy",
            Cut => "Cut",
            Paste => "Paste",
            PastePrimary(_, _) => "Paste primary",
            Tab { .. } => "Tab",
            Undo => "Undo",
            Redo => "Redo",
            RevertBuffer => "Revert buffer",
            VerticalScroll(_) => "Vertical scroll",
            Search => "Search file",
            Replace => "Replace",
            ReplaceCurrentMatch => "Replace current match",
            GlobalSearch => "Global workspace search",
            CaseInsensitive => "Case insensitive",
            NextMatch => "Next match",
            PrevMatch => "Prev match",
            FocusPalette => "Open palette",
            OpenFilePicker => "Open file picker",
            OpenBufferPicker => "Open buffer picker",
            Escape => "Escape",
            SaveAll => "SaveAll",
            Quit => "Quit",
            Close => "Close buffer",
            ClosePane => "Close pane",
            GrowPane => "Grow pane",
            ShrinkPane => "Shrink pane",
            InputMode { name } => name,
            Format => "Format",
            UrlOpen => "Open urls in selection",
            Split(Direction::Right) => "Split right",
            Split(Direction::Left) => "Split left",
            Split(Direction::Up) => "Split up",
            Split(Direction::Down) => "Split down",
            ReopenBuffer => "Reopen buffer",
            New(_) => "New",
            RotateFile => "Rotate file",
            OpenFile(_) => "Open file",
            Cd(_) => "Change project directory",
            Save(_) => "Save buffer",
            Language(_) => "Language",
            Encoding(_) => "Encoding",
            LineEnding(_) => "Line ending",
            RunShellCmd { .. } => "Run shell command",
            OpenShellPalette { .. } => "Open shell command palette",
            Case(_) => "Case",
            ReplaceAll(_) => "Replace all",
            About => "About",
            Path => "Show filepath",
            Pwd => "Print working directory",
            Reload => "Reload",
            ReloadAll => "Reload all buffers",
            Logger => "Logger",
            ForceQuit => "Force quit",
            Goto(_) => "Goto",
            Indent(_) => "Indent",
            Theme(_) => "Theme",
            SortLines(_) => "Sort lines",
            BufferPickerOpen => "Open buffer picker",
            FilePickerOpen => "Open file picker",
            FilePickerReload => "Reload file picker",
            OpenConfig => "Open editor config file",
            DefaultConfig => "Open default editor config",
            OpenLanguages => "Open languages config file",
            DefaultLanguages => "Open default languages config",
            OpenKeymap => "Open keymap config file",
            DefaultKeymap => "Open default keymap",
            ForceClose => "Force close buffer",
            FormatSelection => "Format selection",
            GitReload => "Git reload",
            Trash => "Move to trash",
            ForceRedraw => "Force redraw",
            SwitchPane { direction } => match direction {
                Direction::Up => "Up pane",
                Direction::Down => "Down pane",
                Direction::Right => "Right pane",
                Direction::Left => "Left pane",
            },
            Number(_) => "Number",
            OpenFileExplorer(_) => "Open file explorer",
            TrimTrailingWhitespace => "Trim trailing whitespace",
            ZoomIn => "Zoom in",
            ZoomOut => "Zoom out",
            ResetZoom => "Reset zoom",
            KillJob => "Kill job",
            RunAction { .. } => "Run",
        }
    }

    pub fn is_repeatable(&self) -> bool {
        use Cmd::*;
        match self {
            Nop => false,
            Repeat => false,
            MoveRight { .. } => true,
            MoveLeft { .. } => true,
            MoveUp { .. } => true,
            MoveDown { .. } => true,
            MoveRightWord { .. } => true,
            MoveLeftWord { .. } => true,
            Insert(..) => true,
            Char(..) => true,
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
            RemoveLine => true,
            Copy => false,
            Cut => false,
            Paste => true,
            PastePrimary(..) => true,
            Tab { .. } => true,
            Undo => true,
            Redo => true,
            RevertBuffer => false,
            VerticalScroll(..) => true,
            Search => false,
            Replace => false,
            ReplaceCurrentMatch => true,
            GlobalSearch => false,
            CaseInsensitive => false,
            NextMatch => true,
            PrevMatch => true,
            FocusPalette => false,
            OpenFilePicker => false,
            OpenBufferPicker => false,
            Escape => false,
            SaveAll => false,
            Quit => false,
            Close => false,
            ClosePane => false,
            GrowPane => true,
            ShrinkPane => true,
            InputMode { .. } => false,
            Format => false,
            RunShellCmd { .. } => false,
            OpenShellPalette { .. } => false,
            Split { .. } => false,
            ReopenBuffer => false,
            RotateFile => false,
            OpenFile(_) => false,
            Cd(_) => false,
            Save(_) => false,
            Language(_) => false,
            Encoding(_) => false,
            LineEnding(_) => false,
            Case(_) => false,
            ReplaceAll(_) => false,
            About => false,
            Path => false,
            Pwd => false,
            New(_) => false,
            Reload => false,
            ReloadAll => false,
            Logger => false,
            ForceQuit => false,
            UrlOpen => false,
            Goto(_) => false,
            Indent(_) => false,
            Theme(_) => false,
            SortLines(_) => false,
            BufferPickerOpen => false,
            FilePickerOpen => false,
            FilePickerReload => false,
            OpenConfig => false,
            DefaultConfig => false,
            OpenLanguages => false,
            DefaultLanguages => false,
            OpenKeymap => false,
            DefaultKeymap => false,
            ForceClose => false,
            FormatSelection => false,
            GitReload => false,
            Trash => false,
            ForceRedraw => false,
            SwitchPane { .. } => false,
            Number(_) => false,
            OpenFileExplorer(_) => false,
            TrimTrailingWhitespace => false,
            ZoomIn => false,
            ZoomOut => false,
            ResetZoom => false,
            KillJob => false,
            RunAction { .. } => true,
        }
    }
}

impl fmt::Display for Cmd {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_str().fmt(f)
    }
}
