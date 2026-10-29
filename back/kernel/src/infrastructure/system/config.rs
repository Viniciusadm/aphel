use std::env;
use std::time::Duration;

use crate::application::system::ReadinessThresholds;

pub(crate) const DEFAULT_HEARTBEAT_INTERVAL_SECS: u64 = 15;
pub(crate) const DEFAULT_MIN_FREE_DISK_MB: u64 = 500;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemSettings {
    pub readiness: ReadinessThresholds,
    pub(crate) heartbeat_interval: Duration,
    pub metrics_token: Option<String>,
    pub min_free_disk_mb: u64,
}

impl SystemSettings {
    pub fn from_env() -> Self {
        let defaults = ReadinessThresholds::default();
        Self {
            readiness: ReadinessThresholds {
                worker_max_silence: seconds("HEALTH_WORKER_MAX_SILENCE_SECS")
                    .unwrap_or(defaults.worker_max_silence),
                scheduler_max_delay: seconds("HEALTH_SCHEDULER_MAX_DELAY_SECS")
                    .unwrap_or(defaults.scheduler_max_delay),
                queue_max_delay: seconds("HEALTH_QUEUE_MAX_DELAY_SECS")
                    .unwrap_or(defaults.queue_max_delay),
                require_worker: env::var("HEALTH_REQUIRE_WORKER")
                    .map(|value| !matches!(value.trim(), "0" | "false" | "FALSE" | "no" | "NO"))
                    .unwrap_or(defaults.require_worker),
            },
            heartbeat_interval: seconds("WORKER_HEARTBEAT_INTERVAL_SECS")
                .unwrap_or(Duration::from_secs(DEFAULT_HEARTBEAT_INTERVAL_SECS)),
            metrics_token: env::var("METRICS_TOKEN")
                .ok()
                .map(|value| value.trim().to_owned())
                .filter(|value| !value.is_empty()),
            min_free_disk_mb: env::var("UPGRADE_MIN_FREE_DISK_MB")
                .ok()
                .and_then(|value| value.trim().parse::<u64>().ok())
                .unwrap_or(DEFAULT_MIN_FREE_DISK_MB),
        }
    }
}

fn seconds(name: &str) -> Option<Duration> {
    env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|value| *value > 0)
        .map(Duration::from_secs)
}
