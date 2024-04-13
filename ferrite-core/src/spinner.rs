use std::time::{Duration, Instant};

const FRAMES: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

pub struct Spinner {
    last_update: Instant,
    current: usize,
    is_spinning: bool,
}

impl Spinner {
    pub fn update(&mut self, spin: bool) -> Duration {
        self.is_spinning = spin;
        if spin {
            let frame_time = Duration::from_millis(80);
            let now = Instant::now();
            let since = now.duration_since(self.last_update);
            if since >= frame_time {
                self.last_update = now;
                self.current += 1;
                self.current %= FRAMES.len();
                return frame_time;
            } else {
                return frame_time - since;
            }
        }
        Duration::MAX
    }

    pub fn current(&self) -> Option<char> {
        if self.is_spinning {
            Some(FRAMES[self.current])
        } else {
            None
        }
    }
}

impl Default for Spinner {
    fn default() -> Self {
        Self {
            last_update: Instant::now(),
            current: 0,
            is_spinning: false,
        }
    }
}
