use std::{collections::HashMap, path::PathBuf, sync::mpsc, time::Duration};

use anyhow::Result;
use notify_debouncer_full::{
    DebounceEventResult, Debouncer, RecommendedCache, new_debouncer,
    notify::{RecommendedWatcher, RecursiveMode},
};
use slotmap::SlotMap;

use crate::{
    buffer::Buffer,
    event_loop_proxy::{EventLoopProxy, UserEvent},
    workspace::BufferId,
};

pub struct BufferWatcher {
    pub watched: HashMap<PathBuf, bool>,
    watcher: Debouncer<RecommendedWatcher, RecommendedCache>,
    update_rx: mpsc::Receiver<PathBuf>,
}

impl BufferWatcher {
    pub fn new(proxy: Box<dyn EventLoopProxy<UserEvent>>) -> Result<Self> {
        let (tx, rx) = mpsc::channel();

        let debouncer = new_debouncer(
            Duration::from_millis(200),
            None,
            move |result: DebounceEventResult| {
                let events = match result {
                    Ok(events) => events,
                    Err(err) => {
                        tracing::error!("Error getting events: {err:?}");
                        return;
                    }
                };
                for event in events {
                    if event.kind.is_modify() || event.kind.is_create() {
                        for path in event.event.paths {
                            let _ = tx.send(path);
                            proxy.request_render("watched filed updated");
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
            watched: HashMap::new(),
            watcher,
            update_rx: rx,
        })
    }

    pub fn update(&mut self, buffers: &mut SlotMap<BufferId, Buffer>) {
        while let Ok(path) = self.update_rx.try_recv() {
            for buffer in buffers.values_mut() {
                if let Some(file) = buffer.file()
                    && file == path
                    && !buffer.is_dirty()
                {
                    let _ = buffer.reload();
                }
            }
        }

        for buffer in buffers.values() {
            if let Some(parent) = buffer.directory()
                && !self.watched.contains_key(parent)
            {
                match self.watcher.watch(parent, RecursiveMode::NonRecursive) {
                    Ok(_) => {
                        tracing::info!("Started watching: {parent:?}");
                    }
                    Err(err) => {
                        tracing::info!("Error watching {parent:?} {err}");
                    }
                }
                self.watched.insert(parent.into(), true);
            }
        }

        for (path, touched) in &mut self.watched {
            *touched = false;
            for buffer in buffers.values() {
                if let Some(parent) = buffer.directory()
                    && path == parent
                {
                    *touched = true;
                }
            }
        }

        self.watched.retain(|path, touched| {
            if !*touched {
                let _ = self.watcher.unwatch(path);
                tracing::info!("Stopped watching: {path:?}");
            }
            *touched
        });
    }
}
