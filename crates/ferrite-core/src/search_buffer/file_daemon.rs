use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    sync::{Arc, Condvar, Mutex},
    thread,
    time::{Duration, Instant},
};

use cb::select;
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use lexical_sort::StringSort;
use notify::{RecursiveMode, Watcher};
use rayon::prelude::*;

use crate::{
    config::Config,
    pubsub::{self, Subscriber},
};

fn get_text_file_path(path: PathBuf) -> Option<PathBuf> {
    if is_text_file(&path) {
        Some(path)
    } else {
        None
    }
}

fn is_text_file(path: impl AsRef<Path>) -> bool {
    let Ok(mut file) = File::open(&path) else {
        return false;
    };

    let mut buf = [0; 1024];
    let Ok(read) = file.read(&mut buf) else {
        return false;
    };

    let content_type = content_inspector::inspect(&buf[..read]);
    content_type.is_text()
}

fn trim_path(start: &str, path: &Path) -> String {
    path.to_string_lossy()
        .trim_start_matches(start)
        .trim_start_matches(std::path::MAIN_SEPARATOR)
        .to_string()
}

pub struct FileDaemon {
    subscriber: Subscriber<Vec<String>>,
    change_detector: Subscriber<()>,
    exit_tx: cb::Sender<()>,
}

impl FileDaemon {
    pub fn new(path: PathBuf, config: &Config) -> anyhow::Result<Self> {
        let (exit_tx, exit_rx) = cb::bounded::<()>(1);
        let (publisher, subscriber) = pubsub::create(Vec::new());
        let (change_broadcaster, change_detector) = pubsub::create(());
        let path_to_search = path.clone();
        let picker_config = config.picker;
        let recursive = config.watch_recursive;
        let watch_workspace = config.watch_workspace;

        thread::spawn(move || {
            let pair = Arc::new((Mutex::new(false), Condvar::new()));
            let watch_pair = pair.clone();
            let (update_tx, update_rx) = cb::unbounded();
            let watcher_thread = thread::spawn(move || {
                let mut watcher = match notify::recommended_watcher(
                    move |event: std::result::Result<notify::event::Event, notify::Error>| {
                        if let Ok(event) = event {
                            let _ = update_tx.send(event);
                        }
                    },
                ) {
                    Ok(watcher) => watcher,
                    Err(err) => {
                        tracing::error!("Error starting file watcher {err}");
                        return;
                    }
                };

                if watch_workspace {
                    let mode = match recursive {
                        true => RecursiveMode::Recursive,
                        false => RecursiveMode::NonRecursive,
                    };
                    tracing::info!("watching workspace: {:?} using {:?}", path, mode);

                    if let Err(err) = watcher.watch(&path, mode) {
                        tracing::error!("Error starting file watcher {err}");
                    };
                }
                let (lock, cvar) = &*watch_pair;
                drop(
                    cvar.wait_while(lock.lock().unwrap(), |exit| !*exit)
                        .unwrap(),
                );
            });

            let _guard = Defer::new(|| {
                let (lock, cvar) = &*pair;
                *lock.lock().unwrap() = true;
                cvar.notify_all();
                watcher_thread.join().unwrap();
            });

            let mut tracked_files = HashSet::new();

            let path: PathBuf = path_to_search;
            let path_str = path.to_string_lossy().into_owned();

            let mut iterator = ignore::WalkBuilder::new(&path)
                .follow_links(false)
                .ignore(picker_config.follow_ignore)
                .git_global(picker_config.follow_git_global)
                .git_ignore(picker_config.follow_gitignore)
                .git_exclude(picker_config.follow_git_exclude)
                .build()
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
                            .filter(|entry| entry.file_type().map(|f| f.is_file()).unwrap_or(false))
                            .filter_map(|entry| {
                                if picker_config.show_only_text_files {
                                    get_text_file_path(entry.path().to_path_buf())
                                } else {
                                    Some(entry.path().to_path_buf())
                                }
                            }),
                    );

                    let mut files: Vec<_> = tracked_files
                        .iter()
                        .map(|path| trim_path(&path_str, path))
                        .collect();
                    files.string_sort(lexical_sort::natural_lexical_cmp);
                    if publisher.publish(files).is_err() {
                        return;
                    }

                    tracing::trace!(
                        "Found {} files in {}ms",
                        tracked_files.len(),
                        start.elapsed().as_millis()
                    );
                }
            }

