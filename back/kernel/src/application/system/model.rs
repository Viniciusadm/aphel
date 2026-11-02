use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct JobKindCounts {
    pub queue: String,
    pub kind: String,
    pub ready: u64,
    pub scheduled: u64,
    pub running: u64,
    pub failed: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct JobStateTotals {
    pub ready: u64,
    pub scheduled: u64,
    pub running: u64,
    pub failed: u64,
}

impl JobStateTotals {
    pub fn sum(counts: &[JobKindCounts]) -> Self {
        counts.iter().fold(Self::default(), |total, entry| Self {
            ready: total.ready + entry.ready,
            scheduled: total.scheduled + entry.scheduled,
            running: total.running + entry.running,
            failed: total.failed + entry.failed,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueOverview {
    pub totals: JobStateTotals,
    pub by_kind: Vec<JobKindCounts>,
    pub oldest_ready_at: Option<DateTime<Utc>>,
    pub oldest_ready_age_seconds: Option<i64>,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailedJob {
    pub id: Uuid,
    pub queue: String,
    pub kind: String,
    pub attempts: u32,
    pub max_attempts: u32,
    pub last_error: Option<String>,
    pub failed_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FailedJobFilter {
    pub kind: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageWindow {
    pub offset: u64,
    pub limit: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureSource {
    Automation,
    Webhook,
    Integration,
}

impl FailureSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Automation => "automation",
            Self::Webhook => "webhook",
            Self::Integration => "integration",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailureEntry {
    pub source: FailureSource,
    pub reference_id: String,
    pub kind: String,
    pub error: Option<String>,
    pub response_status: Option<u16>,
    pub occurred_at: DateTime<Utc>,
}

const MAX_FAILURE_ERROR_CHARS: usize = 500;

impl FailureEntry {
    pub fn new(
        source: FailureSource,
        reference_id: String,
        kind: String,
        error: Option<String>,
        response_status: Option<u16>,
        occurred_at: DateTime<Utc>,
    ) -> Self {
        let error = error.map(|text| {
            if text.chars().count() > MAX_FAILURE_ERROR_CHARS {
                let mut short: String = text.chars().take(MAX_FAILURE_ERROR_CHARS).collect();
                short.push('…');
                short
            } else {
                text
            }
        });
        Self {
            source,
            reference_id,
            kind,
            error,
            response_status,
            occurred_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RecentFailures {
    pub automations: Vec<FailureEntry>,
    pub webhooks: Vec<FailureEntry>,
    pub integrations: Vec<FailureEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceHeartbeat {
    pub instance_id: String,
    pub role: String,
    pub hostname: Option<String>,
    pub version: String,
    pub started_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationEntry {
    pub version: i64,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CoreMigrationStatus {
    pub known: u64,
    pub applied: u64,
    pub latest_applied: Option<i64>,
    pub pending: Vec<MigrationEntry>,
    pub failed: Vec<MigrationEntry>,
    pub unknown_applied: Vec<MigrationEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleMigrationStatus {
    pub key: String,
    pub name: String,
    pub version: String,
    pub min_aphel_version: String,
    pub compatibility: String,
    pub compatibility_detail: Option<String>,
    pub enabled: bool,
    pub applied: u64,
    pub pending: Vec<MigrationEntry>,
    pub changed: Vec<MigrationEntry>,
}

impl ModuleMigrationStatus {
    pub fn is_compatible(&self) -> bool {
        self.compatibility == "compatible"
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MigrationStatus {
    pub core: CoreMigrationStatus,
    pub modules: Vec<ModuleMigrationStatus>,
}

impl MigrationStatus {
    pub fn pending_count(&self) -> usize {
        self.core.pending.len()
            + self
                .modules
                .iter()
                .filter(|module| module.is_compatible())
                .map(|module| module.pending.len())
                .sum::<usize>()
    }

    pub fn blocking_problems(&self) -> Vec<String> {
        let mut problems = Vec::new();
        for entry in &self.core.failed {
            problems.push(format!(
                "core migration {} ({}) is marked as failed",
                entry.version, entry.description
            ));
        }
        for entry in &self.core.unknown_applied {
            problems.push(format!(
                "core migration {} ({}) is applied in the database but missing from this binary (database is newer than this version)",
                entry.version, entry.description
            ));
        }
        for module in &self.modules {
            for entry in &module.changed {
                problems.push(format!(
                    "module {} migration {} was changed after being applied",
                    module.key, entry.version
                ));
            }
        }
        problems
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledModuleInfo {
    pub key: String,
    pub name: String,
    pub version: String,
    pub compatibility: String,
    pub compatibility_detail: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SystemAuditEntry {
    pub id: Uuid,
    pub action: &'static str,
    pub actor_user_id: Option<String>,
    pub subject_type: Option<&'static str>,
    pub subject_id: Option<String>,
    pub metadata: Value,
    pub ip: Option<String>,
    pub occurred_at: DateTime<Utc>,
}

pub const SYSTEM_JOB_RETRIED: &str = "system.job_retried";
pub const SYSTEM_JOB_DISCARDED: &str = "system.job_discarded";
