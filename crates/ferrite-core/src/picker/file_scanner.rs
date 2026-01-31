use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Instant,
};

use ferrite_utility::trim::trim_path;
use rayon::prelude::*;

use crate::{
    config::editor::{Editor, PickerConfig},
    pubsub::{self, Publisher, Subscriber},
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

pub struct FileScanner {
    subscriber: Subscriber<boxcar::Vec<String>>,
    running: Arc<AtomicBool>,
}

impl FileScanner {
    pub fn new(path: PathBuf, config: &Editor) -> Self {
        let (publisher, subscriber): (Publisher<boxcar::Vec<String>>, _) =
            pubsub::create(boxcar::Vec::new());
        let path_to_search = path.clone();
        let picker_config = config.picker;
        let running = Arc::new(AtomicBool::new(true));

        let thread_runnig = running.clone();
        thread::spawn(move || {
            scan_files(
                &publisher,
                path_to_search.clone(),
                picker_config,
                thread_runnig.clone(),
            );
        });

        Self {
            subscriber,
            running,
        }
    }

    pub fn subscribe(&self) -> Subscriber<boxcar::Vec<String>> {
        self.subscriber.clone()
    }
}

impl Drop for FileScanner {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
    }
}

fn scan_files(
    publisher: &Publisher<boxcar::Vec<String>>,
    path: PathBuf,
    config: PickerConfig,
    running: Arc<AtomicBool>,
) {
    if publisher.publish().is_err() {
        return;
    }

    let path_str = path.to_string_lossy().into_owned();
    let mut iterator = ignore::WalkBuilder::new(&path)
        .follow_links(false)
        .hidden(!config.show_hidden)
        .ignore(config.follow_ignore)
        .git_global(config.follow_git_global)
        .git_ignore(config.follow_gitignore)
        .git_exclude(config.follow_git_exclude)
        .overrides(config.overrides())
        .sort_by_file_path(move |lhs, rhs| {
            let lhs = lhs.to_string_lossy();
            let rhs = rhs.to_string_lossy();
            ferrite_utility::natural_cmp::natural_cmp(&lhs, &rhs)
        })
        .build()
        .filter_map(|result| result.ok());

    let mut tracked_files = Vec::new();
    let start = Instant::now();

    loop {
        if !running.load(Ordering::Relaxed) {
            return;
        }

        let entries: Vec<_> = iterator.by_ref().take(200).collect();

        if entries.is_empty() {
            break;
        }

        tracing::debug!("extending tracked files in scanner");
        tracked_files.par_extend(
            entries
                .par_iter()
                .filter(|entry| entry.file_type().map(|f| f.is_file()).unwrap_or(false))
                .filter_map(|entry| {
                    if config.show_only_text_files {
                        get_text_file_path(entry.path().to_path_buf())
                    } else {
                        Some(entry.path().to_path_buf())
                    }
                }),
        );

        publisher.modify(|published_files| {
            for file in tracked_files.iter().map(|path| trim_path(&path_str, path)) {
                published_files.push(file);
            }
        });

        tracked_files.clear();

        if publisher.publish().is_err() {
            return;
        }
    }

    publisher.modify(|published_files| {
        tracing::info!(
            "Found {} files in {}ms",
            published_files.count(),
            start.elapsed().as_millis()
        );
    })
}
