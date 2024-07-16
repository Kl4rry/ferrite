use std::{path::PathBuf, time::Instant};

use crate::workspace::BufferId;

pub struct SaveBufferJob {
    pub buffer_id: BufferId,
    pub path: PathBuf,
    pub last_edit: Instant,
    pub written: usize,
}
