use serde::Serialize;
use utoipa::ToSchema;

use crate::api::dto::system_jobs::timestamp;
use crate::application::system::{
    CoreMigrationStatus, FailureEntry, HealthCheck, InstalledModuleInfo, InstanceHeartbeat,
    MigrationEntry, MigrationStatus, ModuleMigrationStatus, Readiness, RecentFailures,
};

#[derive(Debug, Serialize, ToSchema)]
pub struct HealthCheckResponse {
    #[schema(example = "worker")]
    pub name: String,
    #[schema(example = "ok")]
    pub status: String,
    #[schema(example = "heartbeat_stale")]
    pub reason: Option<String>,
    pub age_seconds: Option<i64>,
    pub threshold_seconds: Option<u64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ReadinessResponse {
    #[schema(example = "ok")]
    pub status: String,
    pub checks: Vec<HealthCheckResponse>,
    #[schema(format = DateTime)]
    pub checked_at: String,
}

impl From<HealthCheck> for HealthCheckResponse {
    fn from(check: HealthCheck) -> Self {
        Self {
            name: check.name.to_owned(),
            status: check.status.as_str().to_owned(),
            reason: check.reason.map(str::to_owned),
            age_seconds: check.age_seconds,
            threshold_seconds: check.threshold_seconds,
        }
    }
}

impl From<Readiness> for ReadinessResponse {
    fn from(readiness: Readiness) -> Self {
        Self {
            status: if readiness.is_ready() { "ok" } else { "fail" }.to_owned(),
            checked_at: timestamp(readiness.checked_at),
            checks: readiness.checks.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct HeartbeatResponse {
    pub instance_id: String,
    #[schema(example = "worker")]
    pub role: String,
    pub hostname: Option<String>,
    pub version: String,
    #[schema(format = DateTime)]
    pub started_at: String,
    #[schema(format = DateTime)]
    pub last_seen_at: String,
}

impl From<InstanceHeartbeat> for HeartbeatResponse {
    fn from(heartbeat: InstanceHeartbeat) -> Self {
        Self {
            instance_id: heartbeat.instance_id,
            role: heartbeat.role,
            hostname: heartbeat.hostname,
            version: heartbeat.version,
            started_at: timestamp(heartbeat.started_at),
            last_seen_at: timestamp(heartbeat.last_seen_at),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct InstalledModuleResponse {
    #[schema(example = "proposals")]
    pub key: String,
    pub name: String,
    pub version: String,
    #[schema(example = "compatible")]
    pub compatibility: String,
    pub compatibility_detail: Option<String>,
    pub enabled: bool,
}

impl From<InstalledModuleInfo> for InstalledModuleResponse {
    fn from(info: InstalledModuleInfo) -> Self {
        Self {
            key: info.key,
            name: info.name,
            version: info.version,
            compatibility: info.compatibility,
            compatibility_detail: info.compatibility_detail,
            enabled: info.enabled,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SystemOverviewResponse {
    #[schema(example = "0.1.0")]
    pub version: String,
    #[schema(example = "linux")]
    pub os: String,
    #[schema(example = "x86_64")]
    pub arch: String,
    pub readiness: ReadinessResponse,
    pub heartbeats: Vec<HeartbeatResponse>,
    pub modules: Vec<InstalledModuleResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MigrationEntryResponse {
    pub version: i64,
    pub description: String,
}

impl From<MigrationEntry> for MigrationEntryResponse {
    fn from(entry: MigrationEntry) -> Self {
        Self {
            version: entry.version,
            description: entry.description,
        }
    }
}

fn entries(list: Vec<MigrationEntry>) -> Vec<MigrationEntryResponse> {
    list.into_iter().map(Into::into).collect()
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CoreMigrationsResponse {
    pub known: u64,
    pub applied: u64,
    pub latest_applied: Option<i64>,
    pub pending: Vec<MigrationEntryResponse>,
    pub failed: Vec<MigrationEntryResponse>,
    pub unknown_applied: Vec<MigrationEntryResponse>,
}

impl From<CoreMigrationStatus> for CoreMigrationsResponse {
    fn from(status: CoreMigrationStatus) -> Self {
        Self {
            known: status.known,
            applied: status.applied,
            latest_applied: status.latest_applied,
            pending: entries(status.pending),
            failed: entries(status.failed),
            unknown_applied: entries(status.unknown_applied),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ModuleMigrationsResponse {
    pub key: String,
    pub name: String,
    pub version: String,
    pub min_aphel_version: String,
    pub compatibility: String,
    pub compatibility_detail: Option<String>,
    pub enabled: bool,
    pub applied: u64,
    pub pending: Vec<MigrationEntryResponse>,
    pub changed: Vec<MigrationEntryResponse>,
}

impl From<ModuleMigrationStatus> for ModuleMigrationsResponse {
    fn from(status: ModuleMigrationStatus) -> Self {
        Self {
            key: status.key,
            name: status.name,
            version: status.version,
            min_aphel_version: status.min_aphel_version,
            compatibility: status.compatibility,
            compatibility_detail: status.compatibility_detail,
            enabled: status.enabled,
            applied: status.applied,
            pending: entries(status.pending),
            changed: entries(status.changed),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MigrationsResponse {
    pub pending_total: usize,
    pub problems: Vec<String>,
    pub core: CoreMigrationsResponse,
    pub modules: Vec<ModuleMigrationsResponse>,
}

impl From<MigrationStatus> for MigrationsResponse {
    fn from(status: MigrationStatus) -> Self {
        Self {
            pending_total: status.pending_count(),
            problems: status.blocking_problems(),
            core: status.core.into(),
            modules: status.modules.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FailureResponse {
    #[schema(example = "webhook")]
    pub source: String,
    pub reference_id: String,
    #[schema(example = "deal.updated")]
    pub kind: String,
    pub error: Option<String>,
    pub response_status: Option<u16>,
    #[schema(format = DateTime)]
    pub occurred_at: String,
}

impl From<FailureEntry> for FailureResponse {
    fn from(entry: FailureEntry) -> Self {
        Self {
            source: entry.source.as_str().to_owned(),
            reference_id: entry.reference_id,
            kind: entry.kind,
            error: entry.error,
            response_status: entry.response_status,
            occurred_at: timestamp(entry.occurred_at),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RecentFailuresResponse {
    pub window_days: i64,
    pub automations: Vec<FailureResponse>,
    pub webhooks: Vec<FailureResponse>,
    pub integrations: Vec<FailureResponse>,
}

impl RecentFailuresResponse {
    pub fn new(failures: RecentFailures, window_days: i64) -> Self {
        let convert = |list: Vec<FailureEntry>| list.into_iter().map(Into::into).collect();
        Self {
            window_days,
            automations: convert(failures.automations),
            webhooks: convert(failures.webhooks),
            integrations: convert(failures.integrations),
        }
    }
}
