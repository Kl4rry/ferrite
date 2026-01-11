use std::path::PathBuf;

use anyhow::Result;
use ferrite_blame::BlameHunk;

use crate::{
    event_loop_proxy::{EventLoopProxy, UserEvent},
    promise::Promise,
};

pub struct Blame {
    promise: Promise<Result<Vec<BlameHunk>>>,
    blame: Vec<BlameHunk>,
}

impl Blame {
    pub fn new() -> Self {
        Self {
            promise: Promise::empty(),
            blame: Vec::new(),
        }
    }

    pub fn request_update(&mut self, path: PathBuf, proxy: Box<dyn EventLoopProxy<UserEvent>>) {
        self.promise = Promise::spawn(proxy, move || ferrite_blame::blame(path));
    }

    pub fn get_blame(&mut self) -> &[BlameHunk] {
        if let Some(blame) = self.promise.poll() {
            match blame {
                Ok(blame) => self.blame = blame,
                Err(err) => tracing::error!("Blame error: {err}"),
            }
        }
        &self.blame
    }

    pub fn reset(&mut self) {
        self.blame = Vec::new();
        self.promise = Promise::empty();
    }
}

impl Default for Blame {
    fn default() -> Self {
        Self::new()
    }
}
