use std::{collections::HashMap, path::PathBuf, sync::mpsc};

use anyhow::Result;
use notify::{Error, Event, RecommendedWatcher, RecursiveMode, Watcher};
use slotmap::SlotMap;

use crate::{buffer::Buffer, event_loop_proxy::EventLoopProxy, workspace::BufferId};

pub struct BufferWatcher {
    watcher: RecommendedWatcher,
    buffers: HashMap<PathBuf, bool>,
    update_rx: mpsc::Receiver<PathBuf>,
}

impl BufferWatcher {
    pub fn new(proxy: Box<dyn EventLoopProxy>) -> Result<Self> {
        let (tx, rx) = mpsc::channel();
        let watcher = notify::recommended_watcher(move |event: Result<Event, Error>| {
            if let Ok(mut event) = event {
                if event.kind.is_modify() {
                    if let Some(path) = event.paths.pop() {
                        let _ = tx.send(path);
                        proxy.request_render();
                    }
                }
            }
        });

        let watcher = match watcher {
            Ok(watcher) => watcher,
            Err(err) => {
                tracing::error!("Error starting buffer watcher: {err}");
                return Err(err.into());
            }
        };

        Ok(Self {
            watcher,
            buffers: HashMap::new(),
            update_rx: rx,
        })
    }

    pub fn update(&mut self, buffers: &mut SlotMap<BufferId, Buffer>) {
        while let Ok(path) = self.update_rx.try_recv() {
            for buffer in buffers.values_mut() {
                if let Some(file) = buffer.file() {
                    if file == path && !buffer.is_dirty() {
                        let _ = buffer.reload();
                    }
                }
            }
        }

        for buffer in buffers.values() {
            if let Some(file) = buffer.file() {
                if !self.buffers.contains_key(file) {
                    tracing::info!("Started watching: {file:?}");
                    let _ = self.watcher.watch(file, RecursiveMode::NonRecursive);
                    self.buffers.insert(file.into(), true);
                }
            }
        }

        for (path, touched) in &mut self.buffers {
            *touched = false;
            for buffer in buffers.values() {
                if let Some(file) = buffer.file() {
                    if path == file {
                        *touched = true;
                    }
                }
            }
        }

        self.buffers.retain(|path, touched| {
            if !*touched {
                let _ = self.watcher.unwatch(path);
                tracing::info!("Stopped watching: {path:?}");
            }
            *touched
        });
    }
}
