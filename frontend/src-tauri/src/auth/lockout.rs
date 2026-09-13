use std::time::{Duration, Instant};

const MAX_FAILED_ATTEMPTS: u32 = 10;
const BASE_LOCKOUT_DURATION: Duration = Duration::from_secs(5);
const MAX_LOCKOUT_DURATION: Duration = Duration::from_secs(300);

pub struct LockoutTracker {
    failed_attempts: u32,
    lockout_until: Option<Instant>,
    now: Box<dyn Fn() -> Instant + Send + Sync>,
}

impl LockoutTracker {
    pub fn new() -> Self {
        Self {
            failed_attempts: 0,
            lockout_until: None,
            now: Box::new(Instant::now),
        }
    }

    #[cfg(test)]
    pub fn with_clock(now: Box<dyn Fn() -> Instant + Send + Sync>) -> Self {
        Self {
            failed_attempts: 0,
            lockout_until: None,
            now,
        }
    }

    pub fn is_locked_out(&self) -> bool {
        if let Some(lockout) = self.lockout_until {
            (self.now)() < lockout
        } else {
            false
        }
    }

    pub fn record_failure(&mut self) -> Result<(), String> {
        self.failed_attempts += 1;
        let now = (self.now)();

        if self.failed_attempts >= MAX_FAILED_ATTEMPTS {
            self.lockout_until = Some(now + MAX_LOCKOUT_DURATION);
            return Err(format!(
                "Too many failed attempts. Account locked for {} minutes.",
                MAX_LOCKOUT_DURATION.as_secs() / 60
            ));
        }

        let lockout_duration =
            BASE_LOCKOUT_DURATION.saturating_mul(2_u32.pow(self.failed_attempts.saturating_sub(1)));
        let lockout_duration = std::cmp::min(lockout_duration, MAX_LOCKOUT_DURATION);
        self.lockout_until = Some(now + lockout_duration);

        Err(format!(
            "Too many failed attempts. Please try again in {} seconds.",
            lockout_duration.as_secs()
        ))
    }

    pub fn reset(&mut self) {
        self.failed_attempts = 0;
        self.lockout_until = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_is_not_locked_out() {
        let state = LockoutTracker::new();
        assert!(!state.is_locked_out());
    }

    #[test]
    fn test_first_failure_returns_error_with_wait() {
        let mut state = LockoutTracker::new();
        let result = state.record_failure();
        assert!(result.is_err());
        assert!(state.is_locked_out());
    }

    #[test]
    fn test_reset_clears_lockout() {
        let mut state = LockoutTracker::new();
        state.record_failure().ok();
        state.reset();
        assert!(!state.is_locked_out());
        assert_eq!(state.failed_attempts, 0);
    }
}
