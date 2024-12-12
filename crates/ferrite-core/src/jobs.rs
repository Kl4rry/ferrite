use std::{path::PathBuf, time::Instant};

use ropey::Rope;

use crate::{job_manager::JobHandle, workspace::BufferId};

pub struct SaveBufferJob {
    pub buffer_id: BufferId,
    pub path: PathBuf,
    pub last_edit: Instant,
    pub written: usize,
}

pub type ShellJobHandle =
    JobHandle<Result<(Option<BufferId>, Rope), anyhow::Error>, (BufferId, Rope)>;
