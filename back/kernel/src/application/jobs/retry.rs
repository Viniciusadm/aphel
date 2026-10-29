use std::time::Duration;

pub const DEFAULT_RETRY_BASE: Duration = Duration::from_secs(10);
pub const DEFAULT_RETRY_MAX: Duration = Duration::from_secs(3600);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicy {
    pub base: Duration,
    pub max: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            base: DEFAULT_RETRY_BASE,
            max: DEFAULT_RETRY_MAX,
        }
    }
}

impl RetryPolicy {
    pub fn new(base: Duration, max: Duration) -> Self {
        Self {
            base,
            max: max.max(base),
        }
    }

    pub fn delay_after(&self, attempt: u32) -> Duration {
        let exponent = attempt.saturating_sub(1).min(31);
        self.base
            .checked_mul(1u32 << exponent)
            .unwrap_or(self.max)
            .min(self.max)
    }
}
