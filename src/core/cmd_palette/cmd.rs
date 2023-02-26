use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum Command {
    OpenFile(PathBuf),
    SaveFile(Option<PathBuf>),
    Reload,
    Logger,
    Goto(i64),
    /*
    ViewBuffers,
    ViewWorkspace,
     */
}
