use std::{
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use notify_debouncer_full::{
    DebounceEventResult, Debouncer, RecommendedCache, new_debouncer,
    notify::{self, RecommendedWatcher, RecursiveMode},
};

use crate::event_loop_proxy::{EventLoopProxy, UserEvent};

fn get_current_branch() -> Option<String> {
    let repo = git2::Repository::discover(".").ok()?;
    let head = repo.head().ok()?;
    let head_name: String = String::from_utf8_lossy(head.shorthand_bytes()).into();
    Some(head_name)
}

fn get_git_directory() -> Option<String> {
    let repo = git2::Repository::discover(".").ok()?;
    Some(repo.path().to_string_lossy().to_string())
}

pub struct BranchWatcher {
    current_branch: Arc<Mutex<Option<String>>>,
    proxy: Box<dyn EventLoopProxy<UserEvent>>,
    _watcher: Option<Debouncer<RecommendedWatcher, RecommendedCache>>,
}

impl BranchWatcher {
    pub fn new(proxy: Box<dyn EventLoopProxy<UserEvent>>) -> Result<Self, notify::Error> {
        let current_branch = Arc::new(Mutex::new(None));
        let mut watcher = None;

        {
            let current_branch_thread = current_branch.clone();
            let thread_proxy = proxy.dup();

            if let Some(git_dir) = get_git_directory() {
                watcher = match new_debouncer(
                    Duration::from_millis(200),
                    None,
                    move |_: DebounceEventResult| {
                        if let Some(branch) = get_current_branch() {
                            {
                                let mut guard = current_branch_thread.lock().unwrap();
                                if let Some(current) = &*guard
                                    && current != &branch
                                {
                                    tracing::info!(
                                        "Git branch changed from `{current}` to `{branch}`"
                                    );
                                    thread_proxy.request_render("git branch changed");
                                }
                                *guard = Some(branch);
                            }
                        }
                    },
                ) {
                    Ok(mut watcher) => {
                        if let Err(err) = watcher.watch(&git_dir, RecursiveMode::NonRecursive) {
                            tracing::error!("Error starting branch watcher {err}");
                        }
                        Some(watcher)
                    }
                    Err(err) => {
                        tracing::error!("Error starting branch watcher {err}");
                        None
                    }
                };
            }
        }

        let new = Self {
            proxy,
            current_branch,
            _watcher: watcher,
        };
        new.force_reload();
        Ok(new)
    }

    pub fn current_branch(&self) -> Option<String> {
        self.current_branch.lock().unwrap().clone()
    }

    pub fn force_reload(&self) {
        let proxy = self.proxy.dup();
        let current_branch_thread = self.current_branch.clone();
        thread::spawn(move || {
            if let Some(branch) = get_current_branch() {
                *current_branch_thread.lock().unwrap() = Some(branch);
                proxy.request_render("git branch force reloaded");
            }
        });
    }
}
