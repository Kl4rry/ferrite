use std::path::PathBuf;

use utility::line_ending::LineEnding;

use crate::{ferrite_core::buffer::case::Case, tui_app::panes::Direction};

#[derive(Debug, Clone)]
pub enum Command {
    OpenFile(PathBuf),
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
