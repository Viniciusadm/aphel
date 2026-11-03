use crate::{AppError, access::AccessScope, fields::RecordTypeRegistry, shared::RecordRef};
use aphel_kernel::application::fields::{
    FieldDefinitionRepository,
    permissions::{FieldAccess, FieldAccessPolicy},
};
use aphel_kernel::application::files::RecordAccessChecker;
use sqlx::MySqlPool;
use std::sync::Arc;

pub struct VisibleRecordLabel {
    pub title: Option<String>,
}

pub async fn visible_record_label<D: FieldDefinitionRepository>(
    pool: &MySqlPool,
    registry: &Arc<RecordTypeRegistry>,
    definitions: &D,
    scope: &AccessScope,
    record: &RecordRef,
) -> Result<Option<VisibleRecordLabel>, AppError> {
    let app = aphel_kernel::modules::app();
    if app
        .record_contributions()
        .iter()
        .find(|entry| entry.registers.as_ref() == Some(&record.record_type))
        .is_some_and(|entry| !app.is_enabled(entry.module))
    {
        return Ok(None);
    }
    let Some(definition) = registry.get(&record.record_type) else {
        return Ok(None);
    };
    let checker = aphel_kernel::infrastructure::records::MySqlRecordAccessChecker::new(
        pool.clone(),
        registry.clone(),
    );
    if !checker.can_read(scope, record).await? {
        return Ok(None);
    }
    let policy = FieldAccessPolicy::load(definitions, scope, &record.record_type).await?;
    let snapshot = aphel_kernel::records::inspect_entity(pool, registry, record).await?;
    let Some(snapshot) = snapshot else {
        return Ok(None);
    };
    let title = definition
        .title_field
        .as_deref()
        .filter(|key| policy.key_access(key) != FieldAccess::Hidden)
        .and_then(|key| snapshot.get("values").and_then(|values| values.get(key)))
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned);
    Ok(Some(VisibleRecordLabel { title }))
}
