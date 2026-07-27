//! Reconnect backoff: 0, 1, 2, 5, 10, then 30s.

use std::time::Duration;

const STEPS_MS: &[u64] = &[0, 1_000, 2_000, 5_000, 10_000, 30_000];

#[derive(Debug, Clone)]
pub struct ReconnectBackoff {
    step: usize,
}

impl Default for ReconnectBackoff {
    fn default() -> Self {
        Self { step: 0 }
    }
}

impl ReconnectBackoff {
    pub fn reset(&mut self) {
        self.step = 0;
    }

    pub fn next_delay(&mut self) -> Duration {
        let idx = self.step.min(STEPS_MS.len() - 1);
        let d = Duration::from_millis(STEPS_MS[idx]);
        if self.step < STEPS_MS.len() - 1 {
            self.step += 1;
        }
        d
    }

    /// Shorten wait without starting a second loop (legacy Soft-Wake helper).
    pub fn shorten(&mut self) {
        self.step = 0;
    }

    pub fn step(&self) -> usize {
        self.step
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_sequence() {
        let mut b = ReconnectBackoff::default();
        assert_eq!(b.next_delay(), Duration::from_millis(0));
        assert_eq!(b.next_delay(), Duration::from_millis(1_000));
        assert_eq!(b.next_delay(), Duration::from_millis(2_000));
        assert_eq!(b.next_delay(), Duration::from_millis(5_000));
        assert_eq!(b.next_delay(), Duration::from_millis(10_000));
        assert_eq!(b.next_delay(), Duration::from_millis(30_000));
        assert_eq!(b.next_delay(), Duration::from_millis(30_000));
        b.reset();
        assert_eq!(b.next_delay(), Duration::from_millis(0));
    }

    #[test]
    fn shorten_resets_step() {
        let mut b = ReconnectBackoff::default();
        let _ = b.next_delay();
        let _ = b.next_delay();
        b.shorten();
        assert_eq!(b.step(), 0);
    }
}
