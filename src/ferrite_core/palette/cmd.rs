use std::path::PathBuf;

use utility::line_ending::LineEnding;

#[derive(Debug, Clone)]
pub enum Command {
    OpenFile(PathBuf),
    SaveFile(Option<PathBuf>),
    Language(Option<String>),
    Encoding(Option<String>),
    LineEnding(Option<LineEnding>),
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
    GitReload,
    RevertBuffer,
}
