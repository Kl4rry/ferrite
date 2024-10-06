use std::path::PathBuf;

use editor::Editor;
use keymap::Keymap;
use languages::Languages;

use crate::watcher::{FileWatcher, JsonConfig, TomlConfig};

pub mod editor;
pub mod keymap;
pub mod languages;

pub struct Config {
    pub editor: Editor,
    pub editor_path: Option<PathBuf>,
    pub editor_watcher: Option<FileWatcher<Editor, TomlConfig>>,
    pub languages: Languages,
    pub languages_path: Option<PathBuf>,
    pub languages_watcher: Option<FileWatcher<Languages, TomlConfig>>,
    pub keymap: Keymap,
    pub keymap_path: Option<PathBuf>,
    pub keymap_watcher: Option<FileWatcher<Keymap, JsonConfig>>,
}
