use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum Command {
    OpenFile(PathBuf),
    SaveFile(Option<PathBuf>),
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
