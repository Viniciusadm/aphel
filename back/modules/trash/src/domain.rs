use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

use aphel_kernel::application::fields::{EntityDefinition, RecordTypeRegistry};
use aphel_kernel::application::files::UserReference;
use aphel_kernel::application::shared::RecordType;

pub(crate) const DEFAULT_RETENTION_DAYS: u32 = 30;
pub(crate) const MIN_RETENTION_DAYS: u32 = 1;
pub(crate) const MAX_RETENTION_DAYS: u32 = 365;

pub fn is_trashable(definition: &EntityDefinition) -> bool {
    definition.deleted_column.is_some() && definition.title_column().is_some()
}

pub fn trashable_types(registry: &RecordTypeRegistry) -> Vec<EntityDefinition> {
    registry
        .definitions()
        .filter(|definition| is_trashable(definition))
        .cloned()
        .collect()
}

pub fn trashable_type<'a>(
    registry: &'a RecordTypeRegistry,
    record_type: &str,
) -> Option<&'a EntityDefinition> {
    let record_type = RecordType::parse(record_type.trim()).ok()?;
    registry
        .get(&record_type)
        .filter(|definition| is_trashable(definition))
}

pub(crate) fn is_valid_retention(days: u32) -> bool {
    (MIN_RETENTION_DAYS..=MAX_RETENTION_DAYS).contains(&days)
}

pub(crate) fn purge_threshold(now: DateTime<Utc>, retention_days: u32) -> DateTime<Utc> {
    now - Duration::days(i64::from(retention_days))
}

pub(crate) fn expires_at(deleted_at: DateTime<Utc>, retention_days: u32) -> DateTime<Utc> {
    deleted_at + Duration::days(i64::from(retention_days))
}

pub(crate) const TRASH_ITEM_NOT_FOUND: &str = "trash_item_not_found";
pub(crate) const TRASH_SETTINGS_UPDATED: &str = "trash.settings_updated";
pub(crate) const TRASH_PURGE_JOB: &str = "trash.purge";
pub(crate) const TRASH_PURGE_CRON: &str = "0 30 3 * * *";
pub(crate) const PURGE_BATCH_SIZE: u32 = 200;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TrashItem {
    pub(crate) record_type: RecordType,
    pub(crate) id: Uuid,
    pub(crate) title: String,
    pub(crate) subtitle: Option<String>,
    pub(crate) owner: Option<UserReference>,
    pub(crate) deleted_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TrashEntry {
    pub(crate) item: TrashItem,
    pub(crate) expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct TrashQuery {
    pub(crate) record_type: Option<String>,
    pub(crate) search: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TrashSettings {
    pub(crate) retention_days: u32,
}

pub fn restored_event(record_type: &str) -> String {
    format!("{record_type}.restored")
}

pub fn purged_event(record_type: &str) -> String {
    format!("{record_type}.purged")
}
