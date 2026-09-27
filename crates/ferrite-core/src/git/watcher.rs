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
    proxy: Box<dyn EventLoopProxy<UserEvent>>,
    head: String,
    _watcher: Option<Debouncer<RecommendedWatcher, RecommendedCache>>,
}

impl GitWatcher {
    pub fn new(proxy: Box<dyn EventLoopProxy<UserEvent>>) -> Result<Self, notify::Error> {
        let current_repo_info = Arc::new(Mutex::new(RepoInfo {
            branch: None,
            head: None,
        }));
        let mut watcher = None;

        {
            let current_repo_info_thread = current_repo_info.clone();
            let thread_proxy = proxy.dup();

            if let Some(git_dir) = get_git_directory() {
                watcher = match new_debouncer(
                    Duration::from_millis(200),
                    None,
                    move |_: DebounceEventResult| {
                        if let Some(branch) = get_current_branch() {
                            let mut guard = current_repo_info_thread.lock().unwrap();
                            if let Some(current) = &mut guard.branch
                                && *current != branch
                            {
                                tracing::info!("Git branch changed from `{current}` to `{branch}`");
                                thread_proxy.request_render("git branch changed");
                            }
                            guard.branch = Some(branch);
                        }
                        if let Some(head) = get_current_head() {
                            let mut guard = current_repo_info_thread.lock().unwrap();
                            if let Some(current) = &mut guard.head
                                && *current != head
                            {
                                tracing::info!("Git HEAD changed from `{current}` to `{head}`");
                                thread_proxy.request_render("git head changed");
                            }
                            guard.head = Some(head);
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

        let mut new = Self {
            proxy,
            repo_info: current_repo_info,
            head: String::new(),
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

    pub fn consume_head_change(&mut self) -> Option<String> {
        if let Some(head) = &self.repo_info.lock().unwrap().head
            && *head != self.head
        {
            self.head.clone_from(head);
            return Some(head.clone());
        }
        None
    }

    pub fn force_reload(&mut self) {
        // clear string to force head changed
        self.head.clear();
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
