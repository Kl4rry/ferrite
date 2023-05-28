use std::{
    process::Command,
    sync::{Arc, Mutex},
    thread,
};

use notify::Watcher;

use crate::tui_app::event_loop::TuiEventLoopProxy;

fn get_current_branch() -> Option<String> {
    match Command::new("git")
        .args(["branch", "--show-current"])
        .output()
    {
        Ok(output) => {
            if output.status.success() {
                Some(String::from_utf8_lossy(&output.stdout).to_string())
            } else {
                None
            }
        }
        Err(err) => {
            log::error!("{}", err);
            None
        }
    }
}

struct FileNotificationEventHandler {
    current_branch: Arc<Mutex<Option<String>>>,
    proxy: TuiEventLoopProxy,
}

impl notify::EventHandler for FileNotificationEventHandler {
    fn handle_event(&mut self, event: notify::Result<notify::Event>) {
        if event.is_ok() {
            let mut guard = self.current_branch.lock().unwrap();
            *guard = get_current_branch();
            self.proxy.request_render();
        }
    }
}

pub struct BranchWatcher {
    current_branch: Arc<Mutex<Option<String>>>,
    _watcher: notify::RecommendedWatcher,
}

impl BranchWatcher {
    pub fn new(proxy: TuiEventLoopProxy) -> Result<Self, notify::Error> {
        let current_branch = Arc::new(Mutex::new(None));
        let mut watcher = notify::recommended_watcher(FileNotificationEventHandler {
            current_branch: current_branch.clone(),
            proxy: proxy.clone(),
        })?;
        let _ = watcher.watch(&std::env::current_dir()?, notify::RecursiveMode::Recursive);

        let current_branch_thread = current_branch.clone();
        thread::spawn(move || {
            if let Some(branch) = get_current_branch() {
                *current_branch_thread.lock().unwrap() = Some(branch);
                proxy.request_render();
            }
        });

        Ok(Self {
            current_branch,
            _watcher: watcher,
        })
    }

    pub fn current_branch(&self) -> Option<String> {
        self.current_branch.lock().unwrap().clone()
    }
}
