//! Update timer helper for modules with periodic updates.
//!
//! This provides a reusable pattern for modules that need to update
//! at regular intervals without duplicating the timing logic.

use std::sync::Mutex;
use std::time::{Duration, Instant};

/// A timer that tracks when updates should occur.
///
/// Thread-safe and can be shared across module instances.
pub struct UpdateTimer {
    interval: Duration,
    last_update: Mutex<Option<Instant>>,
}

impl UpdateTimer {
    /// Create a new timer with the given interval in seconds.
    pub fn new(interval_secs: u64) -> Self {
        Self {
            interval: Duration::from_secs(interval_secs),
            last_update: Mutex::new(None),
        }
    }

    /// Create a new timer with a Duration.
    pub fn with_duration(interval: Duration) -> Self {
        Self {
            interval,
            last_update: Mutex::new(None),
        }
    }

    /// Check if enough time has passed since the last update.
    /// If true, the timer is automatically reset.
    ///
    /// Returns true on the first call (no previous update recorded).
    pub fn should_update(&self) -> bool {
        let mut last = self.last_update.lock().unwrap();
        let should = match *last {
            None => true,
            Some(time) => time.elapsed() >= self.interval,
        };
        if should {
            *last = Some(Instant::now());
        }
        should
    }

    /// Check if enough time has passed without resetting the timer.
    /// Use this when you need to check but may not actually perform the update.
    pub fn is_due(&self) -> bool {
        let last = self.last_update.lock().unwrap();
        match *last {
            None => true,
            Some(time) => time.elapsed() >= self.interval,
        }
    }

    /// Manually reset the timer to now.
    pub fn reset(&self) {
        *self.last_update.lock().unwrap() = Some(Instant::now());
    }

    /// Get the interval duration.
    pub fn interval(&self) -> Duration {
        self.interval
    }

    /// Get time until next update (returns zero if already due).
    pub fn time_until_next(&self) -> Duration {
        let last = self.last_update.lock().unwrap();
        match *last {
            None => Duration::ZERO,
            Some(time) => {
                let elapsed = time.elapsed();
                if elapsed >= self.interval {
                    Duration::ZERO
                } else {
                    self.interval - elapsed
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn test_first_call_returns_true() {
        let timer = UpdateTimer::new(60);
        assert!(timer.should_update());
    }

    #[test]
    fn test_immediate_second_call_returns_false() {
        let timer = UpdateTimer::new(60);
        timer.should_update();
        assert!(!timer.should_update());
    }

    #[test]
    fn test_after_interval_returns_true() {
        let timer = UpdateTimer::with_duration(Duration::from_millis(50));
        timer.should_update();
        sleep(Duration::from_millis(60));
        assert!(timer.should_update());
    }

    #[test]
    fn test_is_due_does_not_reset() {
        let timer = UpdateTimer::new(60);
        assert!(timer.is_due());
        assert!(timer.is_due()); // Still true because is_due doesn't reset
    }
}
