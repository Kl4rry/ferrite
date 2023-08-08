use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use anyhow::Result;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};

use crate::tui_app::event_loop::TuiEventLoopProxy;

pub fn default_theme() -> String {
    "default".into()
}

pub fn default_rulers() -> Vec<u16> {
    vec![80]
}

pub fn get_false() -> bool {
    false
}

pub fn get_true() -> bool {
    false
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_rulers")]
    pub rulers: Vec<u16>,
    #[serde(default = "get_false")]
    pub local_clipboard: bool,
    #[serde(default = "get_true")]
    pub show_splash: bool,
    #[serde(default = "get_true")]
    pub watch_recursive: bool,
}

const DEFAULT_CONFIG: &str = include_str!("../../config/default.toml");

impl Config {
    pub fn load_or_create_default() -> Result<Self> {
        let path = Self::get_default_location()?;

        let mut config_folder = path.clone();
        config_folder.pop();

        if !config_folder.exists() {
            fs::create_dir_all(config_folder)?;
        }

        match fs::read_to_string(&path) {
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                if let Err(err) = fs::write(path, DEFAULT_CONFIG) {
                    log::error!("Error creating default config: {err}");
                }
                Ok(Config::default())
            }
            err => Ok(toml::from_str(&err?)?),
        }
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        Ok(toml::from_str(&fs::read_to_string(path)?)?)
    }

    pub fn get_default_location() -> Result<PathBuf> {
        let Some(directories) = directories::ProjectDirs::from("", "", "ferrite") else {
            return Err(anyhow::Error::msg("Unable to find project directory"));
        };
        Ok(directories.config_dir().join("config.toml"))
    }
}

impl Default for Config {
    fn default() -> Self {
        toml::from_str(DEFAULT_CONFIG).unwrap()
    }
}

pub struct ConfigWatcher {
    changed: Arc<AtomicBool>,
    _watcher: RecommendedWatcher,
}

impl ConfigWatcher {
    pub fn watch(path: impl AsRef<Path>, proxy: TuiEventLoopProxy) -> Result<Self> {
        let path = path.as_ref();

        let changed = Arc::new(AtomicBool::new(false));
        let watcher_changed = changed.clone();

        let mut watcher = notify::recommended_watcher(
            move |event: std::result::Result<notify::event::Event, notify::Error>| {
                if let Ok(event) = event {
                    match event.kind {
                        notify::EventKind::Create(_) | notify::EventKind::Modify(_) => {
                            watcher_changed.store(true, Ordering::SeqCst);
                            proxy.request_render();
                        }
                        _ => (),
                    }
                }
            },
        )?;

        let _ = watcher.watch(path, RecursiveMode::NonRecursive);

        Ok(Self {
            _watcher: watcher,
            changed,
        })
    }

    pub fn has_changed(&self) -> bool {
        self.changed.swap(false, Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_config() {
        let _ = Config::default();
    }
}
