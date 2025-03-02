use std::{
    fs,
    path::{Path, PathBuf},
    sync::mpsc,
    time::Duration,
};

use anyhow::Result;
use notify_debouncer_full::{
    DebounceEventResult, Debouncer, RecommendedCache, new_debouncer,
    notify::{self, RecommendedWatcher, RecursiveMode},
};
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
    _watcher: Debouncer<RecommendedWatcher, RecommendedCache>,
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
        let mut debouncer = new_debouncer(
            Duration::from_millis(250),
            None,
            move |result: DebounceEventResult| {
                if let Ok(events) = result {
                    for event in events {
                        match event.kind {
                            notify::EventKind::Create(_) | notify::EventKind::Modify(_) => {
                                let data: Result<T> = C::from_file(&path_buf);
                                let _ = tx.send(data);
                                proxy.request_render();
                            }
                            _ => (),
                        }
                    }
                }
            },
        )?;
        let _ = debouncer.watch(path, RecursiveMode::NonRecursive);

        Ok(Self {
            _watcher: debouncer,
            rx,
            _phantom: std::marker::PhantomData,
        })
    }

    pub fn poll_update(&mut self) -> Option<Result<T>> {
        self.rx.try_recv().ok()
    }
}
