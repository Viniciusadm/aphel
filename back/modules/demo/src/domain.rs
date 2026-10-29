use std::collections::BTreeMap;

use chrono::{DateTime, Utc};

use aphel_kernel::domain::access::Permission;
use aphel_kernel::domain::{Email, Secret, UserId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DemoSeedCommand {
    pub(crate) force: bool,
    pub(crate) visitor: Option<DemoVisitor>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DemoVisitor {
    pub(crate) email: String,
    pub(crate) password: Secret,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DemoSeedReport {
    pub(crate) visitor_email: Option<String>,
    pub(crate) counts: BTreeMap<String, u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DemoResetReport {
    pub(crate) removed: BTreeMap<String, u64>,
    pub(crate) removed_files: u64,
    pub(crate) seeded: BTreeMap<String, u64>,
    pub(crate) visitor_email: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DemoRemovalReport {
    pub(crate) counts: BTreeMap<String, u64>,
    pub(crate) removed_files: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DemoStage {
    pub(crate) id: String,
    pub(crate) kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct DemoCatalog {
    pub(crate) stages: Vec<DemoStage>,
    pub(crate) lost_reason_ids: Vec<String>,
    pub(crate) activity_types: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NewDemoVisitor {
    pub(crate) user_id: UserId,
    pub(crate) email: Email,
    pub(crate) display_name: String,
    pub(crate) password_hash: String,
    pub(crate) role_key: String,
    pub(crate) role_name: String,
    pub(crate) permissions: Vec<Permission>,
    pub(crate) verified_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct DemoCleanup {
    pub(crate) counts: BTreeMap<String, u64>,
    pub(crate) storage_keys: Vec<String>,
}
