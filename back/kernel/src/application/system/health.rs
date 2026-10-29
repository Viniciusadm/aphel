use std::time::Duration;

use chrono::{DateTime, Utc};

use crate::application::security::Clock;
use crate::application::system::WORKER_ROLE;
use crate::application::system::ports::HealthProbe;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckStatus {
    Ok,
    Fail,
    Skipped,
}

impl CheckStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Fail => "fail",
            Self::Skipped => "skipped",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealthCheck {
    pub name: &'static str,
    pub status: CheckStatus,
    pub reason: Option<&'static str>,
    pub age_seconds: Option<i64>,
    pub threshold_seconds: Option<u64>,
}

impl HealthCheck {
    fn new(name: &'static str, status: CheckStatus) -> Self {
        Self {
            name,
            status,
            reason: None,
            age_seconds: None,
            threshold_seconds: None,
        }
    }

    fn ok(name: &'static str) -> Self {
        Self::new(name, CheckStatus::Ok)
    }

    fn fail(name: &'static str, reason: &'static str) -> Self {
        Self::new(name, CheckStatus::Fail).with_reason(reason)
    }

    fn skipped(name: &'static str, reason: &'static str) -> Self {
        Self::new(name, CheckStatus::Skipped).with_reason(reason)
    }

    fn with_reason(mut self, reason: &'static str) -> Self {
        self.reason = Some(reason);
        self
    }

    fn with_age(mut self, age_seconds: Option<i64>, threshold: Duration) -> Self {
        self.age_seconds = age_seconds;
        self.threshold_seconds = Some(threshold.as_secs());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Readiness {
    pub checks: Vec<HealthCheck>,
    pub checked_at: DateTime<Utc>,
}

impl Readiness {
    pub fn is_ready(&self) -> bool {
        self.checks
            .iter()
            .all(|check| check.status != CheckStatus::Fail)
    }

    pub fn check(&self, name: &str) -> Option<&HealthCheck> {
        self.checks.iter().find(|check| check.name == name)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadinessThresholds {
    pub worker_max_silence: Duration,
    pub scheduler_max_delay: Duration,
    pub queue_max_delay: Duration,
    pub require_worker: bool,
}

impl Default for ReadinessThresholds {
    fn default() -> Self {
        Self {
            worker_max_silence: Duration::from_secs(90),
            scheduler_max_delay: Duration::from_secs(120),
            queue_max_delay: Duration::from_secs(300),
            require_worker: true,
        }
    }
}

pub const CHECK_DATABASE: &str = "database";
pub const CHECK_STORAGE: &str = "storage";
pub const CHECK_WORKER: &str = "worker";
pub const CHECK_SCHEDULER: &str = "scheduler";
pub const CHECK_QUEUE: &str = "queue";

pub struct ReadinessUseCases<P, C> {
    probe: P,
    clock: C,
    thresholds: ReadinessThresholds,
}

impl<P, C> ReadinessUseCases<P, C>
where
    P: HealthProbe,
    C: Clock,
{
    pub fn new(probe: P, clock: C, thresholds: ReadinessThresholds) -> Self {
        Self {
            probe,
            clock,
            thresholds,
        }
    }

    pub async fn check(&self) -> Readiness {
        let now = self.clock.now();
        let mut checks = Vec::with_capacity(5);
        let database_ok = self.probe.database().await.is_ok();
        checks.push(if database_ok {
            HealthCheck::ok(CHECK_DATABASE)
        } else {
            HealthCheck::fail(CHECK_DATABASE, "unreachable")
        });
        checks.push(match self.probe.storage().await {
            Ok(()) => HealthCheck::ok(CHECK_STORAGE),
            Err(_) => HealthCheck::fail(CHECK_STORAGE, "unavailable"),
        });
        if !database_ok {
            for name in [CHECK_WORKER, CHECK_SCHEDULER, CHECK_QUEUE] {
                checks.push(HealthCheck::fail(name, "database_unreachable"));
            }
            return Readiness {
                checks,
                checked_at: now,
            };
        }
        checks.push(self.worker_check(now).await);
        checks.push(self.scheduler_check(now).await);
        checks.push(self.queue_check(now).await);
        Readiness {
            checks,
            checked_at: now,
        }
    }

    async fn worker_check(&self, now: DateTime<Utc>) -> HealthCheck {
        let threshold = self.thresholds.worker_max_silence;
        if !self.thresholds.require_worker {
            return HealthCheck::skipped(CHECK_WORKER, "disabled");
        }
        match self.probe.latest_heartbeat(WORKER_ROLE).await {
            Err(_) => HealthCheck::fail(CHECK_WORKER, "query_failed"),
            Ok(None) => HealthCheck::fail(CHECK_WORKER, "no_heartbeat").with_age(None, threshold),
            Ok(Some(seen_at)) => {
                let age = age_seconds(now, seen_at);
                staleness(CHECK_WORKER, age, threshold, "heartbeat_stale")
            }
        }
    }

    async fn scheduler_check(&self, now: DateTime<Utc>) -> HealthCheck {
        let threshold = self.thresholds.scheduler_max_delay;
        if !self.thresholds.require_worker {
            return HealthCheck::skipped(CHECK_SCHEDULER, "disabled");
        }
        match self.probe.oldest_due_schedule().await {
            Err(_) => HealthCheck::fail(CHECK_SCHEDULER, "query_failed"),
            Ok(None) => {
                HealthCheck::fail(CHECK_SCHEDULER, "never_started").with_age(None, threshold)
            }
            Ok(Some(next_run_at)) if next_run_at >= now => {
                HealthCheck::ok(CHECK_SCHEDULER).with_age(Some(0), threshold)
            }
            Ok(Some(next_run_at)) => {
                let age = age_seconds(now, next_run_at);
                staleness(CHECK_SCHEDULER, age, threshold, "schedules_overdue")
            }
        }
    }

    async fn queue_check(&self, now: DateTime<Utc>) -> HealthCheck {
        let threshold = self.thresholds.queue_max_delay;
        match self.probe.oldest_ready_job(now).await {
            Err(_) => HealthCheck::fail(CHECK_QUEUE, "query_failed"),
            Ok(None) => HealthCheck::ok(CHECK_QUEUE).with_age(Some(0), threshold),
            Ok(Some(run_at)) => {
                let age = age_seconds(now, run_at);
                staleness(CHECK_QUEUE, age, threshold, "backlog_delayed")
            }
        }
    }
}

fn staleness(
    name: &'static str,
    age: i64,
    threshold: Duration,
    reason: &'static str,
) -> HealthCheck {
    let check = if age > threshold.as_secs() as i64 {
        HealthCheck::fail(name, reason)
    } else {
        HealthCheck::ok(name)
    };
    check.with_age(Some(age), threshold)
}

pub fn age_seconds(now: DateTime<Utc>, then: DateTime<Utc>) -> i64 {
    (now - then).num_seconds().max(0)
}
