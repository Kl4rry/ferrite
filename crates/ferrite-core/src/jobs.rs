use std::path::PathBuf;

use ropey::Rope;

use crate::{job_manager::JobHandle, workspace::BufferId};

pub struct SaveBufferJob {
    pub buffer_id: BufferId,
    pub path: PathBuf,
    pub history_id: u64,
    pub written: usize,
}

pub type ShellJobHandle =
    JobHandle<Result<(Option<BufferId>, Rope), anyhow::Error>, (BufferId, Rope)>;
