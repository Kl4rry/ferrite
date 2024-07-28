use std::{path::{Path, PathBuf}, sync::mpsc};

use anyhow::Result;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};

use crate::event_loop_proxy::EventLoopProxy;

pub trait FromTomlFile {
    fn from_toml_file(path: impl AsRef<Path>) -> Result<Self>
    where
        Self: Sized;
}

pub struct FileWatcher<T> {
    _watcher: RecommendedWatcher,
    rx: mpsc::Receiver<Result<T>>,
}

impl<T> FileWatcher<T>
where
    T: 'static + FromTomlFile + Send,
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
                            let data: Result<T> = T::from_toml_file(&path_buf);
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
        })
    }

    pub fn poll_update(&mut self) -> Option<Result<T>> {
        self.rx.try_recv().ok()
    }
}
