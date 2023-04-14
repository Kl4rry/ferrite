use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum Command {
    OpenFile(PathBuf),
    SaveFile(Option<PathBuf>),
    Language(Option<String>),
    Encoding(Option<String>),
    Reload,
    Logger,
    ForceQuit,
    Quit,
    Goto(i64),
    Indent,
    Theme(Option<String>),
    BrowseBuffers,
    BrowseWorkspace,
    OpenConfig,
    ForceClose,
    Close,
}