            let global_gitignore = Gitignore::global().0;

            let mut gitignore_cache: HashMap<PathBuf, Gitignore> = HashMap::new();
            let mut last_clear = Instant::now();

            let mut updated = false;
            loop {
                {
                    let now = Instant::now();
                    if now.duration_since(last_clear) > Duration::from_secs(30)
                        && update_rx.is_empty()
                    {
                        last_clear = now;
                        gitignore_cache.clear();
                    }
                }

                let workspace_dir = std::env::current_dir().unwrap();

                select! {
                    recv(exit_rx) -> _ => {
                        tracing::info!("File daemon thread exit");
                        return
                    },
                    recv(update_rx) -> res => {
                        match res {
                            Ok(event) => {
                                for path in event.paths {
                                    if !picker_config.show_only_text_files || is_text_file(&path) {
                                        let str_path = path.to_string_lossy().into_owned();
                                        let relative_path = Path::new(
                                            str_path.trim_start_matches(&*workspace_dir.to_string_lossy()),
                                        );
                                        let is_hidden = !picker_config.show_hidden
                                            && relative_path.components().any(
                                                |component| match component {
                                                    std::path::Component::Normal(name) => {
                                                        name.to_string_lossy().starts_with('.')
                                                    }
                                                    _ => false,
                                                },
                                            );
                                        let is_global_ignore = picker_config.follow_git_global
                                            && global_gitignore.matched(&path, false).is_ignore();

                                        if is_hidden || is_global_ignore {
                                            continue;
                                        }

                                        match gitignore_cache.get(&path) {
                                            Some(ignore) => {
                                                if ignore.matched(&path, false).is_ignore() {
                                                    updated |= tracked_files.insert(path);
                                                }
                                            }
                                            None => {
                                                let mut builder = GitignoreBuilder::new(&workspace_dir);
                                                for part in path.ancestors() {
                                                    if part.starts_with(&workspace_dir) && part != path {
                                                        if picker_config.follow_gitignore {
                                                            let _ = builder.add(part.join(".gitignore"));
                                                        }
                                                        if picker_config.follow_ignore {
                                                            let _ = builder.add(part.join(".ignore"));
                                                        }
                                                        if picker_config.follow_git_exclude {
                                                            let _ =
                                                                builder.add(part.join(".git/info/exclude"));
                                                        }
                                                    }
                                                }

                                                match builder.build() {
                                                    Ok(ignore) => {
                                                        if !ignore
                                                            .matched_path_or_any_parents(&path, false)
                                                            .is_ignore()
                                                        {
                                                            updated |= tracked_files.insert(path.clone());
                                                        }
                                                        if let Some(parent) = path.parent() {
                                                            gitignore_cache
                                                                .insert(parent.to_path_buf(), ignore);
                                                        }
                                                    }
                                                    Err(_) => {
                                                        updated |= tracked_files.insert(path);
                                                    }
                                                }
                                            }
                                        }
                                    } else {
                                        updated |= tracked_files.remove(&path);
                                    };
                                }
                            }
                            Err(err) => {
                                tracing::info!("File daemon thread exit {err}");
                                return;
                            }
                        }

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
                    if change_broadcaster.publish(()).is_err() {
                        return;
                    }
                }
            }
        });

        Ok(Self {
            subscriber,
            change_detector,
            exit_tx,
        })
    }

    pub fn subscribe(&self) -> Subscriber<Vec<String>> {
        self.subscriber.clone()
    }

    pub fn change_detector(&self) -> Subscriber<()> {
        self.change_detector.clone()
    }
}

impl Drop for FileDaemon {
    fn drop(&mut self) {
        let _ = self.exit_tx.send(());
    }
}

struct Defer<F: FnOnce()> {
    closure: Option<F>,
}

impl<F: FnOnce()> Defer<F> {
    fn new(closure: F) -> Self {
        Self {
            closure: Some(closure),
        }
    }
}

impl<F: FnOnce()> Drop for Defer<F> {
    fn drop(&mut self) {
        let closure = self.closure.take().unwrap();
        (closure)()
    }
}
