use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    thread,
    time::Instant,
};

use lexical_sort::StringSort;
use notify::{RecursiveMode, Watcher};
use rayon::prelude::*;

use crate::core::pubsub::{self, Subscriber};

fn get_text_file_path(path: PathBuf) -> Option<PathBuf> {
    if is_text_file(&path) {
        Some(path)
    } else {
        None
    }
}

fn is_text_file(path: impl AsRef<Path>) -> bool {
    let Some(mime) = tree_magic_mini::from_filepath(path.as_ref()) else {
        return false;
    };

    mime.starts_with("text")
}

fn trim_path(start: &str, path: &Path) -> String {
    path.to_string_lossy()
        .trim_start_matches(start)
        .trim_start_matches(std::path::MAIN_SEPARATOR)
        .to_string()
}

pub struct FileDaemon {
    _watcher: notify::RecommendedWatcher,
    subscriber: Subscriber<Vec<String>>,
}

impl FileDaemon {
    pub fn new(path: PathBuf) -> anyhow::Result<Self> {
        let (publisher, subscriber) = pubsub::create(Vec::new());

        let (update_tx, update_rx) = cb::unbounded();
        let mut watcher = notify::recommended_watcher(
            move |event: std::result::Result<notify::event::Event, notify::Error>| {
                if let Ok(event) = event {
                    let _ = update_tx.send(event);
                }
            },
        )?;
        let _ = watcher.watch(&path, RecursiveMode::Recursive);

        let path_to_search = path.clone();
        thread::spawn(move || {
            let mut tracked_files = HashSet::new();

            let path: PathBuf = path_to_search;
            let path_str = path.to_string_lossy().into_owned();

            let mut iterator = jwalk::WalkDir::new(&path)
                .follow_links(true)
                .into_iter()
                .filter_map(|result| result.ok());

            {
                loop {
                    let start = Instant::now();
                    let entries: Vec<_> = iterator.by_ref().take(1000).collect();

                    if entries.is_empty() {
                        break;
                    }

                    tracked_files.par_extend(
                        entries
                            .par_iter()
                            .filter(|entry| entry.file_type().is_file())
                            .filter_map(|entry| get_text_file_path(entry.path())),
                    );

                    let mut files: Vec<_> = tracked_files
                        .iter()
                        .map(|path| trim_path(&path_str, path))
                        .collect();
                    files.string_sort(lexical_sort::natural_lexical_cmp);
                    if publisher.publish(files).is_err() {
                        return;
                    }

                    log::info!(
                        "Found {} files in {}ms",
                        tracked_files.len(),
                        start.elapsed().as_millis()
                    );
                }
            }

            let mut updated = false;
            loop {
                match update_rx.recv() {
                    Ok(event) => {
                        for path in event.paths {
                            updated = updated
                                || if is_text_file(&path) {
                                    tracked_files.insert(path)
                                } else {
                                    tracked_files.remove(&path)
                                };
                        }
                    }
                    Err(err) => {
                        log::error!("File daemon thread exit: {err}");
                        return;
                    }
                }

                if update_rx.is_empty() && updated {
                    updated = false;
                    let mut files: Vec<_> = tracked_files
                        .iter()
                        .map(|path| trim_path(&path_str, path))
                        .collect();
                    files.string_sort(lexical_sort::natural_lexical_cmp);
                    if publisher.publish(files).is_err() {
                        return;
                    }
                }
            }
        });

        Ok(Self {
            _watcher: watcher,
            subscriber,
        })
    }

    pub fn subscribe(&self) -> Subscriber<Vec<String>> {
        self.subscriber.clone()
    }
}
