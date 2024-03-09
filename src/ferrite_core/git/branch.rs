use std::{
    process::Command,
    sync::{Arc, Mutex},
    thread,
};

use crate::{ferrite_core::pubsub::Subscriber, tui_app::event_loop::TuiEventLoopProxy};

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

pub struct BranchWatcher {
    current_branch: Arc<Mutex<Option<String>>>,
    proxy: TuiEventLoopProxy,
}

impl BranchWatcher {
    pub fn new(
        proxy: TuiEventLoopProxy,
        mut change_detector: Subscriber<()>,
    ) -> Result<Self, notify::Error> {
        let current_branch = Arc::new(Mutex::new(None));

        {
            let current_branch_thread = current_branch.clone();
            let thread_proxy = proxy.clone();
            thread::spawn(move || {
                if let Some(branch) = get_current_branch() {
                    *current_branch_thread.lock().unwrap() = Some(branch);
                    thread_proxy.request_render();
                }

                while change_detector.recive().is_ok() {
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
                }
            });
        }

        Ok(Self {
            proxy,
            current_branch,
        })
    }

    pub fn current_branch(&self) -> Option<String> {
        self.current_branch.lock().unwrap().clone()
    }

    pub fn force_reload(&self) {
        let proxy = self.proxy.clone();
        let current_branch_thread = self.current_branch.clone();
        thread::spawn(move || {
            if let Some(branch) = get_current_branch() {
                *current_branch_thread.lock().unwrap() = Some(branch);
                proxy.request_render();
            }
        });
    }
}
