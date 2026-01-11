use std::{
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use anyhow::Result;

use crate::{
    event_loop_proxy::{EventLoopProxy, UserEvent},
    promise::Promise,
};

pub struct Blame {
    promise: Promise<Result<Vec<BlameHunk>>>,
    blame: Vec<BlameHunk>,
}

#[derive(Debug)]
pub struct BlameHunk {
    pub start_line: usize,
    pub len: usize,
    pub commit: String,
    pub name: String,
    pub email: String,
    pub time: SystemTime,
}

impl Blame {
    pub fn new() -> Self {
        Self {
            promise: Promise::empty(),
            blame: Vec::new(),
        }
    }

    pub fn request_update(&mut self, path: PathBuf, proxy: Box<dyn EventLoopProxy<UserEvent>>) {
        self.promise = Promise::spawn(proxy, move || {
            let path: String = path.to_string_lossy().into();
            let repo = git2::Repository::discover(".")?;
            let Some(root) = repo.workdir() else {
                anyhow::bail!("bare repo");
            };
            let blame = repo.blame_file(
                Path::new(path.trim_start_matches(&*root.to_string_lossy())),
                None,
            )?;

            let mut hunks = Vec::with_capacity(blame.len());
            for hunk in blame.iter() {
                let signature = hunk.orig_signature();
                hunks.push(BlameHunk {
                    start_line: hunk.final_start_line() - 1,
                    len: hunk.lines_in_hunk(),
                    commit: format!("{}", hunk.final_commit_id()),
                    name: String::from_utf8_lossy(signature.name_bytes()).into(),
                    email: String::from_utf8_lossy(signature.email_bytes()).into(),
                    time: SystemTime::UNIX_EPOCH
                        .checked_add(Duration::from_secs(
                            signature.when().seconds().try_into().unwrap_or(0),
                        ))
                        .unwrap_or(SystemTime::UNIX_EPOCH),
                });
            }
            Ok(hunks)
        });
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
