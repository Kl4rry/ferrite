use std::{path::PathBuf, time::Instant};

pub struct SaveBufferJob {
    pub buffer_id: usize,
    pub path: PathBuf,
    pub last_edit: Instant,
    pub written: usize,
}
