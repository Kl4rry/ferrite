use std::{collections::HashMap, path::PathBuf, sync::mpsc, time::Duration};

use anyhow::Result;
use notify_debouncer_full::{
    DebounceEventResult, Debouncer, RecommendedCache, new_debouncer,
    notify::{RecommendedWatcher, RecursiveMode},
};
use slotmap::SlotMap;

use crate::{buffer::Buffer, event_loop_proxy::EventLoopProxy, workspace::BufferId};

pub struct BufferWatcher {
    watcher: Debouncer<RecommendedWatcher, RecommendedCache>,
    buffers: HashMap<PathBuf, bool>,
    update_rx: mpsc::Receiver<PathBuf>,
}

impl BufferWatcher {
    pub fn new(proxy: Box<dyn EventLoopProxy>) -> Result<Self> {
        let (tx, rx) = mpsc::channel();

        let debouncer = new_debouncer(
            Duration::from_millis(200),
            None,
            move |result: DebounceEventResult| {
                if let Ok(events) = result {
                    for mut event in events {
                        if event.kind.is_modify() {
                            if let Some(path) = event.event.paths.pop() {
                                let _ = tx.send(path);
                                proxy.request_render();
                            }
                        }
                    }
                }
            },
        );

        let watcher = match debouncer {
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
