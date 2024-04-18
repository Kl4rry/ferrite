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
    Shell(Vec<PathBuf>),
    Case(Case),
    Split(Direction),
    New,
    Reload,
    Logger,
    ForceQuit,
    Quit,
    Goto(i64),
    Indent(Option<String>),
    Theme(Option<String>),
    BrowseBuffers,
    BrowseWorkspace,
    OpenConfig,
    ForceClose,
    Close,
    Paste,
    Copy,
    Format,
    FormatSelection,
    GitReload,
    RevertBuffer,
    Delete,
}
