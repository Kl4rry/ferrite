use std::path::PathBuf;

use ferrite_utility::line_ending::LineEnding;

use crate::{buffer::case::Case, panes::Direction};

#[derive(Debug, Clone)]
pub enum Command {
    OpenFile(PathBuf),
    Cd(PathBuf),
    SaveFile(Option<PathBuf>),
    Language(Option<String>),
    Encoding(Option<String>),
    LineEnding(Option<LineEnding>),
    Shell { args: Vec<PathBuf>, pipe: bool },
    Case(Case),
    Split(Direction),
    ReplaceAll(String),
    About,
    Path,
    Pwd,
    New(Option<PathBuf>),
    Reload,
    ReloadAll,
    Logger,
    ForceQuit,
    Quit,
    Url,
    Goto(i64),
    Indent(Option<String>),
    Theme(Option<String>),
    SortLines(bool),
    BufferPickerOpen,
    FilePickerOpen,
    FilePickerReload,
    OpenConfig,
    DefaultConfig,
    ForceClose,
    Close,
    Paste,
    Copy,
    Format,
    FormatSelection,
    GitReload,
    GitDiff,
    RevertBuffer,
    Delete,
}
