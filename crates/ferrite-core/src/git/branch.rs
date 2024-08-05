use std::{
    path::PathBuf,
    process::Command,
    sync::{Arc, Mutex},
    thread,
};

use notify::{RecommendedWatcher, Watcher};

use crate::event_loop_proxy::EventLoopProxy;

fn get_current_branch() -> Option<String> {
    match Command::new("git")
        .args(["branch", "--show-current"])
        .output()
    {
        Ok(output) => {
            if output.status.success() {
                Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
            } else {
                None
            }
        }
        Err(err) => {
            tracing::error!("{}", err);
            None
        }
    }
}

fn get_git_directory() -> Option<String> {
    match Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
    {
        Ok(output) => {
            if output.status.success() {
                Some(format!(
                    "{}/.git",
                    String::from_utf8_lossy(&output.stdout).trim()
                ))
            } else {
                None
            }
        }
        Err(err) => {
            tracing::error!("{}", err);
            None
        }
    }
}

pub struct BranchWatcher {
    current_branch: Arc<Mutex<Option<String>>>,
    proxy: Box<dyn EventLoopProxy>,
    _watcher: Option<RecommendedWatcher>,
}

impl BranchWatcher {
    pub fn new(proxy: Box<dyn EventLoopProxy>) -> Result<Self, notify::Error> {
        let current_branch = Arc::new(Mutex::new(None));
        let mut watcher = None;

        {
            let current_branch_thread = current_branch.clone();
            let thread_proxy = proxy.dup();

            if let Some(git_dir) = get_git_directory() {
                watcher = match notify::recommended_watcher(
                    move |_: std::result::Result<notify::event::Event, notify::Error>| {
                        if let Some(branch) = get_current_branch() {
                            {
                                let mut guard = current_branch_thread.lock().unwrap();
                                if let Some(current) = &*guard {
                                    if current != &branch {
                                        tracing::info!(
                                            "Git branch changed from `{current}` to `{branch}`"
                                        );
                                    }
                                }
                                *guard = Some(branch);
                            }
                            thread_proxy.request_render();
                        }
                    },
                ) {
                    Ok(watcher) => Some(watcher),
                    Err(err) => {
                        tracing::error!("Error starting branch watcher {err}");
                        None
                    }
                };

                if let Some(watcher) = &mut watcher {
                    if let Err(err) =
                        watcher.watch(&PathBuf::from(git_dir), notify::RecursiveMode::NonRecursive)
                    {
                        tracing::error!("Error starting branch watcher {err}");
                    }
                }
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
                proxy.request_render();
            }
        });
    }
}
