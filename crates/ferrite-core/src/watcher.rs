use std::{
    fs,
    path::{Path, PathBuf},
    sync::mpsc,
};

use anyhow::Result;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use serde::Deserialize;

use crate::event_loop_proxy::EventLoopProxy;

pub trait ConfigType<T> {
    fn from_file(path: impl AsRef<Path>) -> Result<T>;
}

pub struct TomlConfig;

impl<T> ConfigType<T> for TomlConfig
where
    T: for<'a> Deserialize<'a>,
{
    fn from_file(path: impl AsRef<Path>) -> Result<T> {
        Ok(toml::from_str(&fs::read_to_string(path)?)?)
    }
}

pub struct JsonConfig;

impl<T> ConfigType<T> for JsonConfig
where
    T: for<'a> Deserialize<'a>,
{
    fn from_file(path: impl AsRef<Path>) -> Result<T> {
        Ok(serde_json::from_str(&fs::read_to_string(path)?)?)
    }
}

pub struct FileWatcher<T, C> {
    _watcher: RecommendedWatcher,
    rx: mpsc::Receiver<Result<T>>,
    _phantom: std::marker::PhantomData<C>,
}

impl<T, C> FileWatcher<T, C>
where
    T: 'static + for<'a> Deserialize<'a> + Send,
    C: ConfigType<T>,
{
    pub fn new(path: impl AsRef<Path>, proxy: Box<dyn EventLoopProxy>) -> Result<Self> {
        let path = path.as_ref();
        let (tx, rx) = mpsc::channel();

        let path_buf: PathBuf = path.to_path_buf();
        let mut watcher = notify::recommended_watcher(
            move |event: std::result::Result<notify::event::Event, notify::Error>| {
                if let Ok(event) = event {
                    match event.kind {
                        notify::EventKind::Create(_) | notify::EventKind::Modify(_) => {
                            let data: Result<T> = C::from_file(&path_buf);
                            let _ = tx.send(data);
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
            rx,
            _phantom: std::marker::PhantomData,
        })
    }

    pub fn poll_update(&mut self) -> Option<Result<T>> {
        self.rx.try_recv().ok()
    }
}
