use std::time::Duration;

pub struct PollScheduler {
    pub base_interval: Duration,
    pub max_backoff: Duration,
    pub current_interval: Duration,
    pub active_runs_count: usize,
}

impl PollScheduler {
    pub fn new(base_secs: u64, max_secs: u64) -> Self {
        let base_interval = Duration::from_secs(base_secs);
        let max_backoff = Duration::from_secs(max_secs);
        Self {
            base_interval,
            max_backoff,
            current_interval: base_interval,
            active_runs_count: 0,
        }
    }

    pub fn on_poll_completed(&mut self, active_runs: usize, changed: bool) {
        self.active_runs_count = active_runs;
        if active_runs > 0 || changed {
            self.current_interval = self.base_interval;
        } else {
            let next = self.current_interval * 2;
            self.current_interval = next.min(self.max_backoff);
        }
    }

    pub fn reset(&mut self) {
        self.current_interval = self.base_interval;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backoff_schedule() {
        let mut ps = PollScheduler::new(3, 60);
        assert_eq!(ps.current_interval, Duration::from_secs(3));

        ps.on_poll_completed(0, false);
        assert_eq!(ps.current_interval, Duration::from_secs(6));

        ps.on_poll_completed(0, false);
        assert_eq!(ps.current_interval, Duration::from_secs(12));

        ps.reset();
        assert_eq!(ps.current_interval, Duration::from_secs(3));
    }
}
