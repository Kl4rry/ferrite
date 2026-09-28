use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use ferrite_git::repo::{get_current_branch, get_current_head, get_git_directory};
use notify_debouncer_full::{
    DebounceEventResult, Debouncer, RecommendedCache, new_debouncer,
    notify::{self, RecommendedWatcher, RecursiveMode},
};

use crate::event_loop_proxy::{EventLoopProxy, UserEvent};

struct RepoInfo {
    branch: Option<String>,
    head: Option<String>,
}

pub struct GitWatcher {
    repo_info: Arc<Mutex<RepoInfo>>,
    change_detected: Arc<Mutex<bool>>,
    proxy: Box<dyn EventLoopProxy<UserEvent>>,
    _watcher: Option<Debouncer<RecommendedWatcher, RecommendedCache>>,
}

impl GitWatcher {
    pub fn new(proxy: Box<dyn EventLoopProxy<UserEvent>>) -> Result<Self, notify::Error> {
        let current_repo_info = Arc::new(Mutex::new(RepoInfo {
            branch: None,
            head: None,
        }));
        let change_detected = Arc::new(Mutex::new(false));
        let mut watcher = None;

        {
            let current_repo_info_thread = current_repo_info.clone();
            let change_detected_thread = change_detected.clone();
            let thread_proxy = proxy.dup();

            if let Some(git_dir) = get_git_directory() {
                watcher = match new_debouncer(
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
                            if event.kind.is_modify()
                                || event.kind.is_create()
                                || event.kind.is_remove()
                            {
                                if let Some(branch) = get_current_branch() {
                                    let mut guard = current_repo_info_thread.lock().unwrap();
                                    if let Some(current) = &mut guard.branch
                                        && *current != branch
                                    {
                                        tracing::info!(
                                            "Git branch changed from `{current}` to `{branch}`"
                                        );
                                    }
                                    guard.branch = Some(branch);
                                }
                                if let Some(head) = get_current_head() {
                                    let mut guard = current_repo_info_thread.lock().unwrap();
                                    if let Some(current) = &mut guard.head
                                        && *current != head
                                    {
                                        tracing::info!(
                                            "Git HEAD changed from `{current}` to `{head}`"
                                        );
                                    }
                                    guard.head = Some(head);
                                }
                                *change_detected_thread.lock().unwrap() = true;
                                thread_proxy.request_render("change in .git folder detected");
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
            repo_info: current_repo_info,
            change_detected,
            _watcher: watcher,
        };
        new.force_reload();
        Ok(new)
    }

    pub fn current_branch(&self) -> Option<String> {
        self.repo_info.lock().unwrap().branch.clone()
    }

    pub fn current_head(&self) -> Option<String> {
        self.repo_info.lock().unwrap().head.clone()
    }

    pub fn consume_change(&mut self) -> bool {
        let mut guard = self.change_detected.lock().unwrap();
        if *guard {
            *guard = false;
            return true;
        }
        false
    }

    pub fn force_reload(&self) {
        let proxy = self.proxy.dup();
        let current_repo_info_thread = self.repo_info.clone();
        rayon::spawn(move || {
            if let Some(branch) = get_current_branch() {
                current_repo_info_thread.lock().unwrap().branch = Some(branch);
                proxy.request_render("git branch force reloaded");
            }
        });
    }
}
