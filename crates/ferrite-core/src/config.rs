use std::{path::PathBuf, sync::Arc};

use editor::Editor;
use keymap::Keymap;
use languages::Languages;

use crate::watcher::{FileWatcher, TomlConfig};

pub mod editor;
pub mod keymap;
pub mod languages;

pub struct Config {
    pub editor: Arc<Editor>,
    pub editor_path: Option<PathBuf>,
    pub editor_watcher: Option<FileWatcher<Editor, TomlConfig>>,
    pub languages: Languages,
    pub languages_path: Option<PathBuf>,
    pub languages_watcher: Option<FileWatcher<Languages, TomlConfig>>,
    pub keymap: Keymap,
}
