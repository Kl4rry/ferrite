use std::time::{Duration, Instant};

pub struct Timer {
    last_run: Instant,
}

impl Default for Timer {
    fn default() -> Self {
        // NOTE: we default to an hour in the past so that the timer is true for normal things the first frame
        Self {
            last_run: Instant::now()
                .checked_sub(Duration::from_secs(3600))
                .unwrap(),
        }
    }
}

impl Timer {
    pub fn every(&mut self, duration: Duration) -> bool {
        let now = Instant::now();
        if now.duration_since(self.last_run) > duration {
            self.last_run = now;
            true
        } else {
            false
        }
    }
}
